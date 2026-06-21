use std::path::{Path, PathBuf};

pub fn is_symlink(path: &Path) -> anyhow::Result<bool> {
    Ok(std::fs::symlink_metadata(path)?.file_type().is_symlink())
}

/// Reads the symlink target without following it (using read_link).
pub fn read_symlink_target(path: &Path) -> anyhow::Result<PathBuf> {
    Ok(std::fs::read_link(path)?)
}

/// Create a symlink at `link` pointing to `target`.
pub fn create_symlink(target: &Path, link: &Path) -> anyhow::Result<()> {
    match std::fs::symlink_metadata(link) {
        Ok(_) => anyhow::bail!("target already exists: {}", link.display()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(target, link)?;

    Ok(())
}

/// Remove a symlink at `link`.
pub fn remove_symlink(link: &Path) -> anyhow::Result<()> {
    let metadata = std::fs::symlink_metadata(link)?;
    if !metadata.file_type().is_symlink() {
        return Err(anyhow::anyhow!("path is not a symlink: {}", link.display()));
    }

    #[cfg(unix)]
    std::fs::remove_file(link)?;

    #[cfg(windows)]
    std::fs::remove_dir(link)?;

    Ok(())
}

/// Remove a physical directory copy.
pub fn remove_physical_copy(path: &Path) -> anyhow::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(anyhow::anyhow!(
            "refusing to remove: path is a symlink, not a physical copy"
        ));
    }
    if !metadata.is_dir() {
        return Err(anyhow::anyhow!(
            "refusing to remove: path is not a directory"
        ));
    }

    std::fs::remove_dir_all(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::{Builder, TempDir};

    use super::*;

    fn test_dir(name: &str) -> TempDir {
        let root = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test-artifacts")
            .join("symlink");
        fs::create_dir_all(&root).unwrap();
        Builder::new().prefix(name).tempdir_in(root).unwrap()
    }

    #[test]
    fn create_symlink_creates_link() {
        let temp = test_dir("create-link-");
        let source = temp.path().join("source");
        let link = temp.path().join("link");
        fs::create_dir_all(&source).unwrap();

        create_symlink(&source, &link).unwrap();

        assert!(is_symlink(&link).unwrap());
        assert_eq!(read_symlink_target(&link).unwrap(), PathBuf::from(&source));
    }

    #[test]
    fn create_symlink_fails_if_target_exists() {
        let temp = test_dir("create-conflict-");
        let source = temp.path().join("source");
        let link = temp.path().join("link");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&link).unwrap();

        let err = create_symlink(&source, &link).unwrap_err();

        assert!(err.to_string().contains("target already exists"));
    }

    #[test]
    fn remove_symlink_removes_only_link_not_source() {
        let temp = test_dir("remove-link-");
        let source = temp.path().join("source");
        let link = temp.path().join("link");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# skill").unwrap();
        create_symlink(&source, &link).unwrap();

        remove_symlink(&link).unwrap();

        assert!(!link.exists());
        assert!(source.exists());
        assert!(source.join("SKILL.md").exists());
    }

    #[test]
    fn remove_symlink_fails_on_non_symlink() {
        let temp = test_dir("remove-non-link-");
        let dir = temp.path().join("dir");
        fs::create_dir_all(&dir).unwrap();

        let err = remove_symlink(&dir).unwrap_err();

        assert!(err.to_string().contains("not a symlink"));
    }

    #[test]
    fn remove_physical_copy_removes_directory() {
        let temp = test_dir("remove-copy-");
        let dir = temp.path().join("copy");
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("nested").join("SKILL.md"), "# skill").unwrap();

        remove_physical_copy(&dir).unwrap();

        assert!(!dir.exists());
    }

    #[test]
    fn remove_physical_copy_refuses_symlink() {
        let temp = test_dir("remove-copy-symlink-");
        let source = temp.path().join("source");
        let link = temp.path().join("link");
        fs::create_dir_all(&source).unwrap();
        create_symlink(&source, &link).unwrap();

        let err = remove_physical_copy(&link).unwrap_err();

        assert!(
            err.to_string()
                .contains("refusing to remove: path is a symlink, not a physical copy")
        );
    }

    #[test]
    fn create_symlink_no_fallback_to_copy_on_conflict() {
        let temp = test_dir("create-no-fallback-");
        let source = temp.path().join("source");
        let link = temp.path().join("link");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&link).unwrap();

        let err = create_symlink(&source, &link).unwrap_err();

        assert!(err.to_string().contains("target already exists"));
        assert!(link.is_dir());
        assert!(!link.join("SKILL.md").exists());
    }
}
