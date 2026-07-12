use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use crate::tui::app::{App, ImportStep, Mode, RemoveStep, SourceAddStep};

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
            app.source_add_step = SourceAddStep::default();
            app.import_step = ImportStep::default();
            app.remove_step = RemoveStep::default();
            app.error_message = None;
            app.info_message = None;
        }
        KeyCode::Enter => {
            if app.command_menu_open() {
                let input = app.input.clone();
                let command = if has_command_arguments(&input) {
                    input
                } else if let Some(command) = app.selected_command_suggestion() {
                    command.label.to_string()
                } else {
                    input
                };
                let should_quit = app.handle_command(&command)?;
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
        KeyCode::Tab if app.command_menu_open() => {
            if let Some(command) = app.selected_command_suggestion() {
                app.input = completed_command_input(command.label);
                app.normalize_command_suggestion_selection();
            }
        }
        KeyCode::Tab if app.input.is_empty() && app.mode == Mode::List => {
            app.cycle_list_filter(table_height);
        }
        KeyCode::Up
            if app.mode == Mode::Import
                && matches!(app.import_step, ImportStep::SelectAgents { .. }) =>
        {
            app.move_agent_selection_up();
        }
        KeyCode::Down
            if app.mode == Mode::Import
                && matches!(app.import_step, ImportStep::SelectAgents { .. }) =>
        {
            app.move_agent_selection_down();
        }
        KeyCode::Char(' ')
            if app.mode == Mode::Import
                && matches!(app.import_step, ImportStep::SelectAgents { .. }) =>
        {
            app.toggle_agent_selection();
        }
        KeyCode::Char(' ') if app.input.is_empty() && app.mode == Mode::List => {
            if app.selected_inventory_row().is_some() {
                app.list_table.toggle_selected_check();
            }
        }
        KeyCode::Up if app.mode == Mode::List => {
            app.list_table.move_up(table_height);
        }
        KeyCode::Down if app.mode == Mode::List => {
            app.list_table.move_down(table_height);
        }
        KeyCode::Left if app.mode == Mode::List => {
            app.list_table.move_left(table_height);
        }
        KeyCode::Right if app.mode == Mode::List => {
            app.list_table.move_right(table_height);
        }
        KeyCode::Char('?') if app.input.is_empty() => {
            app.mode = Mode::Help;
        }
        KeyCode::Char('q') if app.input.is_empty() => {
            return Ok(true);
        }
        KeyCode::Char('i') if app.input.is_empty() && app.mode == Mode::List => {
            app.start_import_from_selected_list_row()?;
        }
        KeyCode::Char('x') if app.input.is_empty() && app.mode == Mode::List => {
            app.start_remove_from_selected_list_row()?;
        }
        KeyCode::Char('r') if app.input.is_empty() && app.mode == Mode::List => {
            app.refresh_active_table(table_height)?;
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

fn has_command_arguments(input: &str) -> bool {
    let normalized = input
        .trim_start()
        .strip_prefix('/')
        .unwrap_or(input.trim_start());
    normalized.split_whitespace().nth(1).is_some()
}

fn completed_command_input(label: &str) -> String {
    if label == "/source_add" {
        format!("{label} ")
    } else {
        label.to_string()
    }
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

pub(crate) fn table_height_for_main(main_height: u16) -> usize {
    usize::from(main_height.saturating_sub(3)).max(1)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tempfile::tempdir;

    use super::*;
    use crate::config::Config;
    use crate::constants::{AGENT_ID_CLAUDE, AGENT_ID_CODEX, AGENT_NAME_CLAUDE, AGENT_NAME_CODEX};
    use crate::domain::{
        AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure, SkillId, SkillSource,
    };
    use crate::scanner::{ScanResult, SourceKind};
    use crate::tui::source_table::SourceTableRow;
    use crate::tui::unified_list::ListFilter;

    fn test_app() -> App {
        App::new(Config::default_config()).expect("default config resolves")
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
                agent_id: AgentId(AGENT_ID_CODEX.to_string()),
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
        app.global_context = app.config.resolve_global_context().unwrap();
    }

    fn point_config_to_missing_paths(app: &mut App) {
        let root = std::path::Path::new("/tmp/skills-manager");
        app.config.skills.central_dir = root
            .join("missing-refresh-source")
            .to_string_lossy()
            .into_owned();
        app.config.skills.scan_parent_dirs.clear();
        for agent in app.config.agents.values_mut() {
            agent.global_dir = root
                .join(format!("missing-{}", agent.display_name.to_lowercase()))
                .to_string_lossy()
                .into_owned();
            agent.project_dir = None;
            agent.shared_target_ids.clear();
        }
        app.config.shared_targets.clear();
        app.global_context = app.config.resolve_global_context().unwrap();
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
        app.enter_list_mode();

        handle_key_with_table_height(&mut app, key(KeyCode::Right), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Right), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Down), 3).expect("key handled");

        assert_eq!(app.list_table.selected_index(), Some(3));
        assert_eq!(app.list_table.viewport_offset(), 1);
    }

    #[test]
    fn right_expands_list_group_and_left_returns_to_parent() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one"), inventory_row("two")];
        app.enter_list_mode();

        handle_key_with_table_height(&mut app, key(KeyCode::Right), 3).expect("key handled");
        handle_key_with_table_height(&mut app, key(KeyCode::Right), 3).expect("key handled");

        assert!(app.selected_inventory_row().is_some());

        handle_key_with_table_height(&mut app, key(KeyCode::Left), 3).expect("key handled");

        assert!(app.selected_inventory_row().is_none());
        assert_eq!(app.list_table.visible_rows().len(), 3);

        handle_key_with_table_height(&mut app, key(KeyCode::Left), 3).expect("key handled");

        assert_eq!(app.list_table.visible_rows().len(), 1);
    }

    #[test]
    fn group_rows_do_not_start_import_or_remove_actions() {
        let mut app = test_app();
        app.inventory = vec![inventory_row("one")];
        app.scan_results = vec![scan_result("repo-a/one")];

        app.enter_list_mode();
        handle_key_with_table_height(&mut app, key(KeyCode::Char('i')), 3).expect("key handled");
        assert_eq!(app.mode, Mode::List);
        assert_eq!(
            app.info_message.as_deref(),
            Some("Select a skill inside the group.")
        );

        app.info_message = None;
        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");
        assert_eq!(app.mode, Mode::List);
        assert_eq!(
            app.info_message.as_deref(),
            Some("Select a skill inside the group.")
        );
    }

    #[test]
    fn i_in_list_starts_import_for_selected_row_missing_exposures() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.scan_results = vec![scan_result("repo-a/one")];
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);

        handle_key_with_table_height(&mut app, key(KeyCode::Char('i')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Import);
        assert!(matches!(app.import_step, ImportStep::SelectAgents { .. }));
    }

    #[test]
    fn i_in_list_imports_a_selected_discovery_only_row() {
        let mut app = test_app();
        app.scan_results = vec![scan_result("repo-a/discovered")];
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);

        handle_key_with_table_height(&mut app, key(KeyCode::Char('i')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Import);
        assert!(matches!(app.import_step, ImportStep::SelectAgents { .. }));
    }

    #[test]
    fn space_in_list_toggles_checked_skill_rows() {
        let mut app = test_app();
        app.inventory = vec![inventory_row("one"), inventory_row("two")];
        app.enter_list_mode();

        handle_key_with_table_height(&mut app, key(KeyCode::Char(' ')), 3).expect("key handled");
        assert!(app.list_table.checked_items().is_empty());
        assert_eq!(app.input, "");

        app.list_table.move_right(3);
        app.list_table.move_right(3);
        handle_key_with_table_height(&mut app, key(KeyCode::Char(' ')), 3).expect("key handled");

        assert_eq!(app.list_table.checked_items(), vec![0]);
        assert_eq!(app.input, "");
    }

    #[test]
    fn discovery_only_rows_cannot_be_checked_or_removed() {
        let mut app = test_app();
        app.scan_results = vec![scan_result("repo-a/discovered")];
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);

        handle_key_with_table_height(&mut app, key(KeyCode::Char(' ')), 3).expect("key handled");
        assert!(app.list_table.checked_items().is_empty());

        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");
        assert_eq!(app.mode, Mode::List);
        assert!(matches!(app.remove_step, RemoveStep::Done { .. }));
    }

    #[test]
    fn i_in_list_uses_checked_rows_for_batch_import() {
        let mut app = test_app();
        point_config_to_missing_paths(&mut app);
        enable_only(&mut app, AGENT_ID_CLAUDE);
        app.inventory = vec![inventory_row("one"), inventory_row("two")];
        app.scan_results = vec![scan_result("repo-a/one"), scan_result("repo-a/two")];
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);
        handle_key_with_table_height(&mut app, key(KeyCode::Char(' ')), 3).expect("key handled");
        app.list_table.move_down(3);
        handle_key_with_table_height(&mut app, key(KeyCode::Char(' ')), 3).expect("key handled");

        handle_key_with_table_height(&mut app, key(KeyCode::Char('i')), 3).expect("key handled");

        match app.import_step {
            ImportStep::ConfirmPlan { plan, .. } => assert_eq!(plan.changes.len(), 2),
            _ => panic!("expected batch import plan preview"),
        }
    }

    #[test]
    fn x_in_list_starts_remove_plan_for_single_exposure() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);

        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Remove);
        assert!(matches!(app.remove_step, RemoveStep::ConfirmPlan { .. }));
    }

    #[test]
    fn x_in_list_uses_checked_rows_for_batch_remove() {
        let mut app = test_app();
        app.inventory = vec![inventory_row("one"), inventory_row("two")];
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);
        handle_key_with_table_height(&mut app, key(KeyCode::Char(' ')), 3).expect("key handled");
        app.list_table.move_down(3);
        handle_key_with_table_height(&mut app, key(KeyCode::Char(' ')), 3).expect("key handled");

        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");

        match app.remove_step {
            RemoveStep::ConfirmPlan { plan, .. } => assert_eq!(plan.changes.len(), 2),
            _ => panic!("expected batch remove plan preview"),
        }
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
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);

        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::Remove);
        assert!(matches!(app.remove_step, RemoveStep::SelectExposure { .. }));
    }

    #[test]
    fn list_import_shortcut_stages_plan_when_missing_target_is_unambiguous() {
        let mut app = test_app();
        enable_only(&mut app, AGENT_ID_CLAUDE);
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.scan_results = vec![scan_result("repo-a/one")];
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);

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
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);

        handle_key_with_table_height(&mut app, key(KeyCode::Char('x')), 3).expect("key handled");

        match app.remove_step {
            RemoveStep::ConfirmPlan { plan, .. } => assert!(!plan.is_empty()),
            _ => panic!("expected remove plan preview"),
        }
    }

    #[test]
    fn r_in_list_refreshes_and_clears_stale_selection() {
        let mut app = test_app();
        point_config_to_missing_paths(&mut app);
        app.mode = Mode::List;
        app.inventory = vec![inventory_row("one")];
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);

        handle_key_with_table_height(&mut app, key(KeyCode::Char('r')), 3).expect("key handled");

        assert_eq!(app.mode, Mode::List);
        assert!(app.inventory.is_empty());
        assert_eq!(app.list_table.selected_index(), None);
        assert_eq!(app.list_table.viewport_offset(), 0);
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
            Some("/source_add")
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
        app.move_command_suggestion_down();

        let should_quit = handle_key(&mut app, key(KeyCode::Enter)).expect("key handled");

        assert!(!should_quit);
        assert_eq!(app.mode, Mode::Config);
        assert_eq!(app.input, "");
        assert!(!app.command_menu_open());
    }

    #[test]
    fn tab_completes_selected_command_suggestion() {
        let mut app = test_app();
        app.input = "/sou".to_string();
        app.open_command_menu();

        handle_key(&mut app, key(KeyCode::Tab)).expect("key handled");

        assert_eq!(app.input, "/source_add ");
        assert!(app.command_menu_open());
    }

    #[test]
    fn tab_cycles_the_list_filter_only_when_the_prompt_is_empty() {
        let mut app = test_app();
        let exposed = scan_result("repo-a/exposed");
        let mut inventory = inventory_row("exposed");
        inventory.exposures[0].path = exposed.skill_path.clone();
        app.inventory = vec![inventory];
        app.scan_results = vec![exposed, scan_result("repo-a/discovered")];
        app.enter_list_mode();

        handle_key(&mut app, key(KeyCode::Tab)).expect("key handled");
        assert_eq!(app.list_filter, ListFilter::OnlyExposed);
        assert_eq!(app.list_rows.len(), 1);

        handle_key(&mut app, key(KeyCode::Tab)).expect("key handled");
        assert_eq!(app.list_filter, ListFilter::OnlyDiscovered);
        assert_eq!(app.list_rows.len(), 1);

        handle_key(&mut app, key(KeyCode::Tab)).expect("key handled");
        assert_eq!(app.list_filter, ListFilter::Full);
        assert_eq!(app.list_rows.len(), 2);
    }

    #[test]
    fn tab_completes_the_command_menu_instead_of_changing_the_list_filter() {
        let mut app = test_app();
        app.mode = Mode::List;
        app.input = "/sou".to_string();
        app.open_command_menu();

        handle_key(&mut app, key(KeyCode::Tab)).expect("key handled");

        assert_eq!(app.input, "/source_add ");
        assert_eq!(app.list_filter, ListFilter::Full);
    }

    #[test]
    fn refresh_preserves_the_active_list_filter() {
        let mut app = test_app();
        point_config_to_missing_paths(&mut app);
        app.mode = Mode::List;
        app.list_filter = ListFilter::OnlyDiscovered;

        handle_key_with_table_height(&mut app, key(KeyCode::Char('r')), 3).expect("key handled");

        assert_eq!(app.list_filter, ListFilter::OnlyDiscovered);
    }

    #[test]
    fn refresh_preserves_an_expanded_selected_list_skill() {
        let temp = tempdir().unwrap();
        let skill = temp.path().join("skills/discovered");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "# Discovered").unwrap();
        let mut config = Config::default_config();
        config.skills.central_dir = temp.path().join("skills").to_string_lossy().into_owned();
        config.skills.scan_parent_dirs.clear();
        for agent in config.agents.values_mut() {
            agent.global_dir = temp
                .path()
                .join(format!("{}-global", agent.display_name.to_lowercase()))
                .to_string_lossy()
                .into_owned();
            agent.project_dir = None;
            agent.shared_target_ids.clear();
        }
        config.shared_targets.clear();
        let mut app = App::new(config).unwrap();

        app.refresh_inventory().unwrap();
        app.enter_list_mode();
        app.list_table.move_right(3);
        app.list_table.move_right(3);
        assert!(matches!(
            app.list_table.selected_row(),
            Some(SourceTableRow::Item { .. })
        ));

        handle_key_with_table_height(&mut app, key(KeyCode::Char('r')), 3).expect("key handled");

        assert_eq!(app.list_filter, ListFilter::Full);
        assert!(matches!(
            app.list_table.selected_row(),
            Some(SourceTableRow::Item { .. })
        ));
        assert_eq!(app.list_table.visible_rows().len(), 2);
    }

    #[test]
    fn enter_submits_typed_command_with_arguments_when_menu_is_open() {
        let mut app = test_app();
        app.input = "/source_add https://example.com/org/skills.git".to_string();
        app.open_command_menu();

        handle_key(&mut app, key(KeyCode::Enter)).expect("key handled");

        assert_ne!(app.error_message.as_deref(), Some("No matching command."));
        assert_eq!(app.error_message, None);
        assert_eq!(app.mode, Mode::SourceAdd);
        assert!(matches!(app.source_add_step, SourceAddStep::Confirm { .. }));
        assert_eq!(app.input, "");
        assert!(!app.command_menu_open());
    }

    fn agent_selection_app(focused: usize) -> App {
        use crate::tui::app::AgentSelectionItem;

        let mut app = test_app();
        app.mode = Mode::Import;
        app.import_step = ImportStep::SelectAgents {
            selected: Box::new(scan_result("repo-a/skill")),
            agents: vec![
                AgentSelectionItem {
                    target: crate::inventory::AgentTarget {
                        agent_id: AGENT_ID_CLAUDE.to_string(),
                        display_name: AGENT_NAME_CLAUDE.to_string(),
                        global_dir: None,
                        shared_target_dirs: vec![],
                        enabled: true,
                    },
                    checked: true,
                },
                AgentSelectionItem {
                    target: crate::inventory::AgentTarget {
                        agent_id: AGENT_ID_CODEX.to_string(),
                        display_name: AGENT_NAME_CODEX.to_string(),
                        global_dir: None,
                        shared_target_dirs: vec![],
                        enabled: true,
                    },
                    checked: true,
                },
            ],
            focused,
        };
        app
    }

    #[test]
    fn up_moves_agent_selection_focus() {
        let mut app = agent_selection_app(1);

        handle_key(&mut app, key(KeyCode::Up)).expect("key handled");

        assert!(matches!(
            app.import_step,
            ImportStep::SelectAgents { focused: 0, .. }
        ));
    }

    #[test]
    fn down_moves_agent_selection_focus() {
        let mut app = agent_selection_app(0);

        handle_key(&mut app, key(KeyCode::Down)).expect("key handled");

        assert!(matches!(
            app.import_step,
            ImportStep::SelectAgents { focused: 1, .. }
        ));
    }

    #[test]
    fn space_toggles_agent_checked_state() {
        let mut app = agent_selection_app(0);

        handle_key(&mut app, key(KeyCode::Char(' '))).expect("key handled");

        if let ImportStep::SelectAgents { agents, .. } = &app.import_step {
            assert!(!agents[0].checked);
        } else {
            panic!("expected SelectAgents");
        }
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
