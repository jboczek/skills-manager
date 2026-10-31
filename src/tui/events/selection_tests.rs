use super::*;
use crate::{
    config::Config,
    plan::{ChangePlan, StagedChange},
    tui::{app::FixStep, source_table::SourceTableRow, unified_list::UnifiedListRow},
};
use std::{collections::BTreeSet, fs, os::unix::fs::symlink, path::PathBuf};

const SKILLS: [&str; 3] = ["alpha", "beta", "gamma"];
const AGENTS: [&str; 2] = ["claude", "codex"];

fn press(app: &mut App, code: KeyCode) {
    handle_key_with_table_height(app, KeyEvent::new(code, KeyModifiers::NONE), 10).unwrap();
}

fn submit(app: &mut App, input: &str) {
    for c in input.chars() {
        press(app, KeyCode::Char(c));
    }
    press(app, KeyCode::Enter);
}

fn focus(app: &mut App, name: &str) {
    while app.list_table.selected_index().is_some_and(|i| i > 0) {
        press(app, KeyCode::Up);
    }
    for _ in 0..30 {
        match app.list_table.selected_row().unwrap() {
            SourceTableRow::Item { item, .. } => {
                let matches = match &app.list_rows[item] {
                    UnifiedListRow::Exposed(row) => row.skill_id.name == name,
                    UnifiedListRow::Discovered(row) => row.skill_path.file_name().unwrap() == name,
                };
                if matches {
                    return;
                }
                press(app, KeyCode::Down);
            }
            SourceTableRow::Group { .. } => press(app, KeyCode::Right),
        }
    }
    panic!("skill {name} not found");
}

fn fixture(action: char) -> (tempfile::TempDir, App) {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    let mut config = Config::default_config();
    config.skills.central_dir = root.join("sources").display().to_string();
    config.shared_targets.clear();
    for (id, agent) in &mut config.agents {
        agent.enabled = AGENTS.contains(&id.as_str());
        agent.global_dir = root.join(id).display().to_string();
        agent.shared_target_ids.clear();
        fs::create_dir_all(root.join(id)).unwrap();
    }
    for name in SKILLS {
        let repo = root.join("sources").join(name);
        fs::create_dir_all(repo.join(".git")).unwrap();
        let source = repo.join("new").join(name);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "test skill").unwrap();
        for agent in AGENTS {
            if action == 'i' && agent == "claude" {
                continue;
            }
            let target = if action == 'f' {
                repo.join("old").join(name)
            } else {
                source.clone()
            };
            symlink(target, root.join(agent).join(name)).unwrap();
        }
    }
    let mut app = App::new(config).unwrap();
    app.initialize().unwrap();
    app.enter_list_mode();
    (temp, app)
}

fn plan(app: &App) -> &ChangePlan {
    match app.mode {
        Mode::Import => match &app.import_step {
            ImportStep::ConfirmPlan { plan, .. } => plan,
            step => panic!("expected import plan, got {step:?}"),
        },
        Mode::Remove => match &app.remove_step {
            RemoveStep::ConfirmPlan { plan, .. } => plan,
            step => panic!("expected remove plan, got {step:?}"),
        },
        Mode::Fix => match &app.fix_step {
            FixStep::Confirm { plan } => plan,
            step => panic!("expected fix plan, got {step:?}"),
        },
        ref mode => panic!("expected confirmation, got {mode:?}"),
    }
}

fn open_plan(app: &mut App, action: char) {
    press(app, KeyCode::Char(action));
    if matches!(app.import_step, ImportStep::SelectAgents { .. }) && app.mode == Mode::Import {
        submit(app, "");
    }
    if matches!(app.remove_step, RemoveStep::SelectExposure { .. }) && app.mode == Mode::Remove {
        submit(app, "all");
    }
}

fn links(root: &std::path::Path) -> Vec<Option<PathBuf>> {
    AGENTS
        .iter()
        .flat_map(|agent| SKILLS.map(|name| fs::read_link(root.join(agent).join(name)).ok()))
        .collect()
}

#[test]
fn list_actions_preview_and_apply_exact_checked_targets_or_cursor_without_checks() {
    for action in ['i', 'x', 'f'] {
        for count in 0..=2 {
            let (temp, mut app) = fixture(action);
            let root = fs::canonicalize(temp.path()).unwrap();
            for name in &SKILLS[..count] {
                focus(&mut app, name);
                press(&mut app, KeyCode::Char(' '));
            }
            focus(&mut app, "gamma");
            let before = links(&root);
            open_plan(&mut app, action);
            let names = if count == 0 {
                &SKILLS[2..]
            } else {
                &SKILLS[..count]
            };
            let agents = if action == 'i' {
                &AGENTS[..1]
            } else {
                &AGENTS[..]
            };
            let mut expected = BTreeSet::new();
            for agent in agents {
                for name in names {
                    expected.insert(root.join(agent).join(name));
                }
            }
            let changes = &plan(&app).changes;
            let actual: BTreeSet<_> = changes
                .iter()
                .map(|change| match change {
                    StagedChange::ExposeSkill {
                        target_path,
                        source_path,
                        ..
                    } if action == 'i' => {
                        assert!(source_path.join("SKILL.md").is_file());
                        target_path.clone()
                    }
                    StagedChange::DetachSkill { target_path, .. } if action == 'x' => {
                        target_path.clone()
                    }
                    StagedChange::FixBroken {
                        target_path,
                        replacement: Some(source),
                        ..
                    } if action == 'f' => {
                        assert!(source.join("SKILL.md").is_file());
                        target_path.clone()
                    }
                    other => panic!("unexpected change for {action}: {other:?}"),
                })
                .collect();
            assert_eq!(actual, expected, "action {action}, checked count {count}");
            assert_eq!(changes.len(), expected.len(), "duplicate physical targets");
            assert_eq!(links(&root), before, "preview must not mutate links");
            submit(&mut app, "y");
            assert!(app.error_message.is_none(), "{:?}", app.error_message);
            for (index, (agent, name)) in AGENTS
                .iter()
                .flat_map(|agent| SKILLS.map(|name| (agent, name)))
                .enumerate()
            {
                let target = root.join(agent).join(name);
                if !expected.contains(&target) {
                    assert_eq!(fs::read_link(&target).ok(), before[index]);
                } else if action == 'x' {
                    assert!(fs::symlink_metadata(&target).is_err());
                } else {
                    assert_eq!(
                        fs::canonicalize(&target).unwrap(),
                        root.join("sources").join(name).join("new").join(name)
                    );
                }
                assert!(
                    root.join("sources")
                        .join(name)
                        .join("new")
                        .join(name)
                        .join("SKILL.md")
                        .is_file()
                );
            }
        }
    }
}

