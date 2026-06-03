use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{self, Config};
use crate::domain::InventoryRow;
use crate::inventory::{self, AgentTarget, InventoryConfig};
use crate::scanner::{self, ScanConfig, ScanResult};

/// Load config from default path, or return default config if not found.
pub fn load_config() -> Result<Config> {
    match Config::default_path() {
        Some(path) if path.exists() => Config::load_from(&path),
        _ => Ok(Config::default_config()),
    }
}

/// Resolve a raw path string relative to current_dir, expanding tilde first.
pub fn resolve_path(current_dir: &Path, raw_path: &str) -> PathBuf {
    let path = config::expand_tilde(raw_path);
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

/// Build ScanConfig from Config.
pub fn scan_config_from(config: &Config, current_dir: &Path) -> ScanConfig {
    ScanConfig {
        central_dir: resolve_path(current_dir, &config.skills.central_dir),
        scan_parent_dirs: config
            .skills
            .scan_parent_dirs
            .iter()
            .map(|path| resolve_path(current_dir, path))
            .collect(),
        max_scan_depth: config.skills.max_scan_depth as usize,
    }
}

/// Build AgentTargets from Config.
pub fn agent_targets_from(config: &Config, current_dir: &Path) -> Vec<AgentTarget> {
    config
        .agents
        .iter()
        .map(|(agent_id, agent)| AgentTarget {
            agent_id: agent_id.clone(),
            display_name: agent.display_name.clone(),
            global_dir: Some(resolve_path(current_dir, &agent.global_dir)),
            project_dir: agent
                .project_dir
                .as_deref()
                .map(|path| resolve_path(current_dir, path)),
            shared_target_dirs: agent
                .shared_target_ids
                .iter()
                .filter_map(|target_id| config.shared_targets.get(target_id))
                .filter(|target| target.enabled)
                .map(|target| resolve_path(current_dir, &target.project_dir))
                .collect(),
            enabled: agent.enabled,
        })
        .collect()
}

/// Build InventoryConfig: scan + build agent targets.
pub fn build_inventory_config(config: &Config, current_dir: &Path) -> Result<InventoryConfig> {
    let scan_results = scanner::scan(&scan_config_from(config, current_dir))?;
    Ok(InventoryConfig {
        agents: agent_targets_from(config, current_dir),
        scan_results,
    })
}

/// Run a full scan + build inventory + assign disambiguation, returning rows.
pub fn fresh_inventory(config: &Config, current_dir: &Path) -> Result<Vec<InventoryRow>> {
    let inventory_config = build_inventory_config(config, current_dir)?;
    let mut rows = inventory::build_inventory(&inventory_config);
    inventory::assign_disambiguation_indices(&mut rows);
    Ok(rows)
}

/// Check if stdin is interactive (a terminal).
pub fn is_interactive() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

/// Read a line from stdin. Returns None if EOF.
pub fn read_line(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}");
    use std::io::Write;
    std::io::stdout().flush()?;
    let mut input = String::new();
    let n = std::io::stdin().read_line(&mut input)?;
    if n == 0 {
        Ok(None)
    } else {
        Ok(Some(input.trim().to_string()))
    }
}

/// Ask the user a yes/no question. Returns true for 'y'/'Y'. Returns false for anything else.
/// If non-interactive, returns false.
pub fn confirm(prompt: &str) -> Result<bool> {
    if !is_interactive() {
        return Ok(false);
    }

    Ok(matches!(read_line(prompt)?, Some(answer) if answer.eq_ignore_ascii_case("y")))
}

/// Parse a comma-separated list of agent IDs.
pub fn parse_agents(s: &str) -> Vec<String> {
    s.split(',')
        .map(|agent| agent.trim().to_lowercase())
        .filter(|agent| !agent.is_empty())
        .collect()
}

/// Given a skill identifier (e.g. "repo-a/code-review" or "code-review"),
/// find matching scan results. Returns empty vec if none, or multiple if ambiguous.
pub fn find_scan_results_by_id<'a>(
    skill_id: &str,
    results: &'a [ScanResult],
) -> Vec<&'a ScanResult> {
    results
        .iter()
        .filter(|result| matches_skill_id(skill_id, &result.skill_id))
        .collect()
}

