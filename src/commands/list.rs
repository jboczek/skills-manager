use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{self, Config};
use crate::inventory::{self, AgentTarget, InventoryConfig};
use crate::output;
use crate::scanner::{self, ScanConfig};

pub fn run() -> Result<()> {
    let path = Config::default_path().context("cannot determine config file path")?;
    if !path.exists() {
        println!(
            "No config found at {}.\nRun `skills-manager config init` to create one.",
            path.display()
        );
        return Ok(());
    }

    let config = Config::load_from(&path)?;
    let current_dir = env::current_dir().context("failed to determine current directory")?;
    let scan_results = scanner::scan(&ScanConfig {
        central_dir: resolve_path(&current_dir, &config.skills.central_dir),
        scan_parent_dirs: config
            .skills
            .scan_parent_dirs
            .iter()
            .map(|path| resolve_path(&current_dir, path))
            .collect(),
        max_scan_depth: config.skills.max_scan_depth as usize,
    })?;

    let agents = config
        .agents
        .iter()
        .map(|(agent_id, agent)| AgentTarget {
            agent_id: agent_id.clone(),
            display_name: agent.display_name.clone(),
            global_dir: Some(resolve_path(&current_dir, &agent.global_dir)),
            project_dir: agent
                .project_dir
                .as_deref()
                .map(|path| resolve_path(&current_dir, path)),
            shared_target_dirs: agent
                .shared_target_ids
                .iter()
                .filter_map(|target_id| config.shared_targets.get(target_id))
                .filter(|target| target.enabled)
                .map(|target| resolve_path(&current_dir, &target.project_dir))
                .collect(),
            enabled: agent.enabled,
        })
        .collect();

    let mut rows = inventory::build_inventory(&InventoryConfig { agents, scan_results });
    inventory::assign_disambiguation_indices(&mut rows);
    println!("{}", output::render_inventory(&rows));
    Ok(())
}

fn resolve_path(current_dir: &Path, raw_path: &str) -> PathBuf {
    let path = config::expand_tilde(raw_path);
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}
