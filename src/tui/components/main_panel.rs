use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::config::Config;
use crate::output::render_inventory;
use crate::tui::app::{App, ImportStep, Mode, RemoveStep};
use crate::tui::components::{dialog, table};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    match app.mode {
        Mode::Home => render_text_panel(
            frame,
            area,
            " Home ",
            &format!(
                "Welcome to Skills Manager.\n\nLoaded skills: {}\nEnabled agents: {}\n\nTry /list, /scan, /config or /help. Use table shortcuts for import and remove actions.",
                app.inventory.len().max(app.scan_results.len()),
                app.config
                    .agents
                    .values()
                    .filter(|agent| agent.enabled)
                    .count()
            ),
        ),
        Mode::List => table::render_inventory_table(
            frame,
            area,
            &app.inventory,
            app.list_table.viewport_offset,
            app.list_table.selected,
        ),
        Mode::Scan => table::render_scan_table(frame, area, &app.scan_results, app.list_scroll),
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
    "Commands\n  /list              Show current inventory\n  /scan              Scan for available skills\n  /config            Show config\n  /help              Show this help\n  /quit              Exit\n\nTable actions\n  i                  Import selected scan row, or missing list exposures\n  x                  Remove selected list exposure\n  r                  Refresh current table\n\nKeys\n  Enter              Submit prompt / open row details\n  Esc                Return home / cancel\n  Up / Down          Move table or command selection\n  q                  Quit from home\n  ?                  Help from home"
}

fn render_import(frame: &mut Frame, area: Rect, app: &App) {
    match &app.import_step {
        ImportStep::EnterSkill => render_text_panel(
            frame,
            area,
            " Import ",
            "Import a skill\n\nType the skill identifier in the prompt below.",
        ),
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
        ImportStep::SelectAgents { selected } => {
            let agents = app
                .config
                .agents
                .iter()
                .filter(|(_, agent)| agent.enabled)
                .map(|(agent_id, agent)| format!("- {agent_id} ({})", agent.display_name))
                .collect::<Vec<_>>()
                .join("\n");
            let body = format!(
                "Selected skill\n  id: {}\n  path: {}\n  repo: {}\n  origin: {}\n\nSelect target agents:\n{}",
                selected.skill_id,
                selected.skill_path.display(),
                selected.repo_name.as_deref().unwrap_or("-"),
                selected.remote_url.as_deref().unwrap_or("-"),
                agents,
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
        RemoveStep::EnterSkill => render_text_panel(
            frame,
            area,
            " Remove ",
            "Remove a skill\n\nType the skill identifier in the prompt below.",
        ),
        RemoveStep::Disambiguate { matches } => {
            let body = matches
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    format!(
                        "{}. {}/{}",
                        index + 1,
                        row.skill_id.namespace,
                        row.skill_id.name
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_points_to_table_actions_instead_of_standalone_mutation_commands() {
        let text = help_text();

        assert!(!text.contains("/import <skill>"));
        assert!(!text.contains("/remove <skill>"));
        assert!(text.contains("Import selected scan row"));
        assert!(text.contains("Remove selected list exposure"));
    }
}
