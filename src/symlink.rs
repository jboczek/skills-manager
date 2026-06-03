use std::path::{Path, PathBuf};

pub fn is_symlink(path: &Path) -> anyhow::Result<bool> {
    Ok(std::fs::symlink_metadata(path)?.file_type().is_symlink())
}

/// Reads the symlink target without following it (using read_link).
pub fn read_symlink_target(path: &Path) -> anyhow::Result<PathBuf> {
    Ok(std::fs::read_link(path)?)
}