/// Given a skill identifier, find matching inventory rows.
pub fn find_inventory_rows_by_id<'a>(
    skill_id: &str,
    rows: &'a [InventoryRow],
) -> Vec<&'a InventoryRow> {
    rows.iter()
        .filter(|row| matches_skill_id(skill_id, &display_skill_id(row)))
        .collect()
}

/// Print numbered disambiguation choices for scan results.
pub fn print_scan_disambiguation(results: &[&ScanResult]) {
    for (index, result) in results.iter().enumerate() {
        println!(
            "  ({}) {}  {}",
            index + 1,
            result.skill_id,
            result.skill_path.display()
        );
    }
}

/// Print numbered disambiguation choices for inventory rows.
pub fn print_inventory_disambiguation(rows: &[&InventoryRow]) {
    for (index, row) in rows.iter().enumerate() {
        println!(
            "  ({}) {}  {}",
            index + 1,
            display_skill_id(row),
            row_context(row)
        );
    }
}

fn matches_skill_id(input: &str, candidate: &str) -> bool {
    if input == candidate {
        return true;
    }

    if input.contains('/') {
        return false;
    }

    candidate.rsplit('/').next() == Some(input)
}

fn display_skill_id(row: &InventoryRow) -> String {
    if row.skill_id.namespace.is_empty() {
        row.skill_id.name.clone()
    } else {
        format!("{}/{}", row.skill_id.namespace, row.skill_id.name)
    }
}

fn row_context(row: &InventoryRow) -> String {
    if let Some(path) = row.source.repo_path.as_ref() {
        return path.display().to_string();
    }
    if let Some(exposure) = row.exposures.first() {
        return exposure.path.display().to_string();
    }
    "unknown".to_string()
}

pub fn current_dir() -> Result<PathBuf> {
    env::current_dir().context("failed to determine current directory")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::scanner::SourceKind;

    fn scan_result(skill_id: &str, path: &str) -> ScanResult {
        ScanResult {
            skill_id: skill_id.to_string(),
            skill_path: PathBuf::from(path),
            skill_relative_path: None,
            repo_name: skill_id.split_once('/').map(|(repo, _)| repo.to_string()),
            repo_path: None,
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        }
    }

    #[test]
    fn parse_agents_splits_comma_separated() {
        assert_eq!(parse_agents("claude,codex"), vec!["claude", "codex"]);
    }

    #[test]
    fn parse_agents_trims_whitespace() {
        assert_eq!(parse_agents("claude, codex"), vec!["claude", "codex"]);
    }

    #[test]
    fn parse_agents_empty_string_returns_empty() {
        assert!(parse_agents("").is_empty());
    }

    #[test]
    fn find_scan_results_by_id_exact_match() {
        let results = vec![
            scan_result("repo-a/code-review", "/skills/repo-a/code-review"),
            scan_result("repo-b/docs", "/skills/repo-b/docs"),
        ];

        let matches = find_scan_results_by_id("repo-a/code-review", &results);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].skill_id, "repo-a/code-review");
    }

    #[test]
    fn find_scan_results_by_id_name_only_match() {
        let results = vec![
            scan_result("repo-a/code-review", "/skills/repo-a/code-review"),
            scan_result("repo-b/code-review", "/skills/repo-b/code-review"),
            scan_result("repo-c/docs", "/skills/repo-c/docs"),
        ];

        let matches = find_scan_results_by_id("code-review", &results);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].skill_id, "repo-a/code-review");
        assert_eq!(matches[1].skill_id, "repo-b/code-review");
    }

    #[test]
    fn find_scan_results_by_id_no_match() {
        let results = vec![scan_result(
            "repo-a/code-review",
            "/skills/repo-a/code-review",
        )];

        assert!(find_scan_results_by_id("missing", &results).is_empty());
    }
}
