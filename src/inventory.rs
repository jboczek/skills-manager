use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{
    AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure, SkillId, SkillSource,
};
use crate::git;
use crate::scanner::ScanResult;
use crate::symlink;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTarget {
    pub agent_id: String,
    pub display_name: String,
    pub global_dir: Option<PathBuf>,
    pub project_dir: Option<PathBuf>,
    pub shared_target_dirs: Vec<PathBuf>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct InventoryConfig {
    pub agents: Vec<AgentTarget>,
    pub scan_results: Vec<crate::scanner::ScanResult>,
}

#[derive(Debug, Clone)]
struct RowAccumulator {
    skill_id: SkillId,
    source: SkillSource,
    exposures: Vec<SkillExposure>,
    exposure_scopes: Vec<Scope>,
}

#[derive(Debug, Clone)]
struct ExposureResolution {
    row_key: String,
    skill_id: SkillId,
    source: SkillSource,
}

pub fn build_inventory(cfg: &InventoryConfig) -> Vec<InventoryRow> {
    let mut rows = Vec::new();
    let mut row_indices = HashMap::new();
    let scan_index = build_scan_index(&cfg.scan_results);

    for scan_result in &cfg.scan_results {
        let row = row_from_scan_result(scan_result);
        let row_key = source_key(&scan_result.skill_path);
        row_indices.insert(row_key, rows.len());
        rows.push(RowAccumulator {
            skill_id: row.skill_id,
            source: row.source,
            exposures: Vec::new(),
            exposure_scopes: Vec::new(),
        });
    }

    for agent in cfg.agents.iter().filter(|agent| agent.enabled) {
        for (target_dir, scope) in target_dirs(agent) {
            for exposure_path in list_candidate_exposures(&target_dir) {
                let Some(skill_name) = path_name(&exposure_path) else {
                    continue;
                };
                let connection = detect_connection(&exposure_path);
                let resolution =
                    resolve_exposure(&exposure_path, &skill_name, connection, &scan_index);
                let row_index = match row_indices.get(&resolution.row_key).copied() {
                    Some(existing) => existing,
                    None => {
                        let row_index = rows.len();
                        row_indices.insert(resolution.row_key, row_index);
                        rows.push(RowAccumulator {
                            skill_id: resolution.skill_id,
                            source: resolution.source,
                            exposures: Vec::new(),
                            exposure_scopes: Vec::new(),
                        });
                        row_index
                    }
                };

                rows[row_index].exposures.push(SkillExposure {
                    agent_id: AgentId(agent.agent_id.clone()),
                    path: exposure_path,
                    connection,
                });
                rows[row_index].exposure_scopes.push(scope);
            }
        }
    }

    let mut inventory_rows = rows.into_iter().map(finalize_row).collect::<Vec<_>>();
    inventory_rows.sort_by_cached_key(row_sort_key);
    inventory_rows
}

pub fn assign_disambiguation_indices(rows: &mut [InventoryRow]) {
    let mut counts = HashMap::new();
    for row in rows.iter() {
        *counts.entry(display_skill_id(row)).or_insert(0usize) += 1;
    }

    let mut next_indices = HashMap::new();
    for row in rows.iter_mut() {
        let display = display_skill_id(row);
        if counts.get(&display).copied().unwrap_or_default() > 1 {
            let next_index = next_indices.entry(display).or_insert(1usize);
            row.disambiguation_index = Some(*next_index);
            *next_index += 1;
        } else {
            row.disambiguation_index = None;
        }
    }
}

fn build_scan_index(scan_results: &[ScanResult]) -> HashMap<String, ScanResult> {
    scan_results
        .iter()
        .cloned()
        .map(|scan_result| (source_key(&scan_result.skill_path), scan_result))
        .collect()
}

fn row_from_scan_result(scan_result: &ScanResult) -> InventoryRow {
    InventoryRow {
        skill_id: SkillId {
            namespace: scan_result.repo_name.clone().unwrap_or_default(),
            name: path_name(&scan_result.skill_path)
                .unwrap_or_else(|| scan_result.skill_path.display().to_string()),
        },
        source: SkillSource {
            repo_name: scan_result.repo_name.clone(),
            repo_path: scan_result.repo_path.clone(),
            remote_url: scan_result.remote_url.clone(),
        },
        scope: Scope::ProjectLocal,
        exposures: Vec::new(),
        disambiguation_index: None,
    }
}

fn target_dirs(agent: &AgentTarget) -> Vec<(PathBuf, Scope)> {
    let mut dirs = Vec::new();
    if let Some(path) = &agent.global_dir {
        dirs.push((path.clone(), Scope::Global));
    }
    if let Some(path) = &agent.project_dir {
        dirs.push((path.clone(), Scope::ProjectLocal));
    }
    dirs.extend(
        agent
            .shared_target_dirs
            .iter()
            .cloned()
            .map(|path| (path, Scope::ProjectLocal)),
    );
    dirs
}

fn list_candidate_exposures(target_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(target_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry.file_type().ok().and_then(|file_type| {
                if file_type.is_dir() || file_type.is_symlink() {
                    Some(entry.path())
                } else {
                    None
                }
            })
        })
        .collect()
}

