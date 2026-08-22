use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::domain::{ConnectionKind, InventoryRow, Scope};
use crate::tui::source_table::{SourceTable, SourceTableRow};
use crate::tui::theme::Theme;
use crate::tui::unified_list::UnifiedListRow;

const LIST_SKILL_COLUMN_WIDTH: u16 = 57;
const LIST_SCOPE_COLUMN_WIDTH: u16 = 13;

/// Render inventory rows as a table in the given area.
pub fn render_inventory_table(
    frame: &mut Frame,
    area: Rect,
    rows: &[InventoryRow],
    source_table: &SourceTable,
) {
    let rows = rows
        .iter()
        .cloned()
        .map(UnifiedListRow::Exposed)
        .collect::<Vec<_>>();
    render_unified_inventory_table(frame, area, &rows, source_table);
}

pub fn render_unified_inventory_table(
    frame: &mut Frame,
    area: Rect,
    rows: &[UnifiedListRow],
    source_table: &SourceTable,
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
                repository_update,
                ..
            } = projected_row
            {
                let row = Row::new(vec![
                    Cell::from(group_cell_label(
                        *expanded,
                        name,
                        context,
                        repository_update.is_some(),
                    )),
                    Cell::from(skill_count_label(*count)),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
                .height(if repository_update.is_some() { 3 } else { 1 });
                return Some(row.style(row_style(selected)));
            }

            let SourceTableRow::Item {
                item,
                skill_name,
                display_path,
                checked,
                ..
            } = projected_row
            else {
                return None;
            };
            let table_row = match rows.get(*item)? {
                UnifiedListRow::Exposed(row) => {
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
                    let check = if *checked { "[x]" } else { "[ ]" };
                    Row::new(vec![
                        Cell::from(format!("  {check} {skill_name}")),
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
                    ])
                }
                UnifiedListRow::Discovered(_) => {
                    let check = if *checked { "[x]" } else { "[ ]" };
                    Row::new(vec![
                        Cell::from(format!("  {check} {skill_name}")),
                        Cell::from(display_path.clone()),
                        Cell::from("-"),
                        Cell::from("-"),
                        Cell::from("-"),
                        Cell::from("-"),
                        Cell::from("not exposed"),
                    ])
                }
            };
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
            Constraint::Length(LIST_SCOPE_COLUMN_WIDTH),
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

fn group_cell_label(expanded: bool, name: &str, context: &str, has_update: bool) -> String {
    let label = group_label(expanded, name, context);
    if has_update {
        format!("{label}\nNew version of repository available\n(press Cmd+U to update)")
    } else {
        label
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
        Scope::ProjectLocal => "project-local",
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

fn row_style(selected: bool) -> ratatui::style::Style {
    if selected {
        Theme::selected()
    } else {
        Theme::default_style()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::domain::{AgentId, SkillExposure, SkillId, SkillSource};
    use crate::git::{RepositoryCommit, RepositoryUpdate};
    use crate::scanner::{ScanResult, SourceKind};
    use crate::tui::source_table::SourceGroupItem;
    use crate::tui::unified_list::UnifiedListRow;

    fn rendered_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

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
    fn group_update_label_explains_the_cmd_u_action() {
        let label = group_cell_label(false, "skills", "pgit/skills", true);

        assert!(label.contains("New version of repository available"));
        assert!(label.contains("press Cmd+U to update"));
    }

    #[test]
    fn list_skill_column_width_is_preserved() {
        assert_eq!(LIST_SKILL_COLUMN_WIDTH, 57);
    }

    #[test]
    fn list_table_renders_full_project_local_scope_label() {
        let rows = vec![InventoryRow {
            skill_id: SkillId {
                namespace: "project".to_string(),
                name: "adx-intake".to_string(),
            },
            source: SkillSource {
                repo_name: Some("project".to_string()),
                repo_path: Some(PathBuf::from("/workspace/project")),
                remote_url: None,
            },
            scope: Scope::ProjectLocal,
            exposures: vec![SkillExposure {
                agent_id: AgentId("codex".to_string()),
                path: PathBuf::from("/workspace/project/.agents/skills/adx-intake"),
                connection: ConnectionKind::PhysicalCopy,
            }],
            disambiguation_index: None,
        }];
        let mut source_table = SourceTable::new(vec![SourceGroupItem {
            item: 0,
            skill_name: "project/adx-intake".to_string(),
            skill_path: PathBuf::from("/workspace/project/.agents/skills/adx-intake"),
            repo_name: Some("project".to_string()),
            repo_path: Some(PathBuf::from("/workspace/project")),
            relative_path: Some(PathBuf::from(".agents/skills/adx-intake")),
            allow_repository_update: false,
        }]);
        source_table.move_right(10);
        let backend = TestBackend::new(140, 6);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render_inventory_table(frame, frame.area(), &rows, &source_table))
            .unwrap();

        let output = rendered_lines(&terminal).join("\n");
        assert!(output.contains("project-local"), "{output}");
    }

    #[test]
    fn inventory_table_renders_all_headers_and_discovery_only_values() {
        let skill_path = PathBuf::from("/workspace/repository/discovered");
        let rows = vec![UnifiedListRow::Discovered(ScanResult {
            skill_id: "repository/discovered".to_string(),
            skill_path: skill_path.clone(),
            skill_relative_path: Some(PathBuf::from("discovered")),
            repo_name: Some("repository".to_string()),
            repo_path: Some(PathBuf::from("/workspace/repository")),
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        })];
        let mut source_table = SourceTable::new(vec![SourceGroupItem {
            item: 0,
            skill_name: "discovered".to_string(),
            skill_path,
            repo_name: Some("repository".to_string()),
            repo_path: Some(PathBuf::from("/workspace/repository")),
            relative_path: Some(PathBuf::from("discovered")),
            allow_repository_update: true,
        }]);
        source_table.move_right(10);
        let backend = TestBackend::new(140, 6);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render_unified_inventory_table(frame, frame.area(), &rows, &source_table))
            .unwrap();

        let output = rendered_lines(&terminal).join("\n");
        for header in [
            "SKILL",
            "SOURCE",
            "CLAUDE",
            "CODEX",
            "COPILOT",
            "SCOPE",
            "CONNECTION",
        ] {
            assert!(output.contains(header), "{output}");
        }
        assert!(output.contains("not exposed"), "{output}");
        assert!(output.contains("[ ] discovered"), "{output}");
    }

    #[test]
    fn inventory_table_renders_repository_update_notice_on_group_row() {
        let rows = vec![UnifiedListRow::Discovered(ScanResult {
            skill_id: "repository/discovered".to_string(),
            skill_path: PathBuf::from("/workspace/repository/discovered"),
            skill_relative_path: Some(PathBuf::from("discovered")),
            repo_name: Some("repository".to_string()),
            repo_path: Some(PathBuf::from("/workspace/repository")),
            remote_url: Some("https://example.com/repository.git".to_string()),
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        })];
        let mut source_table = SourceTable::new(vec![SourceGroupItem {
            item: 0,
            skill_name: "discovered".to_string(),
            skill_path: PathBuf::from("/workspace/repository/discovered"),
            repo_name: Some("repository".to_string()),
            repo_path: Some(PathBuf::from("/workspace/repository")),
            relative_path: Some(PathBuf::from("discovered")),
            allow_repository_update: true,
        }]);
        source_table.set_repository_updates(&[RepositoryUpdate {
            repo_path: PathBuf::from("/workspace/repository"),
            commits: vec![RepositoryCommit {
                id: "abc1234".to_string(),
                subject: "Add a skill".to_string(),
            }],
        }]);
        assert!(source_table.groups()[0].repository_update.is_some());
        let backend = TestBackend::new(140, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render_unified_inventory_table(frame, frame.area(), &rows, &source_table))
            .unwrap();

        let output = rendered_lines(&terminal).join("\n");
        assert!(
            output.contains("New version of repository available"),
            "{output}"
        );
        assert!(output.contains("press Cmd+U to update"), "{output}");
    }
}
