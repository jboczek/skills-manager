use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::{
    AGENT_ID_CLAUDE, AGENT_ID_CODEX, AGENT_ID_COPILOT, AGENT_PROJECT_DIR_CLAUDE,
    AGENT_PROJECT_DIR_CODEX, AGENT_PROJECT_DIR_COPILOT, SHARED_TARGET_PROJECT_DIR,
};
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
    pub shared_target_dirs: Vec<(PathBuf, Scope)>,
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
    scope: Scope,
    exposures: Vec<SkillExposure>,
}

#[derive(Debug, Clone)]
struct ExposureResolution {
    row_key: String,
    skill_id: SkillId,
    source: SkillSource,
}

#[derive(Debug, Clone)]
struct InventoryTarget {
    agent_id: String,
    path: PathBuf,
    scope: Scope,
    project_root: Option<PathBuf>,
}

pub fn build_inventory(cfg: &InventoryConfig) -> Vec<InventoryRow> {
    let mut rows = Vec::new();
    let mut row_indices = HashMap::new();
    let scan_index = build_scan_index(&cfg.scan_results);

    for target in inventory_targets(cfg) {
        for exposure_path in list_candidate_exposures(&target.path) {
            if target.scope == Scope::ProjectLocal && !contains_skill_file(&exposure_path) {
                continue;
            }
            let Some(skill_name) = path_name(&exposure_path) else {
                continue;
            };
            let connection = detect_connection(&exposure_path);
            let resolution = resolve_exposure(&exposure_path, &skill_name, connection, &scan_index);
            let row_key = contextual_row_key(
                &resolution.row_key,
                target.scope,
                target.project_root.as_deref(),
            );
            let row_index = match row_indices.get(&row_key).copied() {
                Some(existing) => existing,
                None => {
                    let row_index = rows.len();
                    row_indices.insert(row_key, row_index);
                    rows.push(RowAccumulator {
                        skill_id: resolution.skill_id,
                        source: resolution.source,
                        scope: target.scope,
                        exposures: Vec::new(),
                    });
                    row_index
                }
            };

            rows[row_index].exposures.push(SkillExposure {
                agent_id: AgentId(target.agent_id.clone()),
                path: exposure_path,
                connection,
            });
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

pub fn project_root_from_exposure_path(path: &Path) -> Option<PathBuf> {
    let target_dir = path.parent()?;
    if target_dir.file_name()?.to_string_lossy() != "skills" {
        return None;
    }
    let agent_dir = target_dir.parent()?;
    if !matches!(
        agent_dir.file_name()?.to_string_lossy().as_ref(),
        ".claude" | ".codex" | ".copilot" | ".agents"
    ) {
        return None;
    }
    agent_dir.parent().map(Path::to_path_buf)
}

fn build_scan_index(scan_results: &[ScanResult]) -> HashMap<String, ScanResult> {
    scan_results
        .iter()
        .cloned()
        .map(|scan_result| (source_key(&scan_result.skill_path), scan_result))
        .collect()
}

fn inventory_targets(cfg: &InventoryConfig) -> Vec<InventoryTarget> {
    let enabled_agents = cfg
        .agents
        .iter()
        .filter(|agent| agent.enabled)
        .collect::<Vec<_>>();
    let mut targets = Vec::new();

    for agent in &enabled_agents {
        if let Some(path) = &agent.global_dir {
            targets.push(InventoryTarget {
                agent_id: agent.agent_id.clone(),
                path: path.clone(),
                scope: Scope::Global,
                project_root: None,
            });
        }
        targets.extend(
            agent
                .shared_target_dirs
                .iter()
                .filter(|(_, scope)| *scope == Scope::Global)
                .map(|(path, scope)| InventoryTarget {
                    agent_id: agent.agent_id.clone(),
                    path: path.clone(),
                    scope: *scope,
                    project_root: None,
                }),
        );
    }

    let project_roots = cfg
        .scan_results
        .iter()
        .filter_map(|result| result.repo_path.clone())
        .map(|path| fs::canonicalize(&path).unwrap_or(path))
        .collect::<HashSet<_>>();

    for project_root in project_roots {
        for agent in &enabled_agents {
            let project_dirs: &[&str] = match agent.agent_id.as_str() {
                AGENT_ID_CLAUDE => &[AGENT_PROJECT_DIR_CLAUDE],
                AGENT_ID_CODEX => &[AGENT_PROJECT_DIR_CODEX, SHARED_TARGET_PROJECT_DIR],
                AGENT_ID_COPILOT => &[AGENT_PROJECT_DIR_COPILOT, SHARED_TARGET_PROJECT_DIR],
                _ => &[],
            };
            targets.extend(project_dirs.iter().map(|relative_path| InventoryTarget {
                agent_id: agent.agent_id.clone(),
                path: project_root.join(relative_path),
                scope: Scope::ProjectLocal,
                project_root: Some(project_root.clone()),
            }));
        }
    }

    targets
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

fn contains_skill_file(path: &Path) -> bool {
    path.join("SKILL.md").is_file()
}

fn contextual_row_key(row_key: &str, scope: Scope, project_root: Option<&Path>) -> String {
    match (scope, project_root) {
        (Scope::Global, _) => format!("global:{row_key}"),
        (Scope::ProjectLocal, Some(project_root)) => {
            format!("project:{}:{row_key}", source_key(project_root))
        }
        (Scope::ProjectLocal, None) => format!("project:unknown:{row_key}"),
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
    InventoryRow {
        skill_id: row.skill_id,
        source: row.source,
        scope: row.scope,
        exposures: row.exposures,
        disambiguation_index: None,
    }
}

fn display_skill_id(row: &InventoryRow) -> String {
    row.skill_id.to_string()
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
                shared_target_dirs: vec![],
                enabled: true,
            }],
            scan_results: vec![],
        });

        assert!(rows.is_empty());
    }

    #[test]
    fn scanned_skill_without_exposure_is_not_inventory() {
        let rows = build_inventory(&InventoryConfig {
            agents: vec![],
            scan_results: vec![ScanResult {
                skill_id: "skills/code-review".to_string(),
                skill_path: PathBuf::from("/global/skills/code-review"),
                skill_relative_path: Some(PathBuf::from("code-review")),
                repo_name: Some("skills".to_string()),
                repo_path: Some(PathBuf::from("/global/skills")),
                remote_url: None,
                source_kind: SourceKind::CentralDir,
                disambiguation_index: None,
            }],
        });

        assert!(rows.is_empty());
    }

    #[test]
    fn project_agents_directory_is_one_local_row_for_codex_and_copilot() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("analystloop");
        let skill = repo.join(".agents/skills/adx-intake");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "# ADX intake").unwrap();

        let rows = build_inventory(&InventoryConfig {
            agents: vec![
                AgentTarget {
                    agent_id: "codex".to_string(),
                    display_name: "Codex".to_string(),
                    global_dir: None,
                    shared_target_dirs: vec![],
                    enabled: true,
                },
                AgentTarget {
                    agent_id: "copilot".to_string(),
                    display_name: "Copilot".to_string(),
                    global_dir: None,
                    shared_target_dirs: vec![],
                    enabled: true,
                },
            ],
            scan_results: vec![ScanResult {
                skill_id: "analystloop/adx-intake".to_string(),
                skill_path: skill,
                skill_relative_path: Some(PathBuf::from(".agents/skills/adx-intake")),
                repo_name: Some("analystloop".to_string()),
                repo_path: Some(repo),
                remote_url: None,
                source_kind: SourceKind::ScanParentDir,
                disambiguation_index: None,
            }],
        });

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scope, Scope::ProjectLocal);
        assert_eq!(rows[0].exposures.len(), 2);
        assert!(
            rows[0]
                .exposures
                .iter()
                .any(|exposure| exposure.agent_id.0 == "codex")
        );
        assert!(
            rows[0]
                .exposures
                .iter()
                .any(|exposure| exposure.agent_id.0 == "copilot")
        );
    }

    #[cfg(unix)]
    #[test]
    fn same_source_global_and_project_local_exposures_are_separate_rows() {
        let temp = tempdir().unwrap();
        let source_repo = temp.path().join("skills");
        let source = source_repo.join("review");
        let project_repo = temp.path().join("app");
        let project_marker = project_repo.join("skills/marker");
        let global_root = temp.path().join("global-codex");
        let project_root = project_repo.join(".codex/skills");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&project_marker).unwrap();
        fs::create_dir_all(&global_root).unwrap();
        fs::create_dir_all(&project_root).unwrap();
        fs::write(source.join("SKILL.md"), "# Review").unwrap();
        fs::write(project_marker.join("SKILL.md"), "# Marker").unwrap();
        symlink(&source, global_root.join("review")).unwrap();
        symlink(&source, project_root.join("review")).unwrap();

        let rows = build_inventory(&InventoryConfig {
            agents: vec![AgentTarget {
                agent_id: "codex".to_string(),
                display_name: "Codex".to_string(),
                global_dir: Some(global_root),
                shared_target_dirs: vec![],
                enabled: true,
            }],
            scan_results: vec![
                ScanResult {
                    skill_id: "skills/review".to_string(),
                    skill_path: source,
                    skill_relative_path: Some(PathBuf::from("review")),
                    repo_name: Some("skills".to_string()),
                    repo_path: Some(source_repo),
                    remote_url: None,
                    source_kind: SourceKind::CentralDir,
                    disambiguation_index: None,
                },
                ScanResult {
                    skill_id: "app/marker".to_string(),
                    skill_path: project_marker,
                    skill_relative_path: Some(PathBuf::from("skills/marker")),
                    repo_name: Some("app".to_string()),
                    repo_path: Some(project_repo),
                    remote_url: None,
                    source_kind: SourceKind::ScanParentDir,
                    disambiguation_index: None,
                },
            ],
        });

        let review_rows = rows
            .iter()
            .filter(|row| row.skill_id.name == "review")
            .collect::<Vec<_>>();
        assert_eq!(review_rows.len(), 2);
        assert!(review_rows.iter().any(|row| row.scope == Scope::Global));
        assert!(
            review_rows
                .iter()
                .any(|row| row.scope == Scope::ProjectLocal)
        );
    }

    #[test]
    fn project_root_is_derived_from_fixed_exposure_conventions() {
        assert_eq!(
            project_root_from_exposure_path(Path::new(
                "/Users/alice/pgit/analystloop/.agents/skills/adx-intake"
            )),
            Some(PathBuf::from("/Users/alice/pgit/analystloop"))
        );
        assert_eq!(
            project_root_from_exposure_path(Path::new("/tmp/custom/skills/adx-intake")),
            None
        );
    }

    #[test]
    fn fixed_project_directories_map_to_expected_agents() {
        let temp = tempdir().unwrap();
        let repo = temp.path().join("app");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let conventions = [
            (".claude/skills/claude-only", "claude-only"),
            (".codex/skills/codex-only", "codex-only"),
            (".copilot/skills/copilot-only", "copilot-only"),
            (".agents/skills/shared", "shared"),
        ];
        let scan_results = conventions
            .iter()
            .map(|(relative_path, name)| {
                let skill_path = repo.join(relative_path);
                fs::create_dir_all(&skill_path).unwrap();
                fs::write(skill_path.join("SKILL.md"), format!("# {name}")).unwrap();
                ScanResult {
                    skill_id: format!("app/{name}"),
                    skill_path,
                    skill_relative_path: Some(PathBuf::from(relative_path)),
                    repo_name: Some("app".to_string()),
                    repo_path: Some(repo.clone()),
                    remote_url: None,
                    source_kind: SourceKind::ScanParentDir,
                    disambiguation_index: None,
                }
            })
            .collect();

        let rows = build_inventory(&InventoryConfig {
            agents: ["claude", "codex", "copilot"]
                .into_iter()
                .map(|agent_id| AgentTarget {
                    agent_id: agent_id.to_string(),
                    display_name: agent_id.to_string(),
                    global_dir: None,
                    shared_target_dirs: vec![],
                    enabled: true,
                })
                .collect(),
            scan_results,
        });

        let agents_for = |name: &str| {
            rows.iter()
                .find(|row| row.skill_id.name == name)
                .unwrap()
                .exposures
                .iter()
                .map(|exposure| exposure.agent_id.0.as_str())
                .collect::<HashSet<_>>()
        };
        assert_eq!(agents_for("claude-only"), HashSet::from(["claude"]));
        assert_eq!(agents_for("codex-only"), HashSet::from(["codex"]));
        assert_eq!(agents_for("copilot-only"), HashSet::from(["copilot"]));
        assert_eq!(agents_for("shared"), HashSet::from(["codex", "copilot"]));
    }

    #[cfg(unix)]
    #[test]
    fn same_source_exposed_in_two_projects_stays_separate() {
        let temp = tempdir().unwrap();
        let source_repo = temp.path().join("skills");
        let source = source_repo.join("review");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Review").unwrap();

        let mut scan_results = vec![ScanResult {
            skill_id: "skills/review".to_string(),
            skill_path: source.clone(),
            skill_relative_path: Some(PathBuf::from("review")),
            repo_name: Some("skills".to_string()),
            repo_path: Some(source_repo),
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        }];
        for project_name in ["app-one", "app-two"] {
            let project = temp.path().join(project_name);
            let marker = project.join("skills/marker");
            let target = project.join(".codex/skills");
            fs::create_dir_all(&marker).unwrap();
            fs::create_dir_all(&target).unwrap();
            fs::write(marker.join("SKILL.md"), "# Marker").unwrap();
            symlink(&source, target.join("review")).unwrap();
            scan_results.push(ScanResult {
                skill_id: format!("{project_name}/marker"),
                skill_path: marker,
                skill_relative_path: Some(PathBuf::from("skills/marker")),
                repo_name: Some(project_name.to_string()),
                repo_path: Some(project),
                remote_url: None,
                source_kind: SourceKind::ScanParentDir,
                disambiguation_index: None,
            });
        }

        let rows = build_inventory(&InventoryConfig {
            agents: vec![AgentTarget {
                agent_id: "codex".to_string(),
                display_name: "Codex".to_string(),
                global_dir: None,
                shared_target_dirs: vec![],
                enabled: true,
            }],
            scan_results,
        });

        assert_eq!(
            rows.iter()
                .filter(|row| { row.skill_id.name == "review" && row.scope == Scope::ProjectLocal })
                .count(),
            2
        );
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
                    shared_target_dirs: vec![],
                    enabled: true,
                },
                AgentTarget {
                    agent_id: "codex".to_string(),
                    display_name: "Codex".to_string(),
                    global_dir: Some(codex_root),
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
                shared_target_dirs: vec![],
                enabled: true,
            }],
            scan_results: vec![],
        });

        assert!(rows.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn build_inventory_disambiguates_duplicate_display_names() {
        let temp = tempdir().unwrap();
        let skill_a = temp.path().join("repo-a-one").join("docs");
        let skill_b = temp.path().join("repo-a-two").join("docs");
        let codex_root = temp.path().join("codex");
        let copilot_root = temp.path().join("copilot");
        fs::create_dir_all(&skill_a).unwrap();
        fs::create_dir_all(&skill_b).unwrap();
        fs::create_dir_all(&codex_root).unwrap();
        fs::create_dir_all(&copilot_root).unwrap();
        symlink(&skill_a, codex_root.join("docs")).unwrap();
        symlink(&skill_b, copilot_root.join("docs")).unwrap();

        let mut rows = build_inventory(&InventoryConfig {
            agents: vec![
                AgentTarget {
                    agent_id: "codex".to_string(),
                    display_name: "Codex".to_string(),
                    global_dir: Some(codex_root),
                    shared_target_dirs: vec![],
                    enabled: true,
                },
                AgentTarget {
                    agent_id: "copilot".to_string(),
                    display_name: "Copilot".to_string(),
                    global_dir: Some(copilot_root),
                    shared_target_dirs: vec![],
                    enabled: true,
                },
            ],
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
