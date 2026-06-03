use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::domain::{ConnectionKind, InventoryRow, Scope};
use crate::scanner::{ScanResult, SourceKind};
use crate::tui::theme::Theme;

/// Render inventory rows as a table in the given area.
pub fn render_inventory_table(
    frame: &mut Frame,
    area: Rect,
    rows: &[InventoryRow],
    scroll: usize,
    selected: Option<usize>,
) {
    if rows.is_empty() {
        render_empty(frame, area, " Inventory ", "No skills found.");
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .title(" Inventory ")
        .title_style(Theme::header())
        .style(Theme::default_style());
    let inner = block.inner(area);
    let visible_rows = usize::from(inner.height.saturating_sub(1)).max(1);
    let start = scroll.min(rows.len().saturating_sub(visible_rows));
    let end = (start + visible_rows).min(rows.len());

    let display_rows = rows[start..end]
        .iter()
        .enumerate()
        .map(|(offset, row)| {
            let index = start + offset;
            let connections = row
                .exposures
                .iter()
                .map(|exposure| connection_label(exposure.connection))
                .collect::<Vec<_>>()
                .join(",");
            let agents = |agent_id: &str| {
                if row
                    .exposures
                    .iter()
                    .any(|exposure| exposure.agent_id.0 == agent_id)
                {
                    "✓"
                } else {
                    "-"
                }
            };
            let table_row = Row::new(vec![
                Cell::from(skill_label(row)),
                Cell::from(source_label(row)),
                Cell::from(agents("claude")),
                Cell::from(agents("codex")),
                Cell::from(agents("copilot")),
                Cell::from(scope_label(row.scope)),
                Cell::from(if connections.is_empty() {
                    "-".to_string()
                } else {
                    connections
                }),
            ]);
            if selected == Some(index) {
                table_row.style(Theme::selected())
            } else {
                table_row.style(Theme::default_style())
            }
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        display_rows,
        [
            Constraint::Length(24),
            Constraint::Length(16),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(12),
        ],
    )
    .header(
        Row::new([
            "SKILL",
            "SOURCE",
            "CLAUDE",
            "CODEX",
            "COPILOT",
            "SCOPE",
            "CONNECTION",
        ])
        .style(Theme::header()),
    )
    .block(block)
    .column_spacing(1)
    .style(Theme::default_style());

    frame.render_widget(table, area);
}

/// Render scan results as a table in the given area.
pub fn render_scan_table(frame: &mut Frame, area: Rect, results: &[ScanResult], scroll: usize) {
    if results.is_empty() {
        render_empty(frame, area, " Scan ", "No scan results available.");
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .title(" Scan ")
        .title_style(Theme::header())
        .style(Theme::default_style());
    let inner = block.inner(area);
    let visible_rows = usize::from(inner.height.saturating_sub(1)).max(1);
    let start = scroll.min(results.len().saturating_sub(visible_rows));
    let end = (start + visible_rows).min(results.len());

    let display_rows = results[start..end]
        .iter()
        .map(|result| {
            Row::new(vec![
                Cell::from(result.skill_id.clone()),
                Cell::from(source_kind_label(result.source_kind.clone())),
                Cell::from(result.repo_name.clone().unwrap_or_else(|| "-".to_string())),
                Cell::from(
                    result
                        .remote_url
                        .clone()
                        .or_else(|| {
                            result
                                .repo_path
                                .as_ref()
                                .map(|path| path.display().to_string())
                        })
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ])
            .style(Theme::default_style())
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        display_rows,
        [
            Constraint::Length(28),
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Min(20),
        ],
    )
    .header(Row::new(["SKILL", "SOURCE", "REPO", "ORIGIN"]).style(Theme::header()))
    .block(block)
    .column_spacing(1)
    .style(Theme::default_style());

    frame.render_widget(table, area);
}

fn render_empty(frame: &mut Frame, area: Rect, title: &str, message: &str) {
    frame.render_widget(
        Paragraph::new(message)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Theme::border())
                    .title(title)
                    .title_style(Theme::header())
                    .style(Theme::default_style()),
            )
            .style(Theme::muted()),
        area,
    );
}

fn skill_label(row: &InventoryRow) -> String {
    let base = if row.skill_id.namespace.is_empty() {
        row.skill_id.name.clone()
    } else {
        format!("{}/{}", row.skill_id.namespace, row.skill_id.name)
    };

    match row.disambiguation_index {
        Some(index) => format!("({index}) {base}"),
        None => base,
    }
}

fn source_label(row: &InventoryRow) -> String {
    let source = row
        .source
        .repo_name
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    if row.disambiguation_index.is_none() {
        return source;
    }

    match source_context(row) {
        Some(context) => format!("{source} ({context})"),
        None => source,
    }
}

fn scope_label(scope: Scope) -> &'static str {
    match scope {
        Scope::Global => "global",
        Scope::ProjectLocal => "local",
    }
}

fn source_context(row: &InventoryRow) -> Option<String> {
    row.source
        .repo_path
        .as_ref()
        .map(|path| path.display().to_string())
        .or_else(|| row.source.remote_url.clone())
        .or_else(|| {
            row.exposures
                .first()
                .map(|exposure| exposure.path.display().to_string())
        })
}

fn connection_label(connection: ConnectionKind) -> &'static str {
    match connection {
        ConnectionKind::Symlink => "symlink",
        ConnectionKind::PhysicalCopy => "copy",
        ConnectionKind::Missing => "missing",
        ConnectionKind::Unknown => "unknown",
    }
}

fn source_kind_label(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::CentralDir => "central",
        SourceKind::ScanParentDir => "parent",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::{AgentId, SkillExposure, SkillId, SkillSource};

    use super::*;

    fn duplicate_row(index: usize, repo_path: &str) -> InventoryRow {
        InventoryRow {
            skill_id: SkillId {
                namespace: "repo-a".to_string(),
                name: "docs".to_string(),
            },
            source: SkillSource {
                repo_name: Some("repo-a".to_string()),
                repo_path: Some(PathBuf::from(repo_path)),
                remote_url: None,
            },
            scope: Scope::ProjectLocal,
            exposures: vec![SkillExposure {
                agent_id: AgentId("codex".to_string()),
                path: PathBuf::from(format!("{repo_path}/docs")),
                connection: ConnectionKind::Symlink,
            }],
            disambiguation_index: Some(index),
        }
    }

    #[test]
    fn duplicate_rows_include_numbered_label_and_source_context() {
        let row = duplicate_row(1, "/tmp/repo-a-one");

        assert_eq!(skill_label(&row), "(1) repo-a/docs");
        assert_eq!(source_label(&row), "repo-a (/tmp/repo-a-one)");
        assert_eq!(scope_label(row.scope), "local");
    }
}
