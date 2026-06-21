use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};

use crate::git;
use crate::scanner::{self, ScanConfig, ScanResult};

pub use crate::git::clone_arguments;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePreview {
    pub url: String,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireOutcome {
    Cloned,
    Reused,
}

#[derive(Debug, Clone)]
pub struct AcquiredSource {
    pub path: PathBuf,
    pub skills: Vec<ScanResult>,
    pub outcome: AcquireOutcome,
}

pub fn preview(url: &str, central_dir: &Path) -> Result<SourcePreview> {
    canonical_origin(url)?;
    let name = repository_name(url)?;
    Ok(SourcePreview {
        url: url.to_string(),
        destination: central_dir.join(name),
    })
}

pub fn repository_name(url: &str) -> Result<String> {
    let cleaned = strip_query_fragment(url).trim_end_matches('/');
    let path = git_path(cleaned)?;
    let name = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .strip_suffix(".git")
        .unwrap_or_else(|| {
            path.trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or_default()
        });

    if name.is_empty()
        || matches!(name, "." | "..")
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        bail!("Git URL does not contain a safe repository name");
    }

    Ok(name.to_string())
}

pub fn canonical_origin(url: &str) -> Result<String> {
    let cleaned = strip_git_suffix(strip_query_fragment(url).trim_end_matches('/'));

    if let Some((scheme, remainder)) = cleaned.split_once("://") {
        let scheme = scheme.to_ascii_lowercase();
        if scheme == "file" {
            let path = Path::new(remainder.trim_start_matches('/'));
            let absolute = Path::new("/").join(path);
            return Ok(format!(
                "file:{}",
                canonical_local_path(&absolute).display()
            ));
        }

        let (authority, path) = remainder
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("Git URL does not contain a repository path"))?;
        if matches!(scheme.as_str(), "http" | "https") && authority.contains('@') {
            bail!("embedded HTTP credentials are not allowed");
        }
        let host = authority.rsplit('@').next().unwrap_or(authority);
        if host.is_empty() || path.is_empty() {
            bail!("Git URL does not contain a host and repository path");
        }
        return Ok(format!(
            "{}/{}",
            host.to_ascii_lowercase(),
            path.trim_start_matches('/')
        ));
    }

    if let Some((authority, path)) = scp_parts(cleaned) {
        let host = authority.rsplit('@').next().unwrap_or(authority);
        if host.is_empty() || path.is_empty() {
            bail!("Git URL does not contain a host and repository path");
        }
        return Ok(format!(
            "{}/{}",
            host.to_ascii_lowercase(),
            path.trim_start_matches('/')
        ));
    }

    bail!("Git source must be a URL")
}

pub fn acquire(source_preview: &SourcePreview, max_scan_depth: usize) -> Result<AcquiredSource> {
    match fs::symlink_metadata(&source_preview.destination) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!(
                    "managed source destination is a symlink: {}",
                    source_preview.destination.display()
                );
            }
            if !metadata.is_dir() {
                bail!(
                    "managed source destination is not a directory: {}",
                    source_preview.destination.display()
                );
            }
            reuse_existing(source_preview, max_scan_depth)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            clone_and_promote(source_preview, max_scan_depth)
        }
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to inspect managed source destination: {}",
                source_preview.destination.display()
            )
        }),
    }
}

fn reuse_existing(source_preview: &SourcePreview, max_scan_depth: usize) -> Result<AcquiredSource> {
    let existing_origin = git::origin_url(&source_preview.destination)
        .with_context(|| {
            format!(
                "failed to inspect origin for {}",
                source_preview.destination.display()
            )
        })?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "managed source destination has no readable origin: {}",
                source_preview.destination.display()
            )
        })?;

    if canonical_origin(&existing_origin)? != canonical_origin(&source_preview.url)? {
        bail!(
            "managed source destination has a different origin: {}",
            source_preview.destination.display()
        );
    }

    let skills = scan_source(&source_preview.destination, max_scan_depth)?;
    if skills.is_empty() {
        bail!(
            "managed source contains no skills: {}",
            source_preview.destination.display()
        );
    }

    Ok(AcquiredSource {
        path: source_preview.destination.clone(),
        skills,
        outcome: AcquireOutcome::Reused,
    })
}

