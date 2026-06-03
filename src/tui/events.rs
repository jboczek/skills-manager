use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use crate::tui::app::{App, ImportStep, Mode, RemoveStep};

/// Handle a key event. Returns true if the app should quit.
pub fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    handle_key_with_table_height(app, key, current_table_height())
}

fn handle_key_with_table_height(
    app: &mut App,
    key: KeyEvent,
    table_height: usize,
) -> anyhow::Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    match key.code {
        KeyCode::Esc => {
            if app.command_menu_open() {
                app.input.clear();
                app.close_command_menu();
                return Ok(false);
            }
            app.input.clear();
            app.mode = Mode::Home;
            app.import_step = ImportStep::EnterSkill;
            app.remove_step = RemoveStep::EnterSkill;
            app.error_message = None;
            app.info_message = None;
        }
        KeyCode::Enter => {
            if app.command_menu_open() {
                let Some(command) = app.selected_command_suggestion() else {
                    app.error_message = Some("No matching command.".to_string());
                    app.input.clear();
                    app.close_command_menu();
                    return Ok(false);
                };
                let should_quit = app.handle_command(command.label)?;
                app.input.clear();
                return Ok(should_quit);
            }
            let input = app.input.clone();
            let should_quit = app.handle_command(&input)?;
            app.input.clear();
            return Ok(should_quit);
        }
        KeyCode::Backspace => {
            app.input.pop();
            if app.command_menu_open() {
                if app.input.starts_with('/') {
                    app.normalize_command_suggestion_selection();
                } else {
                    app.close_command_menu();
                }
            }
        }
        KeyCode::Up if app.command_menu_open() => {
            app.move_command_suggestion_up();
        }
        KeyCode::Down if app.command_menu_open() => {
            app.move_command_suggestion_down();
        }
        KeyCode::Up if app.mode == Mode::List => {
            app.list_table.move_up(app.inventory.len(), table_height);
            app.sync_legacy_list_navigation();
        }
        KeyCode::Down if app.mode == Mode::List => {
            app.list_table.move_down(app.inventory.len(), table_height);
            app.sync_legacy_list_navigation();
        }
        KeyCode::Char('?') if app.input.is_empty() => {
            app.mode = Mode::Help;
        }
        KeyCode::Char('q') if app.input.is_empty() => {
            return Ok(true);
        }
        KeyCode::Char('/') if app.input.is_empty() => {
            app.input.push('/');
            app.open_command_menu();
        }
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.input.push(c);
            if app.command_menu_open() {
                if app.input.starts_with('/') {
                    app.normalize_command_suggestion_selection();
                } else {
                    app.close_command_menu();
                }
            }
        }
        _ => {}
    }

    Ok(false)
}

fn current_table_height() -> usize {
    let Ok((width, height)) = crossterm::terminal::size() else {
        return 1;
    };
    let layout = crate::tui::layout::AppLayout::compute(Rect {
        x: 0,
        y: 0,
        width,
        height,
    });
    table_height_for_main(layout.main.height)
}

fn table_height_for_main(main_height: u16) -> usize {
    usize::from(main_height.saturating_sub(3)).max(1)
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::config::Config;
    use crate::domain::{
        AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure, SkillId, SkillSource,
    };

    fn test_app() -> App {
        App::new(
            Config::default_config(),
            std::path::PathBuf::from("/tmp/skills-manager"),
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn inventory_row(name: &str) -> InventoryRow {
        InventoryRow {
            skill_id: SkillId {
                namespace: "repo-a".to_string(),
                name: name.to_string(),
            },
            source: SkillSource {
                repo_name: None,
                repo_path: None,
                remote_url: None,
            },
            scope: Scope::Global,
            exposures: vec![SkillExposure {
                agent_id: AgentId("codex".to_string()),
                path: std::path::PathBuf::from(format!("/skills/{name}")),
                connection: ConnectionKind::Symlink,
            }],
            disambiguation_index: None,
        }
    }

    #[test]
    fn question_mark_opens_help_from_any_mode_when_prompt_is_empty() {
        let mut app = test_app();
        app.mode = Mode::List;

        handle_key(&mut app, key(KeyCode::Char('?'))).expect("key handled");

        assert_eq!(app.mode, Mode::Help);
    }

    #[test]
    fn q_quits_from_any_mode_when_prompt_is_empty() {
        let mut app = test_app();
        app.mode = Mode::List;

        let should_quit = handle_key(&mut app, key(KeyCode::Char('q'))).expect("key handled");

        assert!(should_quit);
    }

    #[test]
    fn q_is_text_when_prompt_has_input() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.input = "scan".to_string();

        let should_quit = handle_key(&mut app, key(KeyCode::Char('q'))).expect("key handled");

        assert!(!should_quit);
        assert_eq!(app.input, "scanq");
    }

    #[test]
    fn list_down_keeps_viewport_until_selection_reaches_bottom() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.inventory = vec![
            inventory_row("one"),
            inventory_row("two"),
            inventory_row("three"),
            inventory_row("four"),
        ];
        app.list_table.reset(app.inventory.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");

        assert_eq!(app.list_table.selected, Some(2));
        assert_eq!(app.list_table.viewport_offset, 0);
    }

    #[test]
    fn list_down_scrolls_after_selection_moves_past_bottom() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.inventory = vec![
            inventory_row("one"),
            inventory_row("two"),
            inventory_row("three"),
            inventory_row("four"),
        ];
        app.list_table.reset(app.inventory.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");

        assert_eq!(app.list_table.selected, Some(3));
        assert_eq!(app.list_table.viewport_offset, 1);
    }

    #[test]
    fn slash_opens_command_suggestions_when_prompt_is_empty() {
        let mut app = test_app();

        handle_key(&mut app, key(KeyCode::Char('/'))).expect("key handled");

        assert_eq!(app.input, "/");
        assert!(app.command_menu_open());
    }

    #[test]
    fn up_and_down_move_command_suggestion_selection() {
        let mut app = test_app();
        app.input = "/".to_string();
        app.open_command_menu();

        handle_key(&mut app, key(KeyCode::Down)).expect("key handled");

        assert_eq!(
            app.selected_command_suggestion().map(|item| item.label),
            Some("/scan")
        );

        handle_key(&mut app, key(KeyCode::Up)).expect("key handled");

        assert_eq!(
            app.selected_command_suggestion().map(|item| item.label),
            Some("/list")
        );
    }

    #[test]
    fn enter_runs_selected_command_suggestion() {
        let mut app = test_app();
        app.input = "/".to_string();
        app.open_command_menu();
        app.move_command_suggestion_down();

        let should_quit = handle_key(&mut app, key(KeyCode::Enter)).expect("key handled");

        assert!(!should_quit);
        assert_eq!(app.mode, Mode::Scan);
        assert_eq!(app.input, "");
        assert!(!app.command_menu_open());
    }

    #[test]
    fn escape_closes_command_suggestions_without_changing_mode() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.input = "/".to_string();
        app.open_command_menu();

        handle_key(&mut app, key(KeyCode::Esc)).expect("key handled");

        assert_eq!(app.mode, Mode::List);
        assert_eq!(app.input, "");
        assert!(!app.command_menu_open());
    }
}