fn detect_connection(path: &Path) -> ConnectionKind {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => ConnectionKind::Symlink,
        Ok(metadata) if metadata.is_dir() => ConnectionKind::PhysicalCopy,
        Ok(_) => ConnectionKind::Unknown,
        Err(_) => ConnectionKind::Missing,
    }
}

fn resolve_exposure(
    exposure_path: &Path,
    skill_name: &str,
    connection: ConnectionKind,
    scan_index: &HashMap<String, ScanResult>,
) -> ExposureResolution {
    if let Some(scan_result) = scan_result_for_path(exposure_path, scan_index) {
        return resolution_from_scan(scan_result);
    }

    if connection == ConnectionKind::Symlink
        && let Some(target_path) = resolve_symlink_target(exposure_path)
    {
        if let Some(scan_result) = scan_result_for_path(&target_path, scan_index) {
            return resolution_from_scan(scan_result);
        }

        let source = source_for_path(&target_path);
        return ExposureResolution {
            row_key: source_key(&target_path),
            skill_id: SkillId {
                namespace: source.repo_name.clone().unwrap_or_default(),
                name: skill_name.to_string(),
            },
            source,
        };
    }

    ExposureResolution {
        row_key: format!("unknown:{skill_name}"),
        skill_id: SkillId {
            namespace: String::new(),
            name: skill_name.to_string(),
        },
        source: SkillSource {
            repo_name: None,
            repo_path: None,
            remote_url: None,
        },
    }
}

fn scan_result_for_path<'a>(
    path: &Path,
    scan_index: &'a HashMap<String, ScanResult>,
) -> Option<&'a ScanResult> {
    let key = source_key(path);
    scan_index.get(&key)
}

fn resolution_from_scan(scan_result: &ScanResult) -> ExposureResolution {
    ExposureResolution {
        row_key: source_key(&scan_result.skill_path),
        skill_id: SkillId {
            namespace: scan_result.repo_name.clone().unwrap_or_default(),
            name: path_name(&scan_result.skill_path)
                .unwrap_or_else(|| scan_result.skill_path.display().to_string()),
        },
        source: SkillSource {
            repo_name: scan_result.repo_name.clone(),
            repo_path: scan_result.repo_path.clone(),
            remote_url: scan_result.remote_url.clone(),
        },
    }
}

fn resolve_symlink_target(path: &Path) -> Option<PathBuf> {
    let target = symlink::read_symlink_target(path).ok()?;
    let absolute_target = if target.is_absolute() {
        target
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).join(target)
    };
    absolute_target.exists().then_some(absolute_target)
}

fn source_for_path(path: &Path) -> SkillSource {
    let repo_path = git::find_repo_root(path);
    SkillSource {
        repo_name: repo_path.as_deref().and_then(path_name),
        remote_url: repo_path
            .as_deref()
            .and_then(|repo| git::origin_url(repo).ok().flatten()),
        repo_path,
    }
}

fn source_key(path: &Path) -> String {
    let normalized = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    format!("source:{}", normalized.display())
}

fn finalize_row(row: RowAccumulator) -> InventoryRow {
    let scope = row
        .exposures
        .iter()
        .zip(row.exposure_scopes.iter().copied())
        .max_by_key(|(exposure, scope)| (connection_rank(exposure.connection), scope_rank(*scope)))
        .map(|(_, scope)| scope)
        .unwrap_or(Scope::ProjectLocal);

    InventoryRow {
        skill_id: row.skill_id,
        source: row.source,
        scope,
        exposures: row.exposures,
        disambiguation_index: None,
    }
}

fn connection_rank(connection: ConnectionKind) -> u8 {
    match connection {
        ConnectionKind::Symlink => 4,
        ConnectionKind::PhysicalCopy => 3,
        ConnectionKind::Missing => 2,
        ConnectionKind::Unknown => 1,
    }
}

fn scope_rank(scope: Scope) -> u8 {
    match scope {
        Scope::Global => 2,
        Scope::ProjectLocal => 1,
    }
}

fn display_skill_id(row: &InventoryRow) -> String {
    if row.skill_id.namespace.is_empty() {
        row.skill_id.name.clone()
    } else {
        format!("{}/{}", row.skill_id.namespace, row.skill_id.name)
    }
}

fn row_sort_key(row: &InventoryRow) -> (String, String, String) {
    (
        display_skill_id(row),
        row.source
            .repo_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        row.exposures
            .first()
            .map(|exposure| exposure.path.display().to_string())
            .unwrap_or_default(),
    )
}