fn clone_and_promote(
    source_preview: &SourcePreview,
    max_scan_depth: usize,
) -> Result<AcquiredSource> {
    let central_dir = source_preview
        .destination
        .parent()
        .context("managed source destination has no parent directory")?;
    fs::create_dir_all(central_dir).with_context(|| {
        format!(
            "failed to create managed source directory: {}",
            central_dir.display()
        )
    })?;
    let temporary = unique_temporary_path(central_dir);

    let operation = (|| {
        git::clone_repository(&source_preview.url, &temporary)?;
        let skills = scan_source(&temporary, max_scan_depth)?;
        if skills.is_empty() {
            bail!("cloned repository contains no skills");
        }
        fs::rename(&temporary, &source_preview.destination).with_context(|| {
            format!(
                "failed to promote cloned source to {}",
                source_preview.destination.display()
            )
        })?;
        let skills = scan_source(&source_preview.destination, max_scan_depth)?;
        Ok(AcquiredSource {
            path: source_preview.destination.clone(),
            skills,
            outcome: AcquireOutcome::Cloned,
        })
    })();

    if operation.is_err() && temporary.exists() {
        fs::remove_dir_all(&temporary).with_context(|| {
            format!(
                "failed to clean up temporary clone: {}",
                temporary.display()
            )
        })?;
    }

    operation
}

fn scan_source(path: &Path, max_scan_depth: usize) -> Result<Vec<ScanResult>> {
    scanner::scan(&ScanConfig {
        central_dir: path.to_path_buf(),
        scan_parent_dirs: vec![],
        max_scan_depth,
    })
}

fn unique_temporary_path(central_dir: &Path) -> PathBuf {
    loop {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate =
            central_dir.join(format!(".skills-manager-{}-{sequence}", std::process::id()));
        if !candidate.exists() {
            return candidate;
        }
    }
}

fn git_path(url: &str) -> Result<&str> {
    if let Some((scheme, remainder)) = url.split_once("://") {
        if scheme.eq_ignore_ascii_case("file") {
            return Ok(remainder);
        }
        let (_, path) = remainder
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("Git URL does not contain a repository path"))?;
        return Ok(path);
    }
    if let Some((_, path)) = scp_parts(url) {
        return Ok(path);
    }
    Ok(url)
}

fn scp_parts(url: &str) -> Option<(&str, &str)> {
    let (authority, path) = url.split_once(':')?;
    if authority.contains('/') || authority.is_empty() {
        return None;
    }
    Some((authority, path))
}

fn strip_query_fragment(url: &str) -> &str {
    let query = url.find('?').unwrap_or(url.len());
    let fragment = url.find('#').unwrap_or(url.len());
    &url[..query.min(fragment)]
}

fn strip_git_suffix(value: &str) -> &str {
    value.strip_suffix(".git").unwrap_or(value)
}

