use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::config::Config;
use crate::domain::{ConnectionKind, InventoryRow};
use crate::tui::app::{
    App, ImportStep, Mode, RemoveStep, RepositoryUpdateStep, SourceAddStep, removable_exposures,
};
use crate::tui::components::{dialog, table};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    match app.mode {
        Mode::Home => {
            let last_operation = app
                .error_message
                .as_deref()
                .or(app.info_message.as_deref())
                .map(|message| format!("Last operation\n\n{message}\n\n"))
                .unwrap_or_default();
            render_text_panel(
                frame,
                area,
                " Home ",
                &format!(
                    "{last_operation}Welcome to Skills Manager.\n\nLoaded skills: {}\nEnabled agents: {}\n\nTry /list, /source_add, /config or /help. Use table shortcuts for import and remove actions.",
                    app.loaded_skills_label(),
                    app.config
                        .agents
                        .values()
                        .filter(|agent| agent.enabled)
                        .count()
                ),
            );
        }
        Mode::List => {
            if app.loading {
                render_text_panel(frame, area, " Inventory ", "Loading...");
            } else {
                table::render_unified_inventory_table(frame, area, &app.list_rows, &app.list_table);
            }
        }
        Mode::SourceAdd => render_source_add(frame, area, app),
        Mode::Config => render_config(frame, area, app),
        Mode::Help => render_help(frame, area),
        Mode::Import => render_import(frame, area, app),
        Mode::Remove => render_remove(frame, area, app),
        Mode::RepositoryUpdate => render_repository_update(frame, area, app),
        Mode::Quit => render_text_panel(frame, area, " Goodbye ", "Closing Skills Manager..."),
    }
}

fn render_config(frame: &mut Frame, area: Rect, app: &App) {
    let config_path = Config::default_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unavailable>".to_string());
    let body = match app.config.to_toml() {
        Ok(toml) => format!("Path: {config_path}\n\n{toml}"),
        Err(error) => format!("Path: {config_path}\n\nFailed to render config: {error}"),
    };
    render_text_panel(frame, area, " Config ", &body);
}

fn render_help(frame: &mut Frame, area: Rect) {
    render_text_panel(frame, area, " Help ", help_text());
}

fn help_text() -> &'static str {
    "Commands\n  /list                       Browse exposed and discovered skills\n  /source_add <clone-url>     Add a source from an HTTPS or SSH clone URL\n  /config                     Show config\n  /help                       Show this help\n  /quit                       Exit\n\nTable actions\n  Tab                Cycle Full, exposed, and discovery-only views\n  Space              Check or uncheck skill rows\n  i                  Import a selected skill or checked skills\n  x                  Remove checked or selected exposed skill rows\n  r                  Refresh the current list view\n  Cmd+U              Review and update the selected repository\n\nKeys\n  Enter              Submit prompt / open row details\n  Esc                Return home / cancel\n  Up / Down          Move visible table or command selection\n  Left / Right       Collapse or expand source groups\n  q                  Quit from home\n  ?                  Help from home"
}

fn render_source_add(frame: &mut Frame, area: Rect, app: &App) {
    match &app.source_add_step {
        SourceAddStep::Confirm { preview } => render_text_panel(
            frame,
            area,
            " Add Source ",
            &format!(
                "Source URL\n  {}\n\nDestination\n  {}",
                preview.url,
                preview.destination.display()
            ),
        ),
        SourceAddStep::SelectSkill {
            source_path,
            skills,
            outcome,
        } => {
            let action = match outcome {
                crate::source::AcquireOutcome::Cloned => "Added",
                crate::source::AcquireOutcome::Reused => "Reused",
            };
            let rows = skills
                .iter()
                .enumerate()
                .map(|(index, skill)| format!("{}. {}", index + 1, skill.skill_id))
                .collect::<Vec<_>>()
                .join("\n");
            render_text_panel(
                frame,
                area,
                " Add Source ",
                &format!(
                    "{action} source\n  {}\n\nDiscovered skills\n{rows}",
                    source_path.display()
                ),
            );
        }
        SourceAddStep::Done { message } => {
            render_text_panel(frame, area, " Add Source ", message);
        }
    }
}

