use anyhow::Result;

use crate::config::{self, Config};
use crate::scanner::{self, ScanConfig, SourceKind};

pub fn run() -> Result<()> {
    let config = load_config()?;
    let mut results = scanner::scan(&ScanConfig {
        central_dir: config::expand_tilde(&config.skills.central_dir),
        scan_parent_dirs: config
            .skills
            .scan_parent_dirs
            .iter()
            .map(|path| config::expand_tilde(path))
            .collect(),
        max_scan_depth: config.skills.max_scan_depth as usize,
    })?;

    scanner::assign_disambiguation_indices(&mut results);

    if results.is_empty() {
        println!("No skills found.");
        return Ok(());
    }

    let skill_width = results
        .iter()
        .map(|result| {
            let skill_display = match result.disambiguation_index {
                Some(index) => format!("({index}) {}", result.skill_id),
                None => result.skill_id.clone(),
            };
            skill_display.len()
        })
        .max()
        .unwrap_or(0);
    let source_width = "[central]".len();
    let path_width = results
        .iter()
        .map(|result| result.skill_path.display().to_string().len())
        .max()
        .unwrap_or(0);
    let repo_width = results
        .iter()
        .map(|result| result.repo_name.as_deref().unwrap_or("unknown").len())
        .max()
        .unwrap_or("unknown".len());

    for result in results {
        let skill_display = match result.disambiguation_index {
            Some(index) => format!("({index}) {}", result.skill_id),
            None => result.skill_id,
        };
        let source_display = format!("[{}]", source_label(&result.source_kind));
        let path_display = result.skill_path.display().to_string();
        let repo = result.repo_name.as_deref().unwrap_or("unknown");
        let origin = result.remote_url.as_deref().unwrap_or("unknown");

        println!(
            "{skill_display:<skill_width$}  {source_display:<source_width$}  {path_display:<path_width$}  {repo:<repo_width$}  {origin}"
        );
    }

    Ok(())
}

fn load_config() -> Result<Config> {
    match Config::default_path() {
        Some(path) if path.exists() => Config::load_from(&path),
        _ => Ok(Config::default_config()),
    }
}

fn source_label(source_kind: &SourceKind) -> &'static str {
    match source_kind {
        SourceKind::CentralDir => "central",
        SourceKind::ScanParentDir => "scan",
    }
}
