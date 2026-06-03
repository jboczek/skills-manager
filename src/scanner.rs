use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::git;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    CentralDir,
    ScanParentDir,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub skill_id: String,
    pub skill_path: PathBuf,
    pub skill_relative_path: Option<PathBuf>,
    pub repo_name: Option<String>,
    pub repo_path: Option<PathBuf>,
    pub remote_url: Option<String>,
    pub source_kind: SourceKind,
    pub disambiguation_index: Option<usize>,
}

pub struct ScanConfig {
    pub central_dir: PathBuf,
    pub scan_parent_dirs: Vec<PathBuf>,
    pub max_scan_depth: usize,
}

pub fn scan(scan_config: &ScanConfig) -> anyhow::Result<Vec<ScanResult>> {
    let mut results = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut warnings = Vec::new();

    scan_root(
        &scan_config.central_dir,
        SourceKind::CentralDir,
        scan_config.max_scan_depth,
        &mut results,
        &mut seen_paths,
        &mut warnings,
    );

    for root in &scan_config.scan_parent_dirs {
        scan_root(
            root,
            SourceKind::ScanParentDir,
            scan_config.max_scan_depth,
            &mut results,
            &mut seen_paths,
            &mut warnings,
        );
    }

    for warning in warnings {
        eprintln!("Warning: {warning}");
    }

    Ok(results)
}

#[allow(clippy::ptr_arg)]
pub fn assign_disambiguation_indices(results: &mut Vec<ScanResult>) {
    let mut counts = HashMap::new();
    for result in results.iter() {
        *counts.entry(result.skill_id.clone()).or_insert(0usize) += 1;
    }

    let mut next_indices = HashMap::new();
    for result in results.iter_mut() {
        if counts.get(&result.skill_id).copied().unwrap_or_default() > 1 {
            let next_index = next_indices
                .entry(result.skill_id.clone())
                .or_insert(1usize);
            result.disambiguation_index = Some(*next_index);
            *next_index += 1;
        } else {
            result.disambiguation_index = None;
        }
    }
}