#[test]
fn list_actions_cancel_batch_without_changing_any_links() {
    for action in ['i', 'x', 'f'] {
        for cancel in [KeyCode::Esc, KeyCode::Enter] {
            let (temp, mut app) = fixture(action);
            let root = fs::canonicalize(temp.path()).unwrap();
            for name in ["alpha", "beta"] {
                focus(&mut app, name);
                press(&mut app, KeyCode::Char(' '));
            }
            focus(&mut app, "gamma");
            let before = links(&root);
            open_plan(&mut app, action);
            assert!(!plan(&app).is_empty());
            press(&mut app, cancel);
            assert_ne!(app.mode, Mode::Import);
            assert_ne!(app.mode, Mode::Remove);
            assert_ne!(app.mode, Mode::Fix);
            assert_eq!(links(&root), before, "{action}, {cancel:?}");
        }
    }
}

#[test]
fn discovery_only_check_never_removes_unchecked_cursor_exposures() {
    let (temp, mut app) = fixture('x');
    let root = fs::canonicalize(temp.path()).unwrap();
    for agent in AGENTS {
        fs::remove_file(root.join(agent).join("alpha")).unwrap();
    }
    app.refresh_inventory().unwrap();
    app.enter_list_mode();
    focus(&mut app, "alpha");
    press(&mut app, KeyCode::Char(' '));
    focus(&mut app, "gamma");
    let before = links(&root);
    press(&mut app, KeyCode::Char('x'));
    assert!(!matches!(
        app.remove_step,
        RemoveStep::SelectExposure { .. } | RemoveStep::ConfirmPlan { .. }
    ));
    assert!(app.info_message.is_some());
    assert_eq!(links(&root), before);
}

#[test]
fn healthy_checks_never_import_or_fix_unchecked_cursor_skills() {
    for action in ['i', 'f'] {
        let (temp, mut app) = fixture(action);
        let root = fs::canonicalize(temp.path()).unwrap();
        let source = root.join("sources/alpha/new/alpha");
        for agent in AGENTS {
            let target = root.join(agent).join("alpha");
            if fs::symlink_metadata(&target).is_ok() {
                fs::remove_file(&target).unwrap();
            }
            symlink(&source, target).unwrap();
        }
        app.refresh_inventory().unwrap();
        app.enter_list_mode();
        focus(&mut app, "alpha");
        press(&mut app, KeyCode::Char(' '));
        focus(&mut app, "gamma");
        let before = links(&root);
        press(&mut app, KeyCode::Char(action));
        if action == 'i' && matches!(app.import_step, ImportStep::SelectAgents { .. }) {
            submit(&mut app, "");
        }
        assert!(!matches!(app.import_step, ImportStep::ConfirmPlan { .. }));
        assert!(!matches!(app.fix_step, FixStep::Confirm { .. }));
        assert!(app.info_message.is_some());
        assert_eq!(links(&root), before);
    }
}

#[test]
fn broken_check_without_source_never_imports_unchecked_discovery_cursor() {
    let (temp, mut app) = fixture('i');
    let root = fs::canonicalize(temp.path()).unwrap();
    fs::remove_dir_all(root.join("sources/alpha/new/alpha")).unwrap();
    fs::remove_file(root.join("codex/gamma")).unwrap();
    app.refresh_inventory().unwrap();
    app.enter_list_mode();
    focus(&mut app, "alpha");
    press(&mut app, KeyCode::Char(' '));
    focus(&mut app, "gamma");
    let before = links(&root);
    press(&mut app, KeyCode::Char('i'));
    assert_eq!(app.mode, Mode::List);
    assert!(app.info_message.as_ref().unwrap().contains("F"));
    assert_eq!(links(&root), before);
}

#[test]
fn project_local_checks_stay_read_only_even_with_a_global_cursor() {
    for action in ['i', 'x', 'f'] {
        let (temp, mut app) = fixture(action);
        let root = fs::canonicalize(temp.path()).unwrap();
        let local = root.join("sources/alpha/.agents/skills/local");
        fs::create_dir_all(&local).unwrap();
        fs::write(local.join("SKILL.md"), "local skill").unwrap();
        app.refresh_inventory().unwrap();
        app.enter_list_mode();
        focus(&mut app, "local");
        press(&mut app, KeyCode::Char(' '));
        focus(&mut app, "gamma");
        let before = links(&root);
        press(&mut app, KeyCode::Char(action));
        assert_eq!(app.mode, Mode::List, "action {action}");
        assert!(app.info_message.is_some());
        assert_eq!(
            fs::read_to_string(local.join("SKILL.md")).unwrap(),
            "local skill"
        );
        assert_eq!(links(&root), before);
    }
}
