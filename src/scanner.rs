use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

pub fn scan_for_skill_markers(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut matches = Vec::new();

    for entry in WalkBuilder::new(root)
        .follow_links(false)
        .hidden(false)
        .build()
    {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_file()) && entry.file_name() == "SKILL.md" {
            matches.push(entry.into_path());
        }
    }

    Ok(matches)
}
