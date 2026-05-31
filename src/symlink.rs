use std::path::Path;

pub fn is_symlink(path: &Path) -> anyhow::Result<bool> {
    Ok(std::fs::symlink_metadata(path)?.file_type().is_symlink())
}
