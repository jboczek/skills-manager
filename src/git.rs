use std::path::{Path, PathBuf};
use std::process::Command;

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

#[cfg(test)]
mod tests {
    use std::fs;

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
}