fn canonical_local_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use tempfile::tempdir;

    use super::{
        AcquireOutcome, acquire, canonical_origin, clone_arguments, preview, repository_name,
    };

    #[test]
    fn derives_repository_name_from_supported_git_urls() {
        for (url, expected) in [
            ("https://example.com/org/skills.git", "skills"),
            ("https://example.com/org/skills.git/", "skills"),
            ("https://example.com/org/skills.git?ref=main", "skills"),
            ("ssh://git@example.com/org/skills.git", "skills"),
            ("git@example.com:org/skills.git", "skills"),
        ] {
            assert_eq!(repository_name(url).unwrap(), expected, "URL: {url}");
        }
    }

    #[test]
    fn rejects_urls_without_a_safe_repository_name() {
        for url in [
            "https://example.com/.git",
            "https://example.com/org/..",
            "git@example.com:",
        ] {
            assert!(repository_name(url).is_err(), "URL: {url}");
        }
    }

    #[test]
    fn rejects_embedded_http_credentials() {
        let error = canonical_origin("https://user:secret@example.com/org/skills.git")
            .expect_err("embedded credentials must be rejected");

        assert!(error.to_string().contains("credentials"));
    }

    #[test]
    fn rejects_plain_local_paths() {
        let error = canonical_origin("/tmp/local-repository")
            .expect_err("plain local paths are not Git URLs");

        assert!(error.to_string().contains("URL"));
    }

    #[test]
    fn canonicalizes_equivalent_https_and_ssh_origins() {
        let expected = "example.com/org/Skills";

        for url in [
            "https://Example.com/org/Skills.git/",
            "ssh://git@example.com/org/Skills.git",
            "git@example.com:org/Skills.git",
        ] {
            assert_eq!(canonical_origin(url).unwrap(), expected, "URL: {url}");
        }
    }

    #[test]
    fn canonical_origin_strips_query_fragment_and_preserves_path_case() {
        assert_eq!(
            canonical_origin("https://example.com/org/Skills.git?ref=main#readme").unwrap(),
            "example.com/org/Skills"
        );
        assert_ne!(
            canonical_origin("https://example.com/org/Skills.git").unwrap(),
            canonical_origin("https://example.com/org/skills.git").unwrap()
        );
    }

    #[test]
    fn clone_arguments_disable_submodules() {
        let destination = Path::new("/tmp/managed-source");

        assert_eq!(
            clone_arguments("https://example.com/org/skills.git", destination),
            vec![
                "-c",
                "submodule.recurse=false",
                "clone",
                "--no-recurse-submodules",
                "https://example.com/org/skills.git",
                "/tmp/managed-source",
            ]
        );
    }

    #[test]
    fn acquire_promotes_a_clone_only_after_finding_skills() {
        let temp = tempdir().unwrap();
        let remote = create_git_repo(temp.path().join("remote-skills"), true);
        let central = temp.path().join("central");
        let source_preview = preview(&file_url(&remote), &central).unwrap();

        let acquired = acquire(&source_preview, 10).unwrap();

        assert_eq!(acquired.outcome, AcquireOutcome::Cloned);
        assert_eq!(acquired.path, central.join("remote-skills"));
        assert_eq!(acquired.skills.len(), 1);
        assert!(acquired.path.join("code-review").join("SKILL.md").is_file());
        assert_no_temporary_clone(&central);
    }

    #[test]
    fn acquire_returns_all_skills_from_a_multi_skill_repository() {
        let temp = tempdir().unwrap();
        let remote = create_git_repo(temp.path().join("multi-skills"), true);
        fs::create_dir_all(remote.join("docs")).unwrap();
        fs::write(remote.join("docs/SKILL.md"), "# Docs").unwrap();
        git(&["-C", remote.to_str().unwrap(), "add", "."]);
        git(&[
            "-C",
            remote.to_str().unwrap(),
            "-c",
            "user.name=Skills Manager Tests",
            "-c",
            "user.email=tests@example.com",
            "commit",
            "-m",
            "add docs skill",
        ]);
        let central = temp.path().join("central");
        let source_preview = preview(&file_url(&remote), &central).unwrap();

        let acquired = acquire(&source_preview, 10).unwrap();
        let mut skill_ids = acquired
            .skills
            .iter()
            .map(|skill| skill.skill_id.as_str())
            .collect::<Vec<_>>();
        skill_ids.sort_unstable();

        assert_eq!(
            skill_ids,
            vec!["multi-skills/code-review", "multi-skills/docs"]
        );
    }

    #[test]
    fn acquire_removes_a_clone_without_skills() {
        let temp = tempdir().unwrap();
        let remote = create_git_repo(temp.path().join("empty-source"), false);
        let central = temp.path().join("central");
        let source_preview = preview(&file_url(&remote), &central).unwrap();

        let error = acquire(&source_preview, 10).expect_err("skill-less clone must fail");

        assert!(error.to_string().contains("no skills"));
        assert!(!source_preview.destination.exists());
        assert_no_temporary_clone(&central);
    }

    #[test]
    fn acquire_reuses_an_existing_same_origin_without_updating_it() {
        let temp = tempdir().unwrap();
        let remote = create_git_repo(temp.path().join("reusable-source"), true);
        let central = temp.path().join("central");
        fs::create_dir_all(&central).unwrap();
        let destination = central.join("reusable-source");
        git(&[
            "clone",
            "--no-recurse-submodules",
            &file_url(&remote),
            destination.to_str().unwrap(),
        ]);
        fs::write(destination.join("local-change.txt"), "keep me").unwrap();
        let head_before = git_output(&["-C", destination.to_str().unwrap(), "rev-parse", "HEAD"]);
        let source_preview = preview(&file_url(&remote), &central).unwrap();

        let acquired = acquire(&source_preview, 10).unwrap();

        assert_eq!(acquired.outcome, AcquireOutcome::Reused);
        assert_eq!(
            git_output(&["-C", destination.to_str().unwrap(), "rev-parse", "HEAD"]),
            head_before
        );
        assert_eq!(
            fs::read_to_string(destination.join("local-change.txt")).unwrap(),
            "keep me"
        );
        assert_no_temporary_clone(&central);
    }

    #[test]
    fn acquire_rejects_a_different_origin_collision() {
        let temp = tempdir().unwrap();
        let requested = create_git_repo(temp.path().join("requested").join("skills"), true);
        let different = create_git_repo(temp.path().join("different").join("skills"), true);
        let central = temp.path().join("central");
        fs::create_dir_all(&central).unwrap();
        let destination = central.join("skills");
        git(&[
            "clone",
            "--no-recurse-submodules",
            &file_url(&different),
            destination.to_str().unwrap(),
        ]);
        let source_preview = preview(&file_url(&requested), &central).unwrap();

        let error = acquire(&source_preview, 10).expect_err("different origin must fail");

        assert!(error.to_string().contains("different origin"));
        assert_no_temporary_clone(&central);
    }

    #[test]
    fn acquire_rejects_non_git_file_and_directory_collisions() {
        let temp = tempdir().unwrap();
        let remote = create_git_repo(temp.path().join("remote").join("skills"), true);

        for collision in ["file", "directory"] {
            let central = temp.path().join(collision);
            fs::create_dir_all(&central).unwrap();
            let destination = central.join("skills");
            if collision == "file" {
                fs::write(&destination, "occupied").unwrap();
            } else {
                fs::create_dir_all(&destination).unwrap();
            }
            let source_preview = preview(&file_url(&remote), &central).unwrap();

            let error = acquire(&source_preview, 10).expect_err("collision must fail");

            assert!(
                error.to_string().contains("not a directory")
                    || error.to_string().contains("no readable origin")
            );
            assert_no_temporary_clone(&central);
        }
    }

    #[cfg(unix)]
    #[test]
    fn acquire_rejects_a_symlink_destination() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().unwrap();
        let remote = create_git_repo(temp.path().join("remote").join("skills"), true);
        let central = temp.path().join("central");
        let target = temp.path().join("target");
        fs::create_dir_all(&central).unwrap();
        fs::create_dir_all(&target).unwrap();
        symlink(&target, central.join("skills")).unwrap();
        let source_preview = preview(&file_url(&remote), &central).unwrap();

        let error = acquire(&source_preview, 10).expect_err("symlink must fail");

        assert!(error.to_string().contains("symlink"));
        assert_no_temporary_clone(&central);
    }

    #[test]
    fn acquire_cleans_up_after_clone_failure() {
        let temp = tempdir().unwrap();
        let missing = temp.path().join("missing-source");
        let central = temp.path().join("central");
        let source_preview = preview(&file_url(&missing), &central).unwrap();

        let error = acquire(&source_preview, 10).expect_err("clone must fail");

        assert!(error.to_string().contains("git clone failed"));
        assert!(!source_preview.destination.exists());
        assert_no_temporary_clone(&central);
    }

    fn create_git_repo(path: impl AsRef<Path>, with_skill: bool) -> std::path::PathBuf {
        let path = path.as_ref();
        fs::create_dir_all(path).unwrap();
        git(&["init", path.to_str().unwrap()]);
        if with_skill {
            let skill = path.join("code-review");
            fs::create_dir_all(&skill).unwrap();
            fs::write(skill.join("SKILL.md"), "# Code review").unwrap();
        } else {
            fs::write(path.join("README.md"), "# Empty source").unwrap();
        }
        git(&["-C", path.to_str().unwrap(), "add", "."]);
        git(&[
            "-C",
            path.to_str().unwrap(),
            "-c",
            "user.name=Skills Manager Tests",
            "-c",
            "user.email=tests@example.com",
            "commit",
            "-m",
            "initial",
        ]);
        path.to_path_buf()
    }

    fn git(args: &[&str]) {
        let status = Command::new("git").args(args).status().unwrap();
        assert!(status.success(), "git command failed: {args:?}");
    }

    fn git_output(args: &[&str]) -> String {
        let output = Command::new("git").args(args).output().unwrap();
        assert!(output.status.success(), "git command failed: {args:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn file_url(path: &Path) -> String {
        format!("file://{}", path.display())
    }

    fn assert_no_temporary_clone(central: &Path) {
        let temporary = fs::read_dir(central)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".skills-manager-")
            });
        assert!(!temporary, "temporary clone was not cleaned up");
    }
}
