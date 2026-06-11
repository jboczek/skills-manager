use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::domain::{ConnectionKind, InventoryRow, Scope};
use crate::scanner::{ScanResult, SourceKind};
use crate::tui::source_table::{SourceTable, SourceTableRow};
use crate::tui::theme::Theme;

const LIST_SKILL_COLUMN_WIDTH: u16 = 60;
const SCAN_SKILL_COLUMN_WIDTH: u16 = 35;

/// Render inventory rows as a table in the given area.
pub fn render_inventory_table(
    frame: &mut Frame,
    area: Rect,
    rows: &[InventoryRow],
    source_table: &SourceTable<usize>,
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
    let projected_rows = source_table.visible_rows();
    let start = source_table
        .viewport_offset()
        .min(projected_rows.len().saturating_sub(visible_rows));
    let end = (start + visible_rows).min(projected_rows.len());

    let display_rows = projected_rows[start..end]
        .iter()
        .enumerate()
        .filter_map(|(offset, projected_row)| {
            let index = start + offset;
            let selected = source_table.selected_index() == Some(index);
            if let SourceTableRow::Group {
                name,
                context,
                count,
                expanded,
                ..
            } = projected_row
            {
                return Some(
                    Row::new(vec![
                        Cell::from(group_label(*expanded, name, context)),
                        Cell::from(skill_count_label(*count)),
                        Cell::from(""),
                        Cell::from(""),
                        Cell::from(""),
                        Cell::from(""),
                        Cell::from(""),
                    ])
                    .style(row_style(selected)),
                );
            }

            let SourceTableRow::Item {
                item,
                skill_name,
                display_path,
                ..
            } = projected_row
            else {
                return None;
            };
            let row = rows.get(*item)?;
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
                Cell::from(format!("    {skill_name}")),
                Cell::from(display_path.clone()),
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
            Some(if selected {
                table_row.style(Theme::selected())
            } else {
                table_row.style(Theme::default_style())
            })
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        display_rows,
        [
            Constraint::Length(LIST_SKILL_COLUMN_WIDTH),
            Constraint::Length(24),
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
pub fn render_scan_table(
    frame: &mut Frame,
    area: Rect,
    results: &[ScanResult],
    source_table: &SourceTable<usize>,
) {
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
    let projected_rows = source_table.visible_rows();
    let start = source_table
        .viewport_offset()
        .min(projected_rows.len().saturating_sub(visible_rows));
    let end = (start + visible_rows).min(projected_rows.len());

    let display_rows = projected_rows[start..end]
        .iter()
        .enumerate()
        .filter_map(|(offset, projected_row)| {
            let index = start + offset;
            let selected = source_table.selected_index() == Some(index);
            if let SourceTableRow::Group {
                name,
                context,
                count,
                expanded,
                ..
            } = projected_row
            {
                return Some(
                    Row::new(vec![
                        Cell::from(group_label(*expanded, name, context)),
                        Cell::from(skill_count_label(*count)),
                        Cell::from(""),
                        Cell::from(""),
                    ])
                    .style(row_style(selected)),
                );
            }

            let SourceTableRow::Item {
                item,
                skill_name,
                display_path,
                ..
            } = projected_row
            else {
                return None;
            };
            let result = results.get(*item)?;
            Some(
                Row::new(vec![
                    Cell::from(format!("    {skill_name}")),
                    Cell::from(display_path.clone()),
                    Cell::from(source_kind_label(result.source_kind.clone())),
                    Cell::from(result.remote_url.clone().unwrap_or_else(|| "-".to_string())),
                ])
                .style(row_style(selected)),
            )
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        display_rows,
        [
            Constraint::Length(SCAN_SKILL_COLUMN_WIDTH),
            Constraint::Length(28),
            Constraint::Length(12),
            Constraint::Min(20),
        ],
    )
    .header(Row::new(["SKILL", "PATH", "SOURCE", "ORIGIN"]).style(Theme::header()))
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

fn group_label(expanded: bool, name: &str, context: &str) -> String {
    let marker = if expanded { "v" } else { ">" };
    if context.is_empty() || context == name {
        format!("{marker} {name}")
    } else {
        format!("{marker} {name} · {context}")
    }
}

fn skill_count_label(count: usize) -> String {
    if count == 1 {
        "1 skill".to_string()
    } else {
        format!("{count} skills")
    }
}

fn scope_label(scope: Scope) -> &'static str {
    match scope {
        Scope::Global => "global",
        Scope::ProjectLocal => "local",
    }
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

fn row_style(selected: bool) -> ratatui::style::Style {
    if selected {
        Theme::selected()
    } else {
        Theme::default_style()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_rows_include_marker_context_and_count() {
        assert_eq!(
            group_label(false, "skills", "pgit/skills"),
            "> skills · pgit/skills"
        );
        assert_eq!(
            group_label(true, "skills", "pgit/skills"),
            "v skills · pgit/skills"
        );
        assert_eq!(skill_count_label(1), "1 skill");
        assert_eq!(skill_count_label(2), "2 skills");
    }

    #[test]
    fn row_style_marks_selected_row() {
        assert_eq!(row_style(true), Theme::selected());
        assert_eq!(row_style(false), Theme::default_style());
    }

    #[test]
    fn skill_columns_are_wider_in_list_and_scan() {
        assert_eq!(LIST_SKILL_COLUMN_WIDTH, 30);
        assert_eq!(SCAN_SKILL_COLUMN_WIDTH, 35);
    }
}
