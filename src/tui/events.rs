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
        KeyCode::Up if app.mode == Mode::Scan => {
            app.scan_table.move_up(app.scan_results.len(), table_height);
        }
        KeyCode::Down if app.mode == Mode::Scan => {
            app.scan_table
                .move_down(app.scan_results.len(), table_height);
        }
        KeyCode::Char('?') if app.input.is_empty() => {
            app.mode = Mode::Help;
        }
        KeyCode::Char('q') if app.input.is_empty() => {
            return Ok(true);
        }
        KeyCode::Char('i') if app.input.is_empty() && app.mode == Mode::Scan => {
            app.start_import_from_selected_scan_row()?;
        }
        KeyCode::Char('i') if app.input.is_empty() && app.mode == Mode::List => {
            app.start_import_from_selected_list_row()?;
        }
        KeyCode::Char('x') if app.input.is_empty() && app.mode == Mode::List => {
            app.start_remove_from_selected_list_row()?;
        }
        KeyCode::Char('r')
            if app.input.is_empty() && matches!(app.mode, Mode::List | Mode::Scan) =>
        {
            app.refresh_active_table()?;
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
    use crate::scanner::{ScanResult, SourceKind};

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

    fn scan_result(skill_id: &str) -> ScanResult {
        ScanResult {
            skill_id: skill_id.to_string(),
            skill_path: std::path::PathBuf::from(format!("/skills/{skill_id}")),
            skill_relative_path: None,
            repo_name: None,
            repo_path: None,
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        }
    }

    fn enable_only(app: &mut App, agent_id: &str) {
        for (id, agent) in &mut app.config.agents {
            agent.enabled = id == agent_id;
        }
    }

    fn point_config_to_missing_paths(app: &mut App) {
        app.config.skills.central_dir = app
            .current_dir
            .join("missing-refresh-source")
            .to_string_lossy()
            .into_owned();
        app.config.skills.scan_parent_dirs.clear();
        for agent in app.config.agents.values_mut() {
            agent.global_dir = app
                .current_dir
                .join(format!("missing-{}", agent.display_name.to_lowercase()))
                .to_string_lossy()
                .into_owned();
            agent.project_dir = None;
            agent.shared_target_ids.clear();
        }
        app.config.shared_targets.clear();
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
    fn scan_down_uses_table_navigation() {
        let mut app = test_app();
        app.mode = Mode::Scan;
        app.scan_results = vec![
            scan_result("repo-a/one"),
            scan_result("repo-a/two"),
            scan_result("repo-a/three"),
            scan_result("repo-a/four"),
        ];
        app.scan_table.reset(app.scan_results.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");

        assert_eq!(app.scan_table.selected, Some(3));
        assert_eq!(app.scan_table.viewport_offset, 1);
    }

    #[test]
    fn i_in_scan_starts_import_for_selected_result() {
        let mut app = test_app();
        app.mode = Mode::Scan;
        app.scan_results = vec![scan_result("repo-a/one")];
        app.scan_table.reset(app.scan_results.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('i')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Import);
        assert!(matches!(app.import_step, ImportStep::SelectAgents { .. }));
    }

    #[test]
    fn i_in_list_starts_import_for_selected_row_missing_exposures() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.scan_results = vec![scan_result("repo-a/one")];
        app.list_table.reset(app.inventory.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('i')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Import);
        assert!(matches!(app.import_step, ImportStep::SelectAgents { .. }));
    }

    #[test]
    fn x_in_list_starts_remove_plan_for_single_exposure() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.list_table.reset(app.inventory.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Remove);
        assert!(matches!(app.remove_step, RemoveStep::ConfirmPlan { .. }));
    }

    #[test]
    fn x_in_list_prompts_for_multiple_exposures() {
        let mut app = test_app();
        let mut row = inventory_row("one");
        row.exposures.push(SkillExposure {
            agent_id: AgentId("claude".to_string()),
            path: std::path::PathBuf::from("/skills/one-claude"),
            connection: ConnectionKind::Symlink,
        });
        app.mode = Mode::List;
        app.inventory = vec![row];
        app.list_table.reset(app.inventory.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Remove);
        assert!(matches!(app.remove_step, RemoveStep::SelectExposure { .. }));
    }

    #[test]
    fn scan_import_shortcut_stages_plan_when_target_is_unambiguous() {
        let mut app = test_app();
        enable_only(&mut app, "codex");
        app.mode = Mode::Scan;
        app.scan_results = vec![scan_result("repo-a/one")];
        app.scan_table.reset(app.scan_results.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('i')), 3).expect("key handled");

        match app.import_step {
            ImportStep::ConfirmPlan { plan, .. } => assert!(!plan.is_empty()),
            _ => panic!("expected import plan preview"),
        }
    }

    #[test]
    fn list_import_shortcut_stages_plan_when_missing_target_is_unambiguous() {
        let mut app = test_app();
        enable_only(&mut app, "claude");
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.scan_results = vec![scan_result("repo-a/one")];
        app.list_table.reset(app.inventory.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('i')), 3).expect("key handled");

        match app.import_step {
            ImportStep::ConfirmPlan { plan, .. } => assert!(!plan.is_empty()),
            _ => panic!("expected import plan preview"),
        }
    }

    #[test]
    fn list_remove_shortcut_stages_plan_preview() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.list_table.reset(app.inventory.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");

        match app.remove_step {
            RemoveStep::ConfirmPlan { plan, .. } => assert!(!plan.is_empty()),
            _ => panic!("expected remove plan preview"),
        }
    }

    #[test]
    fn r_in_scan_refreshes_and_clears_stale_selection() {
        let mut app = test_app();
        point_config_to_missing_paths(&mut app);
        app.mode = Mode::Scan;
        app.scan_results = vec![scan_result("repo-a/one")];
        app.scan_table.reset(app.scan_results.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('r')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Scan);
        assert!(app.scan_results.is_empty());
        assert_eq!(app.scan_table.selected, None);
        assert_eq!(app.scan_table.viewport_offset, 0);
    }

    #[test]
    fn r_in_list_refreshes_and_clears_stale_selection() {
        let mut app = test_app();
        point_config_to_missing_paths(&mut app);
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.list_table.reset(app.inventory.len());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('r')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::List);
        assert!(app.inventory.is_empty());
        assert_eq!(app.list_table.selected, None);
        assert_eq!(app.list_table.viewport_offset, 0);
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