fn render_import(frame: &mut Frame, area: Rect, app: &App) {
    match &app.import_step {
        ImportStep::Disambiguate { matches } => {
            let body = matches
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    format!(
                        "{}. {}  ({})",
                        index + 1,
                        item.skill_id,
                        item.skill_path.display()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            render_text_panel(frame, area, " Import ", &body);
        }
        ImportStep::SelectAgents {
            selected,
            agents,
            focused,
        } => {
            let agent_lines = agents
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let cursor = if i == *focused { "►" } else { " " };
                    let check = if item.checked { "✓" } else { " " };
                    format!(
                        "  {} [{}] {} ({})",
                        cursor, check, item.target.agent_id, item.target.display_name
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let selected_skills = selected
                .iter()
                .map(|skill| format!("  {} ({})", skill.skill_id, skill.skill_path.display()))
                .collect::<Vec<_>>()
                .join("\n");
            let body = format!(
                "Selected skills\n{}\n\nSelect agents to import to:\n{}",
                selected_skills, agent_lines,
            );
            render_text_panel(frame, area, " Import ", &body);
        }
        ImportStep::ConfirmPlan { plan, .. } => {
            render_text_panel(frame, area, " Import Plan ", &plan.render());
        }
        ImportStep::ConfirmPhysical { plan } => {
            render_text_panel(frame, area, " Import Plan ", &plan.render());
            dialog::render_confirm_dialog(
                frame,
                "Confirm permanent deletion",
                "This plan includes destructive file operations. Type 'yes' to continue.",
                true,
            );
        }
        ImportStep::Done { message } => {
            render_text_panel(frame, area, " Import ", message);
        }
    }
}

fn render_remove(frame: &mut Frame, area: Rect, app: &App) {
    match &app.remove_step {
        RemoveStep::SelectExposure { selected } => {
            let body = removable_exposure_lines(selected);
            render_text_panel(frame, area, " Remove ", &body);
        }
        RemoveStep::ConfirmPlan { plan, .. } => {
            render_text_panel(frame, area, " Remove Plan ", &plan.render());
        }
        RemoveStep::ConfirmPhysical { plan } => {
            render_text_panel(frame, area, " Remove Plan ", &plan.render());
            dialog::render_confirm_dialog(
                frame,
                "Confirm permanent deletion",
                "This will permanently delete physical copies. Type 'yes' to continue.",
                true,
            );
        }
        RemoveStep::Done { message } => {
            render_text_panel(frame, area, " Remove ", message);
        }
    }
}

fn render_repository_update(frame: &mut Frame, area: Rect, app: &App) {
    match &app.repository_update_step {
        RepositoryUpdateStep::Preview { update, scroll } => {
            let commits = update
                .commits
                .iter()
                .map(|commit| format!("  {}  {}", commit.id, commit.subject))
                .collect::<Vec<_>>()
                .join("\n");
            let body = format!(
                "Repository\n  {}\n\nMissing commits\n{}\n\nPull this repository? [y/N]",
                update.repo_path.display(),
                commits
            );
            render_text_panel_with_scroll(frame, area, " Repository Update ", &body, *scroll);
        }
        RepositoryUpdateStep::Done { message } => {
            render_text_panel(frame, area, " Repository Update ", message);
        }
    }
}

fn render_text_panel(frame: &mut Frame, area: Rect, title: &str, body: &str) {
    render_text_panel_with_scroll(frame, area, title, body, 0);
}

fn render_text_panel_with_scroll(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    body: &str,
    scroll: usize,
) {
    frame.render_widget(
        Paragraph::new(body)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Theme::border())
                    .title(title)
                    .title_style(Theme::header())
                    .style(Theme::default_style()),
            )
            .style(Theme::default_style())
            .scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn removable_exposure_lines(row: &InventoryRow) -> String {
    let mut lines = removable_exposures(row)
        .iter()
        .enumerate()
        .map(|(index, exposure)| {
            format!(
                "{}. {}  {}  {}",
                index + 1,
                exposure.agent_id.0,
                exposure.path.display(),
                connection_label(exposure.connection)
            )
        })
        .collect::<Vec<_>>();
    lines.push("all. Remove from all agents".to_string());
    lines.join("\n")
}

fn connection_label(connection: ConnectionKind) -> &'static str {
    match connection {
        ConnectionKind::Symlink => "symlink",
        ConnectionKind::PhysicalCopy => "copy",
        ConnectionKind::Missing => "missing",
        ConnectionKind::Unknown => "unknown",
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

    fn rendered_lines(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn help_points_to_table_actions_instead_of_standalone_mutation_commands() {
        let text = help_text();

        assert!(!text.contains("/import <skill>"));
        assert!(!text.contains("/remove <skill>"));
        assert!(!text.contains("/scan"));
        assert!(text.contains("Import a selected skill or checked skills"));
        assert!(text.contains("Remove checked or selected exposed skill rows"));
        assert!(text.contains("Check or uncheck skill rows"));
        assert!(text.contains("Cycle Full, exposed, and discovery-only views"));
        assert!(text.contains("Collapse or expand source groups"));
    }

    #[test]
    fn help_describes_source_add_clone_url_formats() {
        let text = help_text();

        assert!(text.contains("/source_add <clone-url>"));
        assert!(text.contains("HTTPS or SSH clone URL"));
    }

    #[test]
    fn remove_picker_offers_an_all_agents_choice() {
        let row = InventoryRow {
            skill_id: SkillId {
                namespace: "repo-a".to_string(),
                name: "skill".to_string(),
            },
            source: SkillSource {
                repo_name: None,
                repo_path: None,
                remote_url: None,
            },
            scope: crate::domain::Scope::Global,
            exposures: vec![
                SkillExposure {
                    agent_id: AgentId("claude".to_string()),
                    path: "/agents/claude/skill".into(),
                    connection: ConnectionKind::Symlink,
                },
                SkillExposure {
                    agent_id: AgentId("codex".to_string()),
                    path: "/agents/codex/skill".into(),
                    connection: ConnectionKind::Symlink,
                },
            ],
            disambiguation_index: None,
        };

        assert!(removable_exposure_lines(&row).contains("all. Remove from all agents"));
    }

    #[test]
    fn repository_update_preview_renders_missing_commit_subjects_and_confirmation() {
        let mut app = App::new(Config::default_config()).unwrap();
        app.mode = Mode::RepositoryUpdate;
        app.repository_update_step = RepositoryUpdateStep::Preview {
            update: RepositoryUpdate {
                repo_path: PathBuf::from("/workspace/skills"),
                commits: vec![RepositoryCommit {
                    id: "abc1234".to_string(),
                    subject: "Add a skill".to_string(),
                }],
            },
            scroll: 0,
        };
        let backend = TestBackend::new(100, 12);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render(frame, frame.area(), &app))
            .unwrap();

        let output = rendered_lines(&terminal);
        assert!(output.contains("Repository Update"), "{output}");
        assert!(output.contains("abc1234  Add a skill"), "{output}");
        assert!(output.contains("Pull this repository? [y/N]"), "{output}");
    }

    #[test]
    fn repository_update_preview_scroll_renders_later_commits() {
        let mut app = App::new(Config::default_config()).unwrap();
        app.mode = Mode::RepositoryUpdate;
        app.repository_update_step = RepositoryUpdateStep::Preview {
            update: RepositoryUpdate {
                repo_path: PathBuf::from("/workspace/skills"),
                commits: (0..8)
                    .map(|index| RepositoryCommit {
                        id: format!("commit{index}"),
                        subject: format!("Change {index}"),
                    })
                    .collect(),
            },
            scroll: 6,
        };
        let backend = TestBackend::new(100, 12);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render(frame, frame.area(), &app))
            .unwrap();

        let output = rendered_lines(&terminal);
        assert!(output.contains("commit7  Change 7"), "{output}");
    }
}