fn path_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;
    use crate::scanner::{ScanResult, SourceKind};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn build_inventory_returns_empty_when_no_targets_exist() {
        let temp = tempdir().unwrap();
        let rows = build_inventory(&InventoryConfig {
            agents: vec![AgentTarget {
                agent_id: "claude".to_string(),
                display_name: "Claude".to_string(),
                global_dir: Some(temp.path().join("missing")),
                project_dir: None,
                shared_target_dirs: vec![],
                enabled: true,
            }],
            scan_results: vec![],
        });

        assert!(rows.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn build_inventory_detects_symlink_exposure() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source-skill");
        let target_root = temp.path().join("claude-skills");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target_root).unwrap();
        symlink(&source, target_root.join("source-skill")).unwrap();

        let rows = build_inventory(&InventoryConfig {
            agents: vec![AgentTarget {
                agent_id: "claude".to_string(),
                display_name: "Claude".to_string(),
                global_dir: Some(target_root),
                project_dir: None,
                shared_target_dirs: vec![],
                enabled: true,
            }],
            scan_results: vec![],
        });

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].exposures.len(), 1);
        assert_eq!(
            rows[0].exposures[0].connection,
            crate::domain::ConnectionKind::Symlink
        );
    }

    #[test]
    fn build_inventory_detects_physical_copy() {
        let temp = tempdir().unwrap();
        let target_root = temp.path().join("claude-skills");
        fs::create_dir_all(target_root.join("physical-skill")).unwrap();

        let rows = build_inventory(&InventoryConfig {
            agents: vec![AgentTarget {
                agent_id: "claude".to_string(),
                display_name: "Claude".to_string(),
                global_dir: Some(target_root),
                project_dir: None,
                shared_target_dirs: vec![],
                enabled: true,
            }],
            scan_results: vec![],
        });

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].exposures[0].connection,
            crate::domain::ConnectionKind::PhysicalCopy
        );
    }

    #[test]
    fn build_inventory_consolidates_same_skill_for_multiple_agents() {
        let temp = tempdir().unwrap();
        let claude_root = temp.path().join("claude-skills");
        let codex_root = temp.path().join("codex-skills");
        fs::create_dir_all(claude_root.join("shared-skill")).unwrap();
        fs::create_dir_all(codex_root.join("shared-skill")).unwrap();

        let rows = build_inventory(&InventoryConfig {
            agents: vec![
                AgentTarget {
                    agent_id: "claude".to_string(),
                    display_name: "Claude".to_string(),
                    global_dir: Some(claude_root),
                    project_dir: None,
                    shared_target_dirs: vec![],
                    enabled: true,
                },
                AgentTarget {
                    agent_id: "codex".to_string(),
                    display_name: "Codex".to_string(),
                    global_dir: Some(codex_root),
                    project_dir: None,
                    shared_target_dirs: vec![],
                    enabled: true,
                },
            ],
            scan_results: vec![],
        });

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].exposures.len(), 2);
    }

    #[test]
    fn build_inventory_handles_missing_target_dir() {
        let rows = build_inventory(&InventoryConfig {
            agents: vec![AgentTarget {
                agent_id: "copilot".to_string(),
                display_name: "Copilot".to_string(),
                global_dir: Some(PathBuf::from("/path/that/does/not/exist")),
                project_dir: None,
                shared_target_dirs: vec![],
                enabled: true,
            }],
            scan_results: vec![],
        });

        assert!(rows.is_empty());
    }

    #[test]
    fn build_inventory_disambiguates_duplicate_display_names() {
        let temp = tempdir().unwrap();
        let skill_a = temp.path().join("repo-a-one").join("docs");
        let skill_b = temp.path().join("repo-a-two").join("docs");
        fs::create_dir_all(&skill_a).unwrap();
        fs::create_dir_all(&skill_b).unwrap();

        let mut rows = build_inventory(&InventoryConfig {
            agents: vec![],
            scan_results: vec![
                ScanResult {
                    skill_id: "repo-a/docs".to_string(),
                    skill_path: skill_a.clone(),
                    skill_relative_path: Some(PathBuf::from("docs")),
                    repo_name: Some("repo-a".to_string()),
                    repo_path: Some(temp.path().join("repo-a-one")),
                    remote_url: None,
                    source_kind: SourceKind::CentralDir,
                    disambiguation_index: None,
                },
                ScanResult {
                    skill_id: "repo-a/docs".to_string(),
                    skill_path: skill_b.clone(),
                    skill_relative_path: Some(PathBuf::from("docs")),
                    repo_name: Some("repo-a".to_string()),
                    repo_path: Some(temp.path().join("repo-a-two")),
                    remote_url: None,
                    source_kind: SourceKind::CentralDir,
                    disambiguation_index: None,
                },
            ],
        });

        assign_disambiguation_indices(&mut rows);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].disambiguation_index, Some(1));
        assert_eq!(rows[1].disambiguation_index, Some(2));
    }
}
