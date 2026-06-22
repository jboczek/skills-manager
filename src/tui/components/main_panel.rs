use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::config::Config;
use crate::domain::{ConnectionKind, InventoryRow};
use crate::output::render_inventory;
use crate::tui::app::{App, ImportStep, Mode, RemoveStep, SourceAddStep, removable_exposures};
use crate::tui::components::{dialog, table};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    match app.mode {
        Mode::Home => render_text_panel(
            frame,
            area,
            " Home ",
            &format!(
                "Welcome to Skills Manager.\n\nLoaded skills: {}\nEnabled agents: {}\n\nTry /list, /scan, /source_add, /config or /help. Use table shortcuts for import and remove actions.",
                app.loaded_skills_label(),
                app.config
                    .agents
                    .values()
                    .filter(|agent| agent.enabled)
                    .count()
            ),
        ),
        Mode::List => {
            if app.loading {
                render_text_panel(frame, area, " Inventory ", "Loading...");
            } else {
                table::render_inventory_table(frame, area, &app.inventory, &app.list_table);
            }
        }
        Mode::Scan => {
            if app.loading {
                render_text_panel(frame, area, " Scan ", "Loading...");
            } else {
                table::render_scan_table(frame, area, &app.scan_results, &app.scan_table);
            }
        }
        Mode::SourceAdd => render_source_add(frame, area, app),
        Mode::Config => render_config(frame, area, app),
        Mode::Help => render_help(frame, area),
        Mode::Import => render_import(frame, area, app),
        Mode::Remove => render_remove(frame, area, app),
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
    "Commands\n  /list                       Show current inventory\n  /scan                       Scan for available skills\n  /source_add <clone-url>     Add a source from an HTTPS or SSH clone URL\n  /config                     Show config\n  /help                       Show this help\n  /quit                       Exit\n\nTable actions\n  i                  Import selected skill child\n  x                  Remove selected list skill exposure\n  r                  Refresh current table\n\nKeys\n  Enter              Submit prompt / open row details\n  Esc                Return home / cancel\n  Up / Down          Move visible table or command selection\n  Left / Right       Collapse or expand source groups\n  q                  Quit from home\n  ?                  Help from home"
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
            let body = format!(
                "Selected skill\n  id: {}\n  path: {}\n\nSelect agents to import to:\n{}",
                selected.skill_id,
                selected.skill_path.display(),
                agent_lines,
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
            let body = format!(
                "{message}\n\nCurrent inventory\n\n{}",
                render_inventory(&app.inventory)
            );
            render_text_panel(frame, area, " Import ", &body);
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
            let body = format!(
                "{message}\n\nCurrent inventory\n\n{}",
                render_inventory(&app.inventory)
            );
            render_text_panel(frame, area, " Remove ", &body);
        }
    }
}

fn render_text_panel(frame: &mut Frame, area: Rect, title: &str, body: &str) {
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
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn removable_exposure_lines(row: &InventoryRow) -> String {
    removable_exposures(row)
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
        .collect::<Vec<_>>()
        .join("\n")
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
    use super::*;

    #[test]
    fn help_points_to_table_actions_instead_of_standalone_mutation_commands() {
        let text = help_text();

        assert!(!text.contains("/import <skill>"));
        assert!(!text.contains("/remove <skill>"));
        assert!(text.contains("Import selected skill child"));
        assert!(text.contains("Remove selected list skill exposure"));
        assert!(text.contains("Collapse or expand source groups"));
    }

    #[test]
    fn help_describes_source_add_clone_url_formats() {
        let text = help_text();

        assert!(text.contains("/source_add <clone-url>"));
        assert!(text.contains("HTTPS or SSH clone URL"));
    }
}
