use super::{App, FixStep, Mode};
use crate::{broken, domain::InventoryRow, scanner::ScanResult};

impl App {
    pub fn start_fix(&mut self) {
        self.error_message = None;
        self.fix_scroll = 0;
        let Some(row) = self
            .selected_inventory_row()
            .filter(InventoryRow::is_broken)
        else {
            self.info_message = Some("Select a broken global skill to fix.".into());
            return;
        };
        let candidates = broken::replacements(&row, &self.scan_results);
        self.mode = Mode::Fix;
        if candidates.len() > 1 {
            self.fix_step = FixStep::Choose {
                row: Box::new(row),
                candidates,
            };
        } else {
            self.prepare_fix(&row, candidates.first());
        }
    }

    fn prepare_fix(&mut self, row: &InventoryRow, replacement: Option<&ScanResult>) {
        self.fix_scroll = 0;
        self.fix_step = FixStep::Confirm {
            plan: broken::plan(row, replacement.map(|s| s.skill_path.clone())),
        };
    }

    pub(super) fn advance_fix(&mut self, input: &str) -> anyhow::Result<()> {
        match self.fix_step.clone() {
            FixStep::Choose { row, candidates } => {
                if let Some(index) = super::tables::parse_selection(input, candidates.len()) {
                    self.prepare_fix(&row, candidates.get(index));
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
            FixStep::Choose { row, candidates } => format!(
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
        app.list_table.move_right(30);
        app.list_table.move_right(30);
        app.start_import_from_selected_list_row().unwrap();
        app.advance_import("").unwrap();
        assert!(app.info_message.as_ref().unwrap().contains("Broken links"));
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
