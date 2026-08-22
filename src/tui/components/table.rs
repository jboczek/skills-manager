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
    let (start, end) = source_table.viewport_indices(visible_rows);

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
                repository_update: _,
                ..
            } = projected_row
            {
                let row = Row::new(vec![
                    Cell::from(group_cell_label(*expanded, name, context)),
                    Cell::from(skill_count_label(*count)),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
                .height(projected_row.rendered_height() as u16);
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

    let notice_x = inner
        .x
        .saturating_add(LIST_SKILL_COLUMN_WIDTH)
        .saturating_add(1)
        .saturating_add(24)
        .saturating_add(1);
    let notice_width = inner.x.saturating_add(inner.width).saturating_sub(notice_x);
    let mut row_y = inner.y.saturating_add(1);
    for projected_row in &projected_rows[start..end] {
        if matches!(
            projected_row,
            SourceTableRow::Group {
                repository_update: Some(_),
                ..
            }
        ) && notice_width > 0
        {
            frame.render_widget(
                Paragraph::new(repository_update_label()).style(Theme::success()),
                Rect {
                    x: notice_x,
                    y: row_y,
                    width: notice_width,
                    height: 1,
                },
            );
        }
        row_y = row_y.saturating_add(projected_row.rendered_height() as u16);
    }
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

fn group_cell_label(expanded: bool, name: &str, context: &str) -> String {
    group_label(expanded, name, context)
}

fn repository_update_label() -> &'static str {
    "New repository version available (Cmd+U to update)"
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
    fn repository_update_label_explains_the_cmd_u_action() {
        assert!(repository_update_label().contains("New repository version available"));
        assert!(repository_update_label().contains("Cmd+U to update"));
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
            output.contains("New repository version available"),
            "{output}"
        );
        assert!(output.contains("Cmd+U to update"), "{output}");

        let notice_line = rendered_lines(&terminal)
            .into_iter()
            .find(|line| line.contains("New repository version available"))
            .expect("update notice is rendered");
        assert!(notice_line.contains("> repository"), "{notice_line}");
        assert!(notice_line.contains("1 skill"), "{notice_line}");

        let buffer = terminal.backend().buffer();
        let notice_x = (0..buffer.area.width)
            .find(|&x| buffer[(x, 2)].symbol() == "N")
            .expect("notice begins on the group row");
        assert_eq!(buffer[(notice_x, 2)].fg, Theme::SUCCESS);
    }

    #[test]
    fn inventory_table_renders_the_selected_later_update_group_in_the_viewport() {
        let rows = vec![
            UnifiedListRow::Discovered(ScanResult {
                skill_id: "first/discovered".to_string(),
                skill_path: PathBuf::from("/workspace/first/discovered"),
                skill_relative_path: Some(PathBuf::from("discovered")),
                repo_name: Some("first".to_string()),
                repo_path: Some(PathBuf::from("/workspace/first")),
                remote_url: Some("https://example.com/first.git".to_string()),
                source_kind: SourceKind::CentralDir,
                disambiguation_index: None,
            }),
            UnifiedListRow::Discovered(ScanResult {
                skill_id: "second/discovered".to_string(),
                skill_path: PathBuf::from("/workspace/second/discovered"),
                skill_relative_path: Some(PathBuf::from("discovered")),
                repo_name: Some("second".to_string()),
                repo_path: Some(PathBuf::from("/workspace/second")),
                remote_url: Some("https://example.com/second.git".to_string()),
                source_kind: SourceKind::CentralDir,
                disambiguation_index: None,
            }),
        ];
        let mut source_table = SourceTable::new(vec![
            SourceGroupItem {
                item: 0,
                skill_name: "first".to_string(),
                skill_path: PathBuf::from("/workspace/first/discovered"),
                repo_name: Some("first".to_string()),
                repo_path: Some(PathBuf::from("/workspace/first")),
                relative_path: Some(PathBuf::from("discovered")),
                allow_repository_update: true,
            },
            SourceGroupItem {
                item: 1,
                skill_name: "second".to_string(),
                skill_path: PathBuf::from("/workspace/second/discovered"),
                repo_name: Some("second".to_string()),
                repo_path: Some(PathBuf::from("/workspace/second")),
                relative_path: Some(PathBuf::from("discovered")),
                allow_repository_update: true,
            },
        ]);
        source_table.set_repository_updates(&[
            RepositoryUpdate {
                repo_path: PathBuf::from("/workspace/first"),
                commits: vec![RepositoryCommit {
                    id: "first123".to_string(),
                    subject: "First update".to_string(),
                }],
            },
            RepositoryUpdate {
                repo_path: PathBuf::from("/workspace/second"),
                commits: vec![RepositoryCommit {
                    id: "second12".to_string(),
                    subject: "Second update".to_string(),
                }],
            },
        ]);
        source_table.move_down(3);
        let backend = TestBackend::new(140, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render_unified_inventory_table(frame, frame.area(), &rows, &source_table))
            .unwrap();

        let output = rendered_lines(&terminal).join("\n");
        assert!(output.contains("> second"), "{output}");
        assert!(
            output.contains("New repository version available"),
            "{output}"
        );
        assert!(output.contains("Cmd+U to update"), "{output}");
    }
}
