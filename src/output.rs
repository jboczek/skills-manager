use std::fs;
use std::path::PathBuf;

use crate::constants::AGENT_COLUMNS;
use crate::domain::{ConnectionKind, InventoryRow, Scope};
use crate::inventory;

pub fn render_inventory(rows: &[InventoryRow]) -> String {
    if rows.is_empty() {
        return "No skills found.".to_string();
    }

    let skill_width = rows
        .iter()
        .map(|row| skill_display(row).len())
        .max()
        .unwrap_or(24)
        .max(24);
    let source_width = rows
        .iter()
        .map(|row| source_display(row).len())
        .max()
        .unwrap_or(14)
        .max(14);
    let scope_width = rows
        .iter()
        .map(|row| scope_display(row).len())
        .max()
        .unwrap_or(8)
        .max(8);
    let connection_width = rows
        .iter()
        .map(|row| connection_display(row).len())
        .max()
        .unwrap_or(10)
        .max(10);

    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(format!(
        "{:<skill_width$}  {:<source_width$}  {:<7}  {:<7}  {:<7}  {:<scope_width$}  {:<connection_width$}",
        "SKILL", "SOURCE", "CLAUDE", "CODEX", "COPILOT", "SCOPE", "CONNECTION"
    ));

    for row in rows {
        lines.push(format!(
            "{:<skill_width$}  {:<source_width$}  {:<7}  {:<7}  {:<7}  {:<scope_width$}  {:<connection_width$}",
            skill_display(row),
            source_display(row),
            exposure_mark(row, AGENT_COLUMNS[0].0),
            exposure_mark(row, AGENT_COLUMNS[1].0),
            exposure_mark(row, AGENT_COLUMNS[2].0),
            scope_display(row),
            connection_display(row),
        ));
    }

    lines.join("\n")
}

fn skill_display(row: &InventoryRow) -> String {
    match row.disambiguation_index {
        Some(index) => format!("({index}) {}", row.skill_id),
        None => row.skill_id.to_string(),
    }
}

fn source_display(row: &InventoryRow) -> String {
    let source = source_path(row)
        .map(|path| path.display().to_string())
        .or_else(|| {
            row.source
                .repo_path
                .as_ref()
                .map(|path| path.display().to_string())
        })
        .or_else(|| row.source.repo_name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    if row.scope == Scope::ProjectLocal
        && let Some(project_root) = row
            .exposures
            .first()
            .and_then(|exposure| inventory::project_root_from_exposure_path(&exposure.path))
        && !source.starts_with(&project_root.display().to_string())
    {
        return format!("{source} (project: {})", project_root.display());
    }

    source
}

fn exposure_mark(row: &InventoryRow, agent_id: &str) -> &'static str {
    if row
        .exposures
        .iter()
        .any(|exposure| exposure.agent_id.0 == agent_id)
    {
        "✓"
    } else {
        "-"
    }
}

fn scope_display(row: &InventoryRow) -> &'static str {
    if row.exposures.is_empty() {
        "unknown"
    } else {
        match row.scope {
            Scope::Global => "global",
            Scope::ProjectLocal => "project-local",
        }
    }
}

fn connection_display(row: &InventoryRow) -> &'static str {
    match primary_connection(row) {
        Some(ConnectionKind::Symlink) => "symlink",
        Some(ConnectionKind::PhysicalCopy) => "physical",
        Some(ConnectionKind::Missing) => "missing",
        Some(ConnectionKind::Unknown) | None => "unknown",
    }
}

fn primary_connection(row: &InventoryRow) -> Option<ConnectionKind> {
    row.exposures
        .iter()
        .map(|exposure| exposure.connection)
        .max_by_key(|connection| match connection {
            ConnectionKind::Symlink => 4,
            ConnectionKind::PhysicalCopy => 3,
            ConnectionKind::Missing => 2,
            ConnectionKind::Unknown => 1,
        })
}

fn source_path(row: &InventoryRow) -> Option<PathBuf> {
    let exposure = row.exposures.first()?;
    if exposure.connection == ConnectionKind::Symlink {
        return fs::canonicalize(&exposure.path)
            .ok()
            .or_else(|| Some(exposure.path.clone()));
    }
    Some(exposure.path.clone())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::constants::{AGENT_ID_CODEX, AGENT_ID_COPILOT};
    use crate::domain::{AgentId, SkillExposure, SkillId, SkillSource};

    use super::*;

    #[test]
    fn duplicate_rows_include_source_path_context() {
        let rows = vec![
            InventoryRow {
                skill_id: SkillId {
                    namespace: "repo-a".to_string(),
                    name: "docs".to_string(),
                },
                source: SkillSource {
                    repo_name: Some("repo-a".to_string()),
                    repo_path: Some(PathBuf::from("/tmp/repo-a-one")),
                    remote_url: None,
                },
                scope: Scope::ProjectLocal,
                exposures: vec![SkillExposure {
                    agent_id: AgentId(AGENT_ID_CODEX.to_string()),
                    path: PathBuf::from("/tmp/repo-a-one/docs"),
                    connection: ConnectionKind::Symlink,
                }],
                disambiguation_index: Some(1),
            },
            InventoryRow {
                skill_id: SkillId {
                    namespace: "repo-a".to_string(),
                    name: "docs".to_string(),
                },
                source: SkillSource {
                    repo_name: Some("repo-a".to_string()),
                    repo_path: Some(PathBuf::from("/tmp/repo-a-two")),
                    remote_url: None,
                },
                scope: Scope::ProjectLocal,
                exposures: vec![SkillExposure {
                    agent_id: AgentId(AGENT_ID_COPILOT.to_string()),
                    path: PathBuf::from("/tmp/repo-a-two/docs"),
                    connection: ConnectionKind::Symlink,
                }],
                disambiguation_index: Some(2),
            },
        ];

        let output = render_inventory(&rows);

        assert!(output.contains("(1) repo-a/docs"), "{output}");
        assert!(output.contains("/tmp/repo-a-one"), "{output}");
        assert!(output.contains("(2) repo-a/docs"), "{output}");
        assert!(output.contains("/tmp/repo-a-two"), "{output}");
    }

    #[test]
    fn project_local_physical_source_includes_project_path() {
        let rows = vec![InventoryRow {
            skill_id: SkillId {
                namespace: "analystloop".to_string(),
                name: "adx-intake".to_string(),
            },
            source: SkillSource {
                repo_name: Some("analystloop".to_string()),
                repo_path: Some(PathBuf::from("/Users/alice/pgit/analystloop")),
                remote_url: None,
            },
            scope: Scope::ProjectLocal,
            exposures: vec![
                SkillExposure {
                    agent_id: AgentId(AGENT_ID_CODEX.to_string()),
                    path: PathBuf::from("/Users/alice/pgit/analystloop/.agents/skills/adx-intake"),
                    connection: ConnectionKind::PhysicalCopy,
                },
                SkillExposure {
                    agent_id: AgentId(AGENT_ID_COPILOT.to_string()),
                    path: PathBuf::from("/Users/alice/pgit/analystloop/.agents/skills/adx-intake"),
                    connection: ConnectionKind::PhysicalCopy,
                },
            ],
            disambiguation_index: None,
        }];

        let output = render_inventory(&rows);

        assert!(
            output.contains("/Users/alice/pgit/analystloop/.agents/skills/adx-intake"),
            "{output}"
        );
        assert!(output.contains("project-local"), "{output}");
    }
}