fn scan_root(
    root: &Path,
    source_kind: SourceKind,
    max_scan_depth: usize,
    results: &mut Vec<ScanResult>,
    seen_paths: &mut HashSet<PathBuf>,
    warnings: &mut Vec<String>,
) {
    if !root.exists() || is_symlink(root) {
        return;
    }

    for entry in WalkBuilder::new(root)
        .follow_links(false)
        .hidden(false)
        .max_depth(Some(max_scan_depth))
        .build()
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("failed to scan {}: {error}", root.display()));
                continue;
            }
        };

        if !entry.file_type().is_some_and(|kind| kind.is_file()) || entry.file_name() != "SKILL.md"
        {
            continue;
        }

        let Some(skill_path) = entry.path().parent().map(Path::to_path_buf) else {
            continue;
        };

        if !seen_paths.insert(skill_path.clone()) {
            continue;
        }

        let repo_path = git::find_repo_root(&skill_path);
        let repo_name = repo_path.as_deref().and_then(path_name);
        let skill_relative_path = repo_path.as_deref().and_then(|repo_root| {
            skill_path
                .strip_prefix(repo_root)
                .ok()
                .map(Path::to_path_buf)
        });
        let remote_url =
            repo_path
                .as_deref()
                .and_then(|repo_root| match git::origin_url(repo_root) {
                    Ok(url) => url,
                    Err(error) => {
                        warnings.push(format!(
                            "failed to read origin for {}: {error}",
                            repo_root.display()
                        ));
                        None
                    }
                });
        let skill_name = path_name(&skill_path).unwrap_or_else(|| skill_path.display().to_string());
        let skill_id = match repo_name.as_deref() {
            Some(repo_name) => format!("{repo_name}/{skill_name}"),
            None => skill_name,
        };

        results.push(ScanResult {
            skill_id,
            skill_path,
            skill_relative_path,
            repo_name,
            repo_path,
            remote_url,
            source_kind: source_kind.clone(),
            disambiguation_index: None,
        });
    }
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn path_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn scan_finds_single_skill_md() {
        let temp = tempdir().unwrap();
        let skill_dir = temp.path().join("single-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# skill").unwrap();

        let results = scan(&ScanConfig {
            central_dir: temp.path().to_path_buf(),
            scan_parent_dirs: vec![],
            max_scan_depth: 10,
        })
        .unwrap();

        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.skill_id, "single-skill");
        assert_eq!(result.skill_path, skill_dir);
        assert_eq!(result.skill_relative_path, None);
        assert_eq!(result.repo_name, None);
        assert_eq!(result.repo_path, None);
        assert_eq!(result.remote_url, None);
        assert_eq!(result.source_kind, SourceKind::CentralDir);
        assert_eq!(result.disambiguation_index, None);
    }

    #[test]
    fn scan_finds_multiple_nested_skills_in_same_repo() {
        let temp = tempdir().unwrap();
        let repo_dir = temp.path().join("repo-a");
        let skill_a = repo_dir.join("skill-a");
        let skill_b = repo_dir.join("nested").join("skill-b");
        fs::create_dir_all(repo_dir.join(".git")).unwrap();
        fs::create_dir_all(&skill_a).unwrap();
        fs::create_dir_all(&skill_b).unwrap();
        fs::write(skill_a.join("SKILL.md"), "# skill a").unwrap();
        fs::write(skill_b.join("SKILL.md"), "# skill b").unwrap();

        let results = scan(&ScanConfig {
            central_dir: temp.path().to_path_buf(),
            scan_parent_dirs: vec![],
            max_scan_depth: 10,
        })
        .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|result| {
            result.skill_id == "repo-a/skill-a"
                && result.skill_path == skill_a
                && result.skill_relative_path == Some(PathBuf::from("skill-a"))
                && result.repo_name.as_deref() == Some("repo-a")
                && result.repo_path.as_ref() == Some(&repo_dir)
                && result.remote_url.is_none()
        }));
        assert!(results.iter().any(|result| {
            result.skill_id == "repo-a/skill-b"
                && result.skill_path == skill_b
                && result.skill_relative_path == Some(PathBuf::from("nested").join("skill-b"))
                && result.repo_name.as_deref() == Some("repo-a")
                && result.repo_path.as_ref() == Some(&repo_dir)
                && result.remote_url.is_none()
        }));
    }

    #[cfg(unix)]
    #[test]
    fn scan_does_not_follow_symlinked_directories() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("real-skill");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("SKILL.md"), "# skill").unwrap();

        let link = temp.path().join("linked-skill");
        symlink(&target, &link).unwrap();

        let results = scan(&ScanConfig {
            central_dir: link,
            scan_parent_dirs: vec![],
            max_scan_depth: 10,
        })
        .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn scan_deduplicates_overlapping_scan_roots() {
        let temp = tempdir().unwrap();
        let repo_dir = temp.path().join("repo-a");
        let skill_dir = repo_dir.join("skill-a");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# skill").unwrap();

        let results = scan(&ScanConfig {
            central_dir: temp.path().to_path_buf(),
            scan_parent_dirs: vec![repo_dir.clone()],
            max_scan_depth: 10,
        })
        .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skill_path, skill_dir);
        assert_eq!(results[0].source_kind, SourceKind::CentralDir);
    }

    #[test]
    fn scan_skips_nonexistent_directory() {
        let temp = tempdir().unwrap();
        let missing = temp.path().join("missing");

        let results = scan(&ScanConfig {
            central_dir: missing,
            scan_parent_dirs: vec![],
            max_scan_depth: 10,
        })
        .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn assign_disambiguation_indices_numbers_duplicate_skill_ids() {
        let skill_a = Path::new("skill-a").to_path_buf();
        let skill_b = Path::new("skill-b").to_path_buf();
        let other = Path::new("other").to_path_buf();
        let mut results = vec![
            ScanResult {
                skill_id: "repo/docs".to_string(),
                skill_path: skill_a,
                skill_relative_path: None,
                repo_name: Some("repo".to_string()),
                repo_path: None,
                remote_url: None,
                source_kind: SourceKind::ScanParentDir,
                disambiguation_index: None,
            },
            ScanResult {
                skill_id: "repo/docs".to_string(),
                skill_path: skill_b,
                skill_relative_path: None,
                repo_name: Some("repo".to_string()),
                repo_path: None,
                remote_url: None,
                source_kind: SourceKind::ScanParentDir,
                disambiguation_index: None,
            },
            ScanResult {
                skill_id: "repo/code-review".to_string(),
                skill_path: other,
                skill_relative_path: None,
                repo_name: Some("repo".to_string()),
                repo_path: None,
                remote_url: None,
                source_kind: SourceKind::CentralDir,
                disambiguation_index: Some(99),
            },
        ];

        assign_disambiguation_indices(&mut results);

        assert_eq!(results[0].disambiguation_index, Some(1));
        assert_eq!(results[1].disambiguation_index, Some(2));
        assert_eq!(results[2].disambiguation_index, None);
    }
}
