use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryCommit {
    pub id: String,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryUpdate {
    pub repo_path: PathBuf,
    pub commits: Vec<RepositoryCommit>,
}

pub fn find_repo_root(path: &Path) -> Option<PathBuf> {
    let start = if path.is_dir() { path } else { path.parent()? };

    start
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(Path::to_path_buf)
}

pub fn origin_url(repo: &Path) -> anyhow::Result<Option<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["remote", "get-url", "origin"])
        .output()?;

    if output.status.success() {
        let url = String::from_utf8(output.stdout)?.trim().to_owned();
        Ok(Some(url))
    } else {
        Ok(None)
    }
}

pub fn repository_update(repo: &Path) -> anyhow::Result<Option<RepositoryUpdate>> {
    let fetch = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["fetch", "--quiet", "origin"])
        .output()
        .with_context(|| format!("failed to start git fetch for {}", repo.display()))?;
    if !fetch.status.success() {
        bail!(
            "git fetch failed for {}: {}",
            repo.display(),
            String::from_utf8_lossy(&fetch.stderr).trim()
        );
    }

    let upstream = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .output()?;
    if !upstream.status.success() {
        return Ok(None);
    }
    let upstream = String::from_utf8(upstream.stdout)?.trim().to_owned();

    let log = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", "--format=%h%x09%s", &format!("HEAD..{upstream}")])
        .output()?;
    if !log.status.success() {
        bail!(
            "git log failed for {}: {}",
            repo.display(),
            String::from_utf8_lossy(&log.stderr).trim()
        );
    }

    let commits = String::from_utf8(log.stdout)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (id, subject) = line.split_once('\t').unwrap_or((line, ""));
            RepositoryCommit {
                id: id.to_string(),
                subject: subject.to_string(),
            }
        })
        .collect::<Vec<_>>();
    if commits.is_empty() {
        return Ok(None);
    }

    Ok(Some(RepositoryUpdate {
        repo_path: repo.to_path_buf(),
        commits,
    }))
}

pub fn pull_repository(repo: &Path) -> anyhow::Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["pull", "--ff-only"])
        .output()
        .with_context(|| format!("failed to start git pull for {}", repo.display()))?;
    if output.status.success() {
        return Ok(());
    }

    bail!(
        "git pull failed for {}: {}",
        repo.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
}

pub fn clone_repository(url: &str, destination: &Path) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args([
            "-c",
            "submodule.recurse=false",
            "clone",
            "--no-recurse-submodules",
        ])
        .arg(url)
        .arg(destination)
        .output()
        .with_context(|| format!("failed to start git clone for {url}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("git clone failed: {}", stderr.trim());
}

pub fn clone_arguments(url: &str, destination: &Path) -> Vec<String> {
    vec![
        "-c".to_string(),
        "submodule.recurse=false".to_string(),
        "clone".to_string(),
        "--no-recurse-submodules".to_string(),
        url.to_string(),
        destination.display().to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn find_repo_root_finds_git_ancestor() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("repo-a");
        let nested = repo.join("nested").join("skill");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir_all(&nested).unwrap();

        let found = find_repo_root(&nested);

        assert_eq!(found, Some(repo));
    }

    #[test]
    fn find_repo_root_returns_none_without_git_in_hierarchy() {
        let temp = tempdir().unwrap();
        let nested = temp.path().join("nested").join("skill");
        fs::create_dir_all(&nested).unwrap();

        let found = find_repo_root(&nested);

        assert_eq!(found, None);
    }

    #[test]
    fn repository_update_lists_missing_upstream_commits_and_pull_applies_them() {
        let temp = tempdir().unwrap();
        let remote = temp.path().join("remote.git");
        let local = temp.path().join("local");
        let publisher = temp.path().join("publisher");

        git(
            temp.path(),
            &["init", "--bare", "--initial-branch=main", "remote.git"],
        );
        git(temp.path(), &["clone", remote.to_str().unwrap(), "local"]);
        configure_user(&local);
        fs::write(local.join("README.md"), "base\n").unwrap();
        git(&local, &["add", "README.md"]);
        git(&local, &["commit", "-m", "base commit"]);
        git(&local, &["push", "--set-upstream", "origin", "main"]);

        git(
            temp.path(),
            &["clone", remote.to_str().unwrap(), "publisher"],
        );
        configure_user(&publisher);
        fs::write(publisher.join("README.md"), "base\nremote update\n").unwrap();
        git(&publisher, &["add", "README.md"]);
        git(&publisher, &["commit", "-m", "remote update"]);
        git(&publisher, &["push", "origin", "main"]);

        let update = repository_update(&local)
            .unwrap()
            .expect("remote update should be detected");

        assert_eq!(update.commits.len(), 1);
        assert_eq!(update.commits[0].subject, "remote update");
        assert!(update.commits[0].id.len() >= 7);

        pull_repository(&local).unwrap();

        assert_eq!(
            fs::read_to_string(local.join("README.md")).unwrap(),
            "base\nremote update\n"
        );
        assert!(repository_update(&local).unwrap().is_none());
    }

    fn git(directory: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn configure_user(directory: &Path) {
        git(directory, &["config", "user.email", "tests@example.com"]);
        git(directory, &["config", "user.name", "Tests"]);
    }
}
