use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::InventoryRow;
use crate::scanner::ScanResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFilter {
    Full,
    OnlyExposed,
    OnlyDiscovered,
}

impl ListFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::OnlyExposed => "Only exposed",
            Self::OnlyDiscovered => "Only discovered not applied",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Full => Self::OnlyExposed,
            Self::OnlyExposed => Self::OnlyDiscovered,
            Self::OnlyDiscovered => Self::Full,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnifiedListRow {
    Exposed(InventoryRow),
    Discovered(ScanResult),
}

pub fn project_rows(
    inventory: &[InventoryRow],
    scan_results: &[ScanResult],
    filter: ListFilter,
) -> Vec<UnifiedListRow> {
    let exposed_sources = inventory
        .iter()
        .flat_map(|row| row.exposures.iter())
        .map(|exposure| canonical_source_identity(&exposure.path))
        .collect::<HashSet<_>>();
    let discovered = scan_results
        .iter()
        .filter(|result| !exposed_sources.contains(&canonical_source_identity(&result.skill_path)))
        .cloned()
        .map(UnifiedListRow::Discovered);

    match filter {
        ListFilter::Full => inventory
            .iter()
            .cloned()
            .map(UnifiedListRow::Exposed)
            .chain(discovered)
            .collect(),
        ListFilter::OnlyExposed => inventory
            .iter()
            .cloned()
            .map(UnifiedListRow::Exposed)
            .collect(),
        ListFilter::OnlyDiscovered => discovered.collect(),
    }
}

fn canonical_source_identity(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::{ListFilter, UnifiedListRow, project_rows};
    use crate::domain::{
        AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure, SkillId, SkillSource,
    };
    use crate::scanner::{ScanResult, SourceKind};

    fn inventory_row(skill_id: &str, exposure_path: PathBuf) -> InventoryRow {
        let (namespace, name) = skill_id.split_once('/').unwrap_or(("", skill_id));
        InventoryRow {
            skill_id: SkillId {
                namespace: namespace.to_string(),
                name: name.to_string(),
            },
            source: SkillSource {
                repo_name: (!namespace.is_empty()).then(|| namespace.to_string()),
                repo_path: None,
                remote_url: None,
            },
            scope: Scope::Global,
            exposures: vec![SkillExposure {
                agent_id: AgentId("codex".to_string()),
                path: exposure_path,
                connection: ConnectionKind::Symlink,
            }],
            disambiguation_index: None,
        }
    }

    fn scan_result(skill_id: &str, skill_path: PathBuf) -> ScanResult {
        ScanResult {
            skill_id: skill_id.to_string(),
            skill_path,
            skill_relative_path: None,
            repo_name: skill_id
                .split_once('/')
                .map(|(repository, _)| repository.to_string()),
            repo_path: None,
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        }
    }

    fn ids(rows: &[UnifiedListRow]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                UnifiedListRow::Exposed(row) => row.skill_id.to_string(),
                UnifiedListRow::Discovered(result) => result.skill_id.clone(),
            })
            .collect()
    }

    #[test]
    fn full_keeps_every_exposure_and_adds_only_unexposed_discoveries() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source-skill");
        let unmatched_exposure = temp.path().join("legacy-copy");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&unmatched_exposure).unwrap();

        let rows = project_rows(
            &[
                inventory_row("repo/exposed", source.clone()),
                inventory_row("legacy", unmatched_exposure),
            ],
            &[
                scan_result("repo/exposed", source),
                scan_result("repo/discovered", temp.path().join("discovered")),
            ],
            ListFilter::Full,
        );

        assert_eq!(
            ids(&rows),
            vec!["repo/exposed", "legacy", "repo/discovered"]
        );
    }

    #[test]
    fn only_exposed_preserves_inventory_rows_without_matching_sources() {
        let rows = project_rows(
            &[inventory_row("legacy", PathBuf::from("/legacy/copy"))],
            &[scan_result(
                "repo/discovered",
                PathBuf::from("/source/discovered"),
            )],
            ListFilter::OnlyExposed,
        );

        assert_eq!(ids(&rows), vec!["legacy"]);
    }

    #[cfg(unix)]
    #[test]
    fn only_discovered_uses_canonical_source_identity_to_skip_exposed_sources() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().unwrap();
        let source = temp.path().join("source-skill");
        let exposure = temp.path().join("codex-skill");
        fs::create_dir_all(&source).unwrap();
        symlink(&source, &exposure).unwrap();

        let rows = project_rows(
            &[inventory_row("repo/exposed", exposure)],
            &[
                scan_result("repo/exposed", source),
                scan_result("repo/discovered", temp.path().join("discovered")),
            ],
            ListFilter::OnlyDiscovered,
        );

        assert_eq!(ids(&rows), vec!["repo/discovered"]);
    }

    #[test]
    fn filter_cycle_is_full_then_exposed_then_discovered() {
        assert_eq!(ListFilter::Full.next(), ListFilter::OnlyExposed);
        assert_eq!(ListFilter::OnlyExposed.next(), ListFilter::OnlyDiscovered);
        assert_eq!(ListFilter::OnlyDiscovered.next(), ListFilter::Full);
    }

    #[test]
    fn discovery_rows_remain_distinct_from_exposure_rows() {
        let rows = project_rows(
            &[inventory_row(
                "legacy",
                Path::new("/legacy/copy").to_path_buf(),
            )],
            &[scan_result(
                "repo/discovered",
                Path::new("/source/discovered").to_path_buf(),
            )],
            ListFilter::Full,
        );

        assert!(matches!(rows[0], UnifiedListRow::Exposed(_)));
        assert!(matches!(rows[1], UnifiedListRow::Discovered(_)));
    }
}
