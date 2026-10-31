use super::{App, FixStep, Mode};
use crate::{broken, domain::InventoryRow};

impl App {
    pub fn start_fix(&mut self) {
        self.error_message = None;
        self.fix_scroll = 0;
        let rows = if self.list_table.checked_items().is_empty() {
            self.selected_inventory_row()
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            self.checked_inventory_rows()
        };
        let rows = rows
            .into_iter()
            .filter(InventoryRow::is_broken)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            self.info_message = Some("Select or check broken global skills to fix.".into());
            return;
        }
        self.info_message = None;
        self.mode = Mode::Fix;
        self.prepare_fixes(rows, crate::plan::ChangePlan::new(vec![]));
    }

    fn prepare_fixes(
        &mut self,
        mut remaining: Vec<InventoryRow>,
        mut plan: crate::plan::ChangePlan,
    ) {
        self.fix_scroll = 0;
        while !remaining.is_empty() {
            let row = remaining.remove(0);
            let candidates = broken::replacements(&row, &self.scan_results);
            if candidates.len() > 1 {
                self.fix_step = FixStep::Choose {
                    row: Box::new(row),
                    candidates,
                    remaining,
                    plan,
                };
                return;
            }
            plan.changes.extend(
                broken::plan(&row, candidates.first().map(|s| s.skill_path.clone())).changes,
            );
        }
        self.fix_step = FixStep::Confirm { plan };
    }

    pub(super) fn advance_fix(&mut self, input: &str) -> anyhow::Result<()> {
        match self.fix_step.clone() {
            FixStep::Choose {
                row,
                candidates,
                remaining,
                mut plan,
            } => {
                if let Some(index) = super::tables::parse_selection(input, candidates.len()) {
                    plan.changes.extend(
                        broken::plan(&row, Some(candidates[index].skill_path.clone())).changes,
                    );
                    self.prepare_fixes(remaining, plan);
                } else {
                    self.error_message =
                        Some("Choose a replacement number, or Esc to cancel.".into());
                }
            }
            FixStep::Confirm { plan } => {
                if input.eq_ignore_ascii_case("y") {
                    let message = self.apply_plan_and_refresh(&plan)?;
                    self.mode = Mode::List;
                    self.list_table =
                        crate::tui::source_table::SourceTable::new(self.unified_list_table_items());
                    self.info_message = Some(message);
                    self.fix_step = FixStep::Done;
                } else if input.is_empty() || input.eq_ignore_ascii_case("n") {
                    self.mode = Mode::List;
                    self.info_message = Some("Fix cancelled.".into());
                    self.fix_step = FixStep::Done;
                } else {
                    self.error_message = Some("Enter y or n.".into());
                }
            }
            FixStep::Done => self.mode = Mode::List,
        }
        Ok(())
    }

    pub fn fix_hint(&self) -> &'static str {
        match self.fix_step {
            FixStep::Choose { .. } => "Enter replacement number:",
            FixStep::Confirm { .. } => "Apply this plan? [y/N]:",
            FixStep::Done => "Press Enter to return.",
        }
    }

    pub fn fix_body(&self) -> String {
        match &self.fix_step {
            FixStep::Choose {
                row, candidates, ..
            } => format!(
                "Multiple replacements for {} in the original repository:\n\n{}",
                row.skill_id,
                candidates
                    .iter()
                    .enumerate()
                    .map(|(i, s)| format!("{}. {}", i + 1, s.skill_path.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            FixStep::Confirm { plan } => plan.render(),
            FixStep::Done => String::new(),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        tui::{events::handle_key, unified_list::ListFilter},
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::{fs, path::Path};

    fn fixture(root: &Path, replacements: &[&str]) -> App {
        let repo = root.join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        for path in replacements {
            let dir = repo.join(path);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("SKILL.md"),
                "---\nname: code\ndescription: test\n---\n",
            )
            .unwrap();
        }
        let mut config = Config::default_config();
        config.skills.central_dir = repo.display().to_string();
        config.shared_targets.clear();
        for (id, agent) in &mut config.agents {
            let target = root.join(id);
            fs::create_dir_all(&target).unwrap();
            std::os::unix::fs::symlink(repo.join("old/code"), target.join("code")).unwrap();
            agent.global_dir = target.display().to_string();
            agent.shared_target_ids.clear();
        }
        let mut app = App::new(config).unwrap();
        app.initialize().unwrap();
        app.enter_list_mode();
        app.list_filter = ListFilter::OnlyDiscovered;
        app.cycle_list_filter(30); // OnlyBroken
        app.list_table.move_right(30);
        app.list_table.move_right(30);
        app
    }

    fn batch_fixture(root: &Path, ambiguous: bool, removed: bool) -> App {
        let mut app = fixture(root, &["new/code"]);
        for skill in ["commit", "verify"] {
            if !removed || skill == "verify" {
                let source = root.join("repo/new").join(skill);
                fs::create_dir_all(&source).unwrap();
                fs::write(source.join("SKILL.md"), "test").unwrap();
            }
            for id in ["codex", "claude", "copilot"] {
                std::os::unix::fs::symlink(
                    root.join("repo/old").join(skill),
                    root.join(id).join(skill),
                )
                .unwrap();
            }
        }
        if ambiguous {
            for skill in ["code", "commit"] {
                let source = root.join("repo/other").join(skill);
                fs::create_dir_all(&source).unwrap();
                fs::write(source.join("SKILL.md"), "test").unwrap();
            }
        }
        app.refresh_inventory().unwrap();
        app.enter_list_mode();
        app.list_table.move_right(30);
        app.list_table.move_right(30);
        app.list_table.toggle_selected_check(); // code
        app.list_table.move_down(30);
        app.list_table.toggle_selected_check(); // commit
        app.list_table.move_down(30); // cursor on unchecked verify
        app
    }

    #[test]
    fn f_fixes_every_checked_skill_and_ignores_unchecked_cursor_skill() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = batch_fixture(temp.path(), false, false);
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
        )
        .unwrap();
        let FixStep::Confirm { plan } = &app.fix_step else {
            panic!("expected combined plan");
        };
        assert_eq!(plan.changes.len(), 6);
        assert!(app.fix_body().contains("old/code"));
        assert!(app.fix_body().contains("old/commit"));
        assert!(!app.fix_body().contains("old/verify"));
        app.advance_fix("y").unwrap();
        for id in ["codex", "claude", "copilot"] {
            for skill in ["code", "commit"] {
                assert!(temp.path().join(id).join(skill).join("SKILL.md").is_file());
            }
            assert!(!temp.path().join(id).join("verify").exists());
        }
    }

    #[test]
    fn batch_resolves_each_ambiguous_skill_before_any_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = batch_fixture(temp.path(), true, false);
        app.start_fix();
        let FixStep::Choose { row, .. } = &app.fix_step else {
            panic!("expected first choice");
        };
        assert_eq!(row.skill_id.name, "code");
        app.advance_fix("99").unwrap();
        assert!(matches!(app.fix_step, FixStep::Choose { .. }));
        app.advance_fix("1").unwrap();
        let FixStep::Choose { row, .. } = &app.fix_step else {
            panic!("expected second choice");
        };
        assert_eq!(row.skill_id.name, "commit");
        assert!(!temp.path().join("codex/code").exists());
        app.advance_fix("2").unwrap();
        let FixStep::Confirm { plan } = &app.fix_step else {
            panic!("expected combined plan");
        };
        assert_eq!(plan.changes.len(), 6);
        assert!(!temp.path().join("codex/commit").exists());
        app.advance_fix("y").unwrap();
        assert!(temp.path().join("codex/code/SKILL.md").is_file());
        assert!(temp.path().join("codex/commit/SKILL.md").is_file());
        assert!(!temp.path().join("codex/verify").exists());
    }

    #[test]
    fn mixed_repair_and_removal_batch_is_cancelled_or_applied_together() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = batch_fixture(temp.path(), false, true);
        app.start_fix();
        assert!(app.fix_body().contains("No replacement found"));
        assert!(app.fix_body().contains("new/code"));
        app.advance_fix("n").unwrap();
        for id in ["codex", "claude", "copilot"] {
            for skill in ["code", "commit", "verify"] {
                assert!(fs::read_link(temp.path().join(id).join(skill)).is_ok());
            }
        }
        app.start_fix();
        app.advance_fix("y").unwrap();
        for id in ["codex", "claude", "copilot"] {
            assert!(temp.path().join(id).join("code/SKILL.md").is_file());
            assert!(fs::symlink_metadata(temp.path().join(id).join("commit")).is_err());
            assert!(fs::read_link(temp.path().join(id).join("verify")).is_ok());
        }
    }

    #[test]
    fn healthy_checks_do_not_fall_back_to_broken_cursor_and_mixed_checks_skip_healthy() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = batch_fixture(temp.path(), false, false);
        for id in ["codex", "claude", "copilot"] {
            let link = temp.path().join(id).join("code");
            fs::remove_file(&link).unwrap();
            std::os::unix::fs::symlink(temp.path().join("repo/new/code"), link).unwrap();
        }
        app.refresh_inventory().unwrap();
        app.enter_list_mode();
        app.list_table.move_right(30);
        app.list_table.move_right(30);
        app.list_table.toggle_selected_check(); // healthy code
        app.list_table.move_down(30); // unchecked broken commit
        app.start_fix();
        assert_eq!(app.mode, Mode::List);
        assert!(!temp.path().join("codex/commit").exists());
        app.list_table.toggle_selected_check();
        app.list_table.move_down(30); // unchecked verify
        app.start_fix();
        let FixStep::Confirm { plan } = &app.fix_step else {
            panic!("expected plan");
        };
        assert_eq!(plan.changes.len(), 3);
        assert!(app.fix_body().contains("old/commit"));
        assert!(!app.fix_body().contains("old/verify"));
        assert!(!app.fix_body().contains("old/code"));
    }

    #[test]
    fn f_previews_all_agents_cancel_leaves_links_and_confirm_refreshes_inventory() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = fixture(temp.path(), &["new/code"]);
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('F'), KeyModifiers::NONE),
        )
        .unwrap();
        assert_eq!(app.mode, Mode::Fix);
        let FixStep::Confirm { plan } = &app.fix_step else {
            panic!("expected plan");
        };
        assert_eq!(plan.changes.len(), 3);
        assert!(app.fix_body().contains("old/code"));
        assert!(app.fix_body().contains("new/code"));
        app.advance_fix("n").unwrap();
        assert!(!temp.path().join("codex/code").exists());
        assert!(fs::read_link(temp.path().join("codex/code")).is_ok());
        app.start_fix();
        app.advance_fix("y").unwrap();
        assert_eq!(app.mode, Mode::List);
        assert!(app.inventory.iter().all(|r| !r.is_broken()));
        assert!(app.list_rows.is_empty()); // still filtered to OnlyBroken
        for id in ["codex", "claude", "copilot"] {
            assert!(temp.path().join(id).join("code/SKILL.md").is_file());
        }
    }

    #[test]
    fn ambiguous_replacement_requires_choice_and_no_match_offers_detach_only() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = fixture(temp.path(), &["a/code", "b/code"]);
        app.start_fix();
        assert!(matches!(app.fix_step, FixStep::Choose { .. }));
        app.advance_fix("9").unwrap();
        assert!(matches!(app.fix_step, FixStep::Choose { .. }));
        app.advance_fix("2").unwrap();
        assert!(matches!(app.fix_step, FixStep::Confirm { .. }));
        assert!(!temp.path().join("codex/code").exists());

        let temp = tempfile::tempdir().unwrap();
        let mut app = fixture(temp.path(), &["unrelated"]);
        app.start_fix();
        assert!(app.fix_body().contains("Remove symlink only"));
        app.advance_fix("y").unwrap();
        assert!(fs::symlink_metadata(temp.path().join("codex/code")).is_err());
        assert!(temp.path().join("repo/unrelated/SKILL.md").is_file());
    }

    #[test]
    fn moved_skill_is_one_red_inventory_row_with_current_source_and_original_repair_paths() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = fixture(temp.path(), &["new/code"]);
        app.enter_list_mode();
        assert_eq!(app.list_rows.len(), 1);
        let items = app.unified_list_table_items();
        assert_eq!(items[0].skill_path, temp.path().join("repo/new/code"));
        app.list_table.move_right(30);
        app.list_table.move_right(30);
        app.start_fix();
        assert!(app.fix_body().contains("old/code"));
        assert!(app.fix_body().contains("new/code"));
        app.advance_fix("y").unwrap();
        assert_eq!(app.list_rows.len(), 1);
        assert!(!app.inventory[0].is_broken());
    }

    #[test]
    fn import_explains_broken_collision_and_fix_plan_can_be_scrolled() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = fixture(temp.path(), &["new/code"]);
        app.start_import_from_selected_list_row().unwrap();
        assert!(app.info_message.as_ref().unwrap().contains("F"));
        app.start_fix();
        handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)).unwrap();
        assert_eq!(app.fix_scroll, 1);
        handle_key(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)).unwrap();
        assert_eq!(app.fix_scroll, 0);
        app.advance_fix("n").unwrap();
        app.list_filter = ListFilter::OnlyExposed;
        app.cycle_list_filter(30);
        assert!(app.list_rows.is_empty()); // Replacement is represented by the broken row.
    }

    #[test]
    fn removed_and_moved_reasons_and_partial_repairs_keep_one_row() {
        use crate::tui::unified_list::{UnifiedListRow, project_rows};
        let temp = tempfile::tempdir().unwrap();
        let mut app = fixture(temp.path(), &["new/code"]);
        let broken = app.inventory[0].clone();
        assert_eq!(
            crate::broken::connection_label(&broken, &app.scan_results),
            "broken (folder changed)"
        );
        fs::remove_file(temp.path().join("codex/code")).unwrap();
        std::os::unix::fs::symlink(
            temp.path().join("repo/new/code"),
            temp.path().join("codex/code"),
        )
        .unwrap();
        app.refresh_inventory().unwrap();
        let projected = project_rows(&app.inventory, &app.scan_results, ListFilter::Full);
        assert_eq!(projected.len(), 1);
        let UnifiedListRow::Exposed(row) = &projected[0] else {
            panic!("expected exposure");
        };
        assert!(row.is_broken());
        assert_eq!(row.exposures.len(), 3);
        assert_eq!(
            crate::broken::plan(row, Some(temp.path().join("repo/new/code")))
                .changes
                .len(),
            2
        );
        let mut other = app.scan_results[0].clone();
        other.repo_path = Some(temp.path().join("other-repo"));
        other.skill_path = temp.path().join("other-repo/code");
        fs::create_dir_all(&other.skill_path).unwrap();
        fs::write(other.skill_path.join("SKILL.md"), "test").unwrap();
        assert_eq!(
            project_rows(std::slice::from_ref(&broken), &[other], ListFilter::Full).len(),
            2
        );
        fs::remove_file(temp.path().join("repo/new/code/SKILL.md")).unwrap();
        assert_eq!(
            crate::broken::connection_label(&broken, &app.scan_results),
            "broken (removed)"
        );
        assert_eq!(project_rows(&[broken], &[], ListFilter::Full).len(), 1);
    }

    #[test]
    fn project_local_dangling_links_are_not_broken_and_other_repositories_are_not_candidates() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = fixture(temp.path(), &["new/code"]);
        let local = temp.path().join("repo/.agents/skills");
        fs::create_dir_all(&local).unwrap();
        std::os::unix::fs::symlink(temp.path().join("missing"), local.join("ghost")).unwrap();
        app.refresh_inventory().unwrap();
        assert!(!app.inventory.iter().any(|r| r.skill_id.name == "ghost"));
        let broken = app.inventory.iter().find(|r| r.is_broken()).unwrap();
        let mut other = app.scan_results[0].clone();
        other.repo_path = Some(temp.path().join("other-repo"));
        assert!(crate::broken::replacements(broken, &[other]).is_empty());
        let mut shared_row = broken.clone();
        shared_row.exposures.push(shared_row.exposures[0].clone());
        assert_eq!(crate::broken::plan(&shared_row, None).changes.len(), 3);
    }

    #[test]
    fn broken_rows_and_repository_warning_render_red_at_two_terminal_sizes() {
        use ratatui::{Terminal, backend::TestBackend, style::Color};
        let temp = tempfile::tempdir().unwrap();
        let app = fixture(temp.path(), &["new/code"]);
        for width in [120, 180] {
            let mut terminal = Terminal::new(TestBackend::new(width, 25)).unwrap();
            terminal
                .draw(|f| {
                    crate::tui::components::table::render_unified_inventory_table(
                        f,
                        f.area(),
                        &app.list_rows,
                        &app.list_table,
                        &app.scan_results,
                    )
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            let text = buffer
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(text.contains("!"));
            assert!(text.contains("code"));
            if width == 180 {
                assert!(text.contains("broken (folder changed)"));
            }
            assert!(
                buffer
                    .content
                    .iter()
                    .any(|c| c.symbol() == "!" && c.fg == Color::Red)
            );
            let row_y = (0..25)
                .find(|&y| (0..width).any(|x| buffer[(x, y)].symbol() == "["))
                .unwrap();
            for x in 1..width - 1 {
                assert_eq!(buffer[(x, row_y)].fg, Color::Red);
            }
        }
    }
}
