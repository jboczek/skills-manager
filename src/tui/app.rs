use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::commands::helpers;
use crate::config::{Config, GlobalContext};
use crate::domain::InventoryRow;
use crate::git::{self, RepositoryUpdate};
use crate::inventory::{self, InventoryConfig};
use crate::scanner::{self, ScanResult};
use crate::source;
use crate::tui::source_table::SourceTable;
use crate::tui::unified_list::{ListFilter, UnifiedListRow, project_rows};

mod command;
mod state;
mod tables;
mod workflows;

pub use command::{CommandSuggestion, TuiCommand, parse_command};
pub use state::{
    AgentSelectionItem, ImportStep, Mode, PendingLoad, RemoveStep, RepositoryUpdateStep,
    SourceAddStep,
};
pub(crate) use workflows::removable_exposures;

use command::COMMAND_SUGGESTIONS;

pub struct App {
    pub mode: Mode,
    pub input: String,
    pub inventory: Vec<InventoryRow>,
    pub list_rows: Vec<UnifiedListRow>,
    pub list_filter: ListFilter,
    pub scan_results: Vec<ScanResult>,
    pub repository_updates: Vec<RepositoryUpdate>,
    pub status_messages: Vec<String>,
    pub list_table: SourceTable,
    pub config: Config,
    pub global_context: GlobalContext,
    pub prompt_label: String,
    pub source_add_step: SourceAddStep,
    pub import_step: ImportStep,
    pub remove_step: RemoveStep,
    pub repository_update_step: RepositoryUpdateStep,
    pub error_message: Option<String>,
    pub info_message: Option<String>,
    pub loading: bool,
    pub pending_load: Option<PendingLoad>,
    initial_loading: bool,
    initial_load: Option<Receiver<anyhow::Result<InitialLoad>>>,
    repository_update_load: Option<Receiver<Vec<RepositoryUpdate>>>,
    command_menu_selected: Option<usize>,
}

struct InitialLoad {
    scan_results: Vec<ScanResult>,
    inventory: Vec<InventoryRow>,
}

impl App {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let global_context = config.resolve_global_context()?;
        Ok(Self {
            mode: Mode::Home,
            input: String::new(),
            inventory: Vec::new(),
            list_rows: Vec::new(),
            list_filter: ListFilter::Full,
            scan_results: Vec::new(),
            repository_updates: Vec::new(),
            status_messages: Vec::new(),
            list_table: SourceTable::default(),
            config,
            global_context,
            prompt_label: "Skills".to_string(),
            source_add_step: SourceAddStep::default(),
            import_step: ImportStep::default(),
            remove_step: RemoveStep::default(),
            repository_update_step: RepositoryUpdateStep::default(),
            error_message: None,
            info_message: None,
            loading: false,
            pending_load: None,
            initial_loading: false,
            initial_load: None,
            repository_update_load: None,
            command_menu_selected: None,
        })
    }

    /// Load initial global scan and inventory state.
    pub fn initialize(&mut self) -> anyhow::Result<()> {
        let load = load_initial_state(self.global_context.clone())?;
        self.apply_initial_load(load);
        Ok(())
    }

    pub fn start_initial_load(&mut self) {
        if self.initial_loading {
            return;
        }

        self.initial_loading = true;
        self.rebuild_status_messages();

        let context = self.global_context.clone();
        let (sender, receiver) = mpsc::channel();
        self.initial_load = Some(receiver);
        thread::spawn(move || {
            let _ = sender.send(load_initial_state(context));
        });
    }

    pub fn poll_initial_load(&mut self) {
        let result = match self.initial_load.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => result,
            Some(Err(TryRecvError::Empty)) | None => return,
            Some(Err(TryRecvError::Disconnected)) => {
                Err(anyhow::anyhow!("initial skill scan stopped unexpectedly"))
            }
        };

        self.initial_load = None;
        match result {
            Ok(load) => self.apply_initial_load(load),
            Err(error) => {
                self.initial_loading = false;
                self.error_message = Some(error.to_string());
                self.rebuild_status_messages();
            }
        }
    }

    pub fn poll_repository_update_load(&mut self) {
        let Some(receiver) = self.repository_update_load.as_ref() else {
            return;
        };
        let Ok(updates) = receiver.try_recv() else {
            return;
        };

        self.repository_update_load = None;
        self.repository_updates = updates;
        if self.mode == Mode::List {
            self.list_table
                .set_repository_updates(&self.repository_updates);
        }
    }

    pub fn initial_load_in_progress(&self) -> bool {
        self.initial_loading
    }

    pub fn loaded_skills_label(&self) -> String {
        if self.initial_loading {
            "(loading)".to_string()
        } else {
            self.loaded_skill_count().to_string()
        }
    }

    /// Refresh inventory from filesystem.
    pub fn refresh_inventory(&mut self) -> anyhow::Result<()> {
        let raw_scan_results =
            scanner::scan(&helpers::scan_config_from_global(&self.global_context))?;
        self.scan_results = raw_scan_results.clone();
        scanner::exclude_dot_directory_results(&mut self.scan_results);
        scanner::assign_disambiguation_indices(&mut self.scan_results);
        self.repository_updates.clear();
        self.start_repository_update_load();
        self.inventory = inventory::build_inventory(&InventoryConfig {
            agents: helpers::agent_targets_from_global(&self.global_context),
            scan_results: raw_scan_results,
        });
        inventory::assign_disambiguation_indices(&mut self.inventory);
        self.rebuild_list_rows();
        self.rebuild_status_messages();
        Ok(())
    }

    /// Execute a deferred list load after a loading frame renders.
    pub fn execute_pending_load(&mut self) -> anyhow::Result<()> {
        let Some(load) = self.pending_load.take() else {
            return Ok(());
        };
        self.loading = false;
        match load {
            PendingLoad::List => {
                self.refresh_inventory()?;
                self.enter_list_mode();
                self.info_message = Some(self.list_summary_message());
            }
        }
        Ok(())
    }

    /// Handle a submitted command string from the prompt.
    /// Returns true if the app should quit.
    pub fn handle_command(&mut self, input: &str) -> anyhow::Result<bool> {
        self.error_message = None;
        self.info_message = None;
        self.close_command_menu();

        match self.mode {
            Mode::SourceAdd => {
                self.advance_source_add(input)?;
                return Ok(false);
            }
            Mode::Import => {
                self.advance_import(input)?;
                return Ok(false);
            }
            Mode::Remove => {
                self.advance_remove(input)?;
                return Ok(false);
            }
            Mode::RepositoryUpdate => {
                self.advance_repository_update(input)?;
                return Ok(false);
            }
            _ => {}
        }

        match parse_command(input) {
            TuiCommand::List => {
                self.mode = Mode::List;
                self.loading = true;
                self.pending_load = Some(PendingLoad::List);
            }
            TuiCommand::SourceAdd(url) => {
                if url.is_empty() {
                    self.error_message =
                        Some("Usage: /source_add <clone-url> (HTTPS or SSH clone URL)".to_string());
                } else {
                    match source::preview(&url, &self.global_context.central_dir) {
                        Ok(preview) => {
                            self.mode = Mode::SourceAdd;
                            self.source_add_step = SourceAddStep::Confirm { preview };
                        }
                        Err(error) => self.error_message = Some(error.to_string()),
                    }
                }
            }
            TuiCommand::Import => {
                self.import_step = ImportStep::default();
                self.info_message = Some(
                    "Use table shortcuts: run /list, select a discovery row and press i, or press Space then i to import checked skills to selected agents."
                        .to_string(),
                );
            }
            TuiCommand::Remove => {
                self.remove_step = RemoveStep::default();
                self.info_message = Some(
                    "Use table shortcuts: run /list, press Space to check rows, then press x to remove exposed rows."
                        .to_string(),
                );
            }
            TuiCommand::Config => {
                self.mode = Mode::Config;
            }
            TuiCommand::Help => {
                self.mode = Mode::Help;
            }
            TuiCommand::Quit => {
                self.mode = Mode::Quit;
                return Ok(true);
            }
            TuiCommand::Unknown(command) => {
                self.error_message = Some(format!(
                    "Unknown command: '{}'. Type /help for available commands.",
                    command
                ));
            }
        }

        Ok(false)
    }

    pub fn command_menu_open(&self) -> bool {
        self.command_menu_selected.is_some()
    }

    pub fn open_command_menu(&mut self) {
        self.command_menu_selected = Some(0);
        self.normalize_command_suggestion_selection();
    }

    pub fn close_command_menu(&mut self) {
        self.command_menu_selected = None;
    }

    pub fn filtered_command_suggestions(&self) -> Vec<CommandSuggestion> {
        let query = self
            .input
            .trim_start()
            .strip_prefix('/')
            .unwrap_or("")
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        COMMAND_SUGGESTIONS
            .iter()
            .copied()
            .filter(|suggestion| {
                query.is_empty()
                    || suggestion
                        .label
                        .trim_start_matches('/')
                        .starts_with(query.as_str())
            })
            .collect()
    }

    pub fn selected_command_suggestion(&self) -> Option<CommandSuggestion> {
        let selected = self.command_menu_selected?;
        self.filtered_command_suggestions().get(selected).copied()
    }

    pub fn move_command_suggestion_up(&mut self) {
        let Some(selected) = self.command_menu_selected else {
            return;
        };
        self.command_menu_selected = Some(selected.saturating_sub(1));
    }

    pub fn move_command_suggestion_down(&mut self) {
        let Some(selected) = self.command_menu_selected else {
            return;
        };
        let max = self.filtered_command_suggestions().len().saturating_sub(1);
        self.command_menu_selected = Some(selected.saturating_add(1).min(max));
    }

    pub fn normalize_command_suggestion_selection(&mut self) {
        let Some(selected) = self.command_menu_selected else {
            return;
        };
        let max = self.filtered_command_suggestions().len().saturating_sub(1);
        self.command_menu_selected = Some(selected.min(max));
    }

    pub fn enter_list_mode(&mut self) {
        self.mode = Mode::List;
        self.list_filter = ListFilter::Full;
        self.rebuild_list_rows();
        self.list_table = SourceTable::new(self.unified_list_table_items());
        self.list_table
            .set_repository_updates(&self.repository_updates);
    }

    pub fn cycle_list_filter(&mut self, viewport_height: usize) {
        self.list_filter = self.list_filter.next();
        self.rebuild_list_rows();
        self.list_table
            .refresh(self.unified_list_table_items(), viewport_height);
        self.list_table
            .set_repository_updates(&self.repository_updates);
    }

    pub fn refresh_active_table(&mut self, viewport_height: usize) -> anyhow::Result<()> {
        self.error_message = None;
        self.info_message = None;

        if self.mode == Mode::List {
            self.refresh_inventory()?;
            let items = self.unified_list_table_items();
            self.list_table.refresh(items, viewport_height);
            self.list_table
                .set_repository_updates(&self.repository_updates);
            self.info_message = Some(self.list_summary_message());
        }

        Ok(())
    }

    pub fn sync_active_table(&mut self, viewport_height: usize) {
        if self.mode == Mode::List {
            self.list_table.sync(viewport_height);
        }
    }

    fn reload_scan_results(&mut self) -> anyhow::Result<()> {
        self.scan_results = scanner::scan(&helpers::scan_config_from_global(&self.global_context))?;
        scanner::exclude_dot_directory_results(&mut self.scan_results);
        scanner::assign_disambiguation_indices(&mut self.scan_results);
        self.rebuild_status_messages();
        Ok(())
    }

    fn rebuild_list_rows(&mut self) {
        self.list_rows = project_rows(&self.inventory, &self.scan_results, self.list_filter);
    }

    fn list_summary_message(&self) -> String {
        let discovered = project_rows(
            &self.inventory,
            &self.scan_results,
            ListFilter::OnlyDiscovered,
        )
        .len();
        format!(
            "Imported: {} · Discovered not imported: {discovered}",
            self.inventory.len()
        )
    }

    fn rebuild_status_messages(&mut self) {
        let scan_status = if self.initial_loading {
            "loading"
        } else {
            "OK"
        };
        self.status_messages = vec![
            format!(
                "• Global context: {} skills, {} agents",
                self.loaded_skills_label(),
                self.global_context
                    .agents
                    .iter()
                    .filter(|agent| agent.enabled)
                    .count()
            ),
            format!("• Scan: {scan_status}"),
        ];
        self.status_messages.extend(
            self.global_context
                .diagnostics
                .iter()
                .map(|diagnostic| format!("• Warning: {diagnostic}")),
        );
    }

    fn apply_initial_load(&mut self, load: InitialLoad) {
        self.initial_loading = false;
        self.initial_load = None;
        self.scan_results = load.scan_results;
        self.inventory = load.inventory;
        self.repository_updates.clear();
        self.start_repository_update_load();
        self.rebuild_list_rows();
        self.rebuild_status_messages();
    }

    fn start_repository_update_load(&mut self) {
        self.repository_update_load = Some(repository_updates(&self.scan_results));
    }

    fn loaded_skill_count(&self) -> usize {
        self.inventory.len().max(self.scan_results.len())
    }
}

fn load_initial_state(context: GlobalContext) -> anyhow::Result<InitialLoad> {
    let raw_scan_results = scanner::scan(&helpers::scan_config_from_global(&context))?;
    let mut scan_results = raw_scan_results.clone();
    scanner::exclude_dot_directory_results(&mut scan_results);
    scanner::assign_disambiguation_indices(&mut scan_results);

    let mut inventory = inventory::build_inventory(&InventoryConfig {
        agents: helpers::agent_targets_from_global(&context),
        scan_results: raw_scan_results,
    });
    inventory::assign_disambiguation_indices(&mut inventory);

    Ok(InitialLoad {
        scan_results,
        inventory,
    })
}

fn repository_updates(scan_results: &[ScanResult]) -> Receiver<Vec<RepositoryUpdate>> {
    let mut repositories = Vec::new();
    for result in scan_results {
        let Some(repo_path) = result.repo_path.as_ref() else {
            continue;
        };
        if result.remote_url.is_none() || repositories.iter().any(|path| path == repo_path) {
            continue;
        }
        repositories.push(repo_path.clone());
    }

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let updates = repositories
            .into_iter()
            .filter_map(|repo_path| git::repository_update(&repo_path).ok().flatten())
            .collect();
        let _ = sender.send(updates);
    });
    receiver
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use tempfile::tempdir;

    use super::*;
    use crate::domain::{
        AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure, SkillId, SkillSource,
    };
    use crate::plan::{ChangePlan, StagedChange};
    use crate::scanner::SourceKind;
    use crate::tui::source_table::SourceTableRow;
    use crate::tui::unified_list::{ListFilter, UnifiedListRow};

    fn test_config(root: &std::path::Path) -> Config {
        let mut config = Config::default_config();
        config.skills.central_dir = root.join("skills").to_string_lossy().into_owned();
        config.skills.scan_parent_dirs.clear();
        for agent in config.agents.values_mut() {
            agent.global_dir = root
                .join(format!("{}-global", agent.display_name.to_lowercase()))
                .to_string_lossy()
                .into_owned();
            agent.project_dir = None;
            agent.shared_target_ids.clear();
        }
        config.shared_targets.clear();
        config
    }

    fn test_app() -> App {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().to_path_buf();
        std::mem::forget(temp);
        App::new(test_config(&path)).expect("test config resolves")
    }

    #[test]
    fn initialization_uses_global_prompt_and_omits_legacy_diagnostics() {
        let temp = tempdir().expect("tempdir");
        let mut config = test_config(temp.path());
        config.agents.get_mut("claude").unwrap().project_dir = Some(".claude/skills".to_string());
        let launch_dir = temp.path().join("repo-on-feature-branch");
        let mut app = App::new(config).expect("legacy config resolves");

        app.initialize().expect("initialization succeeds");

        assert_eq!(app.prompt_label, "Skills");
        assert!(
            app.status_messages
                .iter()
                .all(|message| !message.contains("agents.claude.project_dir"))
        );
        assert!(
            app.status_messages
                .iter()
                .all(|message| !message.contains(&launch_dir.display().to_string()))
        );
    }

    #[test]
    fn start_initial_load_shows_loading_skill_stats() {
        let mut app = test_app();

        app.start_initial_load();

        assert!(app.initial_load_in_progress());
        assert_eq!(app.loaded_skills_label(), "(loading)");
        assert!(
            app.status_messages
                .iter()
                .any(|message| message.contains("(loading) skills"))
        );
        assert!(
            app.status_messages
                .iter()
                .any(|message| message.contains("Scan: loading"))
        );
    }

    #[test]
    fn apply_initial_load_replaces_loading_stats_with_loaded_data() {
        let mut app = test_app();
        app.start_initial_load();

        app.apply_initial_load(InitialLoad {
            scan_results: vec![scan_result("repo-a/one")],
            inventory: vec![inventory_row("repo-a/one")],
        });

        assert!(!app.initial_load_in_progress());
        assert_eq!(app.loaded_skills_label(), "1");
        assert_eq!(app.scan_results.len(), 1);
        assert_eq!(app.inventory.len(), 1);
        assert!(
            app.status_messages
                .iter()
                .any(|message| message.contains("1 skills"))
        );
        assert!(
            app.status_messages
                .iter()
                .any(|message| message.contains("Scan: OK"))
        );
    }

    #[test]
    fn repository_update_results_are_polled_without_blocking_the_list() {
        let mut app = test_app();
        let (sender, receiver) = mpsc::channel();
        app.repository_update_load = Some(receiver);

        let started = Instant::now();
        app.poll_repository_update_load();

        assert!(started.elapsed() < Duration::from_millis(100));
        assert!(app.repository_update_load.is_some());
        assert!(app.repository_updates.is_empty());

        sender
            .send(vec![RepositoryUpdate {
                repo_path: PathBuf::from("/skills/repository"),
                commits: vec![crate::git::RepositoryCommit {
                    id: "abc1234".to_string(),
                    subject: "Update".to_string(),
                }],
            }])
            .unwrap();
        app.poll_repository_update_load();

        assert!(app.repository_update_load.is_none());
        assert_eq!(app.repository_updates.len(), 1);
    }

    #[test]
    fn entering_list_mode_projects_full_rows_without_repeating_exposed_sources() {
        let mut app = test_app();
        let exposed = scan_result("repo-a/exposed");
        let mut inventory = inventory_row("repo-a/exposed");
        inventory.exposures[0].path = exposed.skill_path.clone();
        app.inventory = vec![inventory];
        app.scan_results = vec![exposed, scan_result("repo-a/discovered")];

        app.enter_list_mode();

        assert_eq!(app.list_filter, ListFilter::Full);
        assert_eq!(app.list_rows.len(), 2);
        assert!(matches!(app.list_rows[0], UnifiedListRow::Exposed(_)));
        assert!(matches!(app.list_rows[1], UnifiedListRow::Discovered(_)));
    }

    #[test]
    fn confirmed_repository_update_pulls_and_returns_to_list() {
        let temp = tempdir().unwrap();
        let remote = temp.path().join("remote.git");
        let repo = temp.path().join("skills/repo");
        let publisher = temp.path().join("publisher");
        fs::create_dir_all(temp.path().join("skills")).unwrap();
        git(&[
            "init",
            "--bare",
            "--initial-branch=main",
            remote.to_str().unwrap(),
        ]);
        git(&["clone", remote.to_str().unwrap(), repo.to_str().unwrap()]);
        fs::create_dir_all(repo.join("review")).unwrap();
        fs::write(repo.join("review/SKILL.md"), "# base").unwrap();
        git(&["-C", repo.to_str().unwrap(), "add", "."]);
        git(&[
            "-C",
            repo.to_str().unwrap(),
            "-c",
            "user.name=Skills Manager Tests",
            "-c",
            "user.email=tests@example.com",
            "commit",
            "-m",
            "base",
        ]);
        git(&[
            "-C",
            repo.to_str().unwrap(),
            "push",
            "--set-upstream",
            "origin",
            "main",
        ]);

        git(&[
            "clone",
            remote.to_str().unwrap(),
            publisher.to_str().unwrap(),
        ]);
        fs::write(publisher.join("review/SKILL.md"), "# updated").unwrap();
        git(&["-C", publisher.to_str().unwrap(), "add", "."]);
        git(&[
            "-C",
            publisher.to_str().unwrap(),
            "-c",
            "user.name=Skills Manager Tests",
            "-c",
            "user.email=tests@example.com",
            "commit",
            "-m",
            "remote update",
        ]);
        git(&["-C", publisher.to_str().unwrap(), "push", "origin", "main"]);

        let mut app = App::new(test_config(temp.path())).unwrap();
        app.mode = Mode::RepositoryUpdate;
        app.repository_update_step = RepositoryUpdateStep::Preview {
            update: RepositoryUpdate {
                repo_path: repo.clone(),
                commits: vec![crate::git::RepositoryCommit {
                    id: "abc1234".to_string(),
                    subject: "remote update".to_string(),
                }],
            },
            scroll: 0,
        };

        app.advance_repository_update("y").unwrap();

        assert_eq!(app.mode, Mode::List);
        assert_eq!(
            fs::read_to_string(repo.join("review/SKILL.md")).unwrap(),
            "# updated"
        );
        assert!(matches!(
            app.repository_update_step,
            RepositoryUpdateStep::Done { .. }
        ));
    }

    fn scan_result(skill_id: &str) -> ScanResult {
        ScanResult {
            skill_id: skill_id.to_string(),
            skill_path: PathBuf::from(format!("/skills/{skill_id}")),
            skill_relative_path: None,
            repo_name: skill_id.split_once('/').map(|(repo, _)| repo.to_string()),
            repo_path: None,
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        }
    }

    fn inventory_row(skill_id: &str) -> InventoryRow {
        let (namespace, name) = skill_id
            .split_once('/')
            .map(|(namespace, name)| (namespace.to_string(), name.to_string()))
            .unwrap_or_else(|| (String::new(), skill_id.to_string()));
        InventoryRow {
            skill_id: SkillId { namespace, name },
            source: SkillSource {
                repo_name: None,
                repo_path: None,
                remote_url: None,
            },
            scope: Scope::Global,
            exposures: vec![SkillExposure {
                agent_id: AgentId("claude".to_string()),
                path: PathBuf::from("/agents/claude/skill"),
                connection: ConnectionKind::Symlink,
            }],
            disambiguation_index: None,
        }
    }

    fn project_local_inventory_row(skill_id: &str, project: &str) -> InventoryRow {
        let mut row = inventory_row(skill_id);
        row.scope = Scope::ProjectLocal;
        row.source.repo_name = Some(
            skill_id
                .split_once('/')
                .map(|(namespace, _)| namespace)
                .unwrap_or(skill_id)
                .to_string(),
        );
        row.source.repo_path = Some(PathBuf::from(project));
        row.exposures = vec![
            SkillExposure {
                agent_id: AgentId("codex".to_string()),
                path: PathBuf::from(project)
                    .join(".agents/skills")
                    .join(&row.skill_id.name),
                connection: ConnectionKind::PhysicalCopy,
            },
            SkillExposure {
                agent_id: AgentId("copilot".to_string()),
                path: PathBuf::from(project)
                    .join(".agents/skills")
                    .join(&row.skill_id.name),
                connection: ConnectionKind::PhysicalCopy,
            },
        ];
        row
    }

    #[test]
    fn parse_command_list() {
        assert_eq!(parse_command("list"), TuiCommand::List);
    }

    #[test]
    fn parse_command_slash_list() {
        assert_eq!(parse_command("/list"), TuiCommand::List);
    }

    #[test]
    fn parse_command_scan_is_unknown_in_the_tui() {
        assert_eq!(
            parse_command("/scan"),
            TuiCommand::Unknown("/scan".to_string())
        );
    }

    #[test]
    fn parse_command_import_with_arg() {
        assert_eq!(parse_command("import repo-a/skill"), TuiCommand::Import);
    }

    #[test]
    fn parse_command_remove_with_arg() {
        assert_eq!(parse_command("remove repo-a/skill"), TuiCommand::Remove);
    }

    #[test]
    fn parse_command_source_add_with_url() {
        assert_eq!(
            parse_command("/source_add https://example.com/org/skills.git"),
            TuiCommand::SourceAdd("https://example.com/org/skills.git".to_string())
        );
    }

    #[test]
    fn parse_command_source_add_with_space_is_unknown() {
        assert_eq!(
            parse_command("/source add https://example.com/org/skills.git"),
            TuiCommand::Unknown("/source add https://example.com/org/skills.git".to_string())
        );
    }

    #[test]
    fn parse_command_help_question_mark() {
        assert_eq!(parse_command("?"), TuiCommand::Help);
    }

    #[test]
    fn parse_command_quit() {
        assert_eq!(parse_command("q"), TuiCommand::Quit);
    }

    #[test]
    fn parse_command_unknown() {
        assert_eq!(
            parse_command("foobar"),
            TuiCommand::Unknown("foobar".to_string())
        );
    }

    #[test]
    fn source_add_step_default_is_done() {
        assert!(matches!(
            SourceAddStep::default(),
            SourceAddStep::Done { .. }
        ));
    }

    #[test]
    fn handle_command_list_switches_mode() {
        let mut app = test_app();

        app.handle_command("list").expect("command succeeds");

        assert_eq!(app.mode, Mode::List);
    }

    #[test]
    fn enter_list_mode_selects_first_row_when_inventory_exists() {
        let mut app = test_app();
        app.inventory = vec![inventory_row("repo-a/one"), inventory_row("repo-a/two")];

        app.enter_list_mode();

        assert_eq!(app.mode, Mode::List);
        assert_eq!(app.list_table.selected_index(), Some(0));
        assert_eq!(app.list_table.viewport_offset(), 0);
        assert_eq!(app.list_table.visible_rows().len(), 1);
    }

    #[test]
    fn list_groups_rows_by_global_and_project_exposure_context() {
        let mut app = test_app();
        let mut global = inventory_row("skills/review");
        global.source.repo_name = Some("skills".to_string());
        global.source.repo_path = Some(PathBuf::from("/Users/alice/pgit/skills"));
        global.exposures[0].path = PathBuf::from("/Users/alice/pgit/skills/review");
        global.exposures[0].connection = ConnectionKind::PhysicalCopy;
        let local =
            project_local_inventory_row("analystloop/adx-intake", "/Users/alice/pgit/analystloop");
        app.inventory = vec![global, local];

        app.enter_list_mode();

        assert_eq!(app.list_table.groups().len(), 2);
        assert!(
            app.list_table
                .groups()
                .iter()
                .any(|group| group.name == "skills")
        );
        let project_group = app
            .list_table
            .groups()
            .iter()
            .find(|group| group.name == "analystloop")
            .expect("project group");
        assert_eq!(project_group.context, "pgit/analystloop");
        assert_eq!(
            project_group.items[0].display_path,
            ".agents/skills/adx-intake"
        );
    }

    #[test]
    fn list_groups_global_rows_by_source_repository() {
        let mut app = test_app();
        let mut repo_a = inventory_row("repo-a/one");
        repo_a.source.repo_name = Some("repo-a".to_string());
        repo_a.source.repo_path = Some(PathBuf::from("/Users/alice/pgit/repo-a"));
        repo_a.exposures[0].path = PathBuf::from("/Users/alice/.codex/skills/one");
        let mut repo_b = inventory_row("repo-b/two");
        repo_b.source.repo_name = Some("repo-b".to_string());
        repo_b.source.repo_path = Some(PathBuf::from("/Users/alice/pgit/repo-b"));
        repo_b.exposures[0].path = PathBuf::from("/Users/alice/.codex/skills/two");
        app.inventory = vec![repo_a, repo_b];

        app.enter_list_mode();

        assert_eq!(app.list_table.groups().len(), 2);
        assert_eq!(
            app.list_table
                .groups()
                .iter()
                .map(|group| group.name.as_str())
                .collect::<Vec<_>>(),
            vec!["repo-a", "repo-b"]
        );
    }

    #[test]
    fn project_local_rows_are_read_only_for_import_and_remove() {
        let mut app = test_app();
        app.inventory = vec![project_local_inventory_row(
            "analystloop/adx-intake",
            "/Users/alice/pgit/analystloop",
        )];
        app.enter_list_mode();
        app.list_table.move_right(5);
        app.list_table.move_right(5);

        app.start_import_from_selected_list_row()
            .expect("import action");
        assert_eq!(app.mode, Mode::List);
        assert!(
            app.info_message
                .as_deref()
                .is_some_and(|message| message.contains("read-only"))
        );

        app.start_remove_from_selected_list_row()
            .expect("remove action");
        assert_eq!(app.mode, Mode::List);
        assert!(
            app.info_message
                .as_deref()
                .is_some_and(|message| message.contains("read-only"))
        );
    }

    #[test]
    fn remove_all_creates_one_change_per_unique_target_path() {
        let mut app = test_app();
        let mut row = inventory_row("repo-a/skill");
        row.exposures.push(SkillExposure {
            agent_id: AgentId("codex".to_string()),
            path: PathBuf::from("/agents/claude/skill"),
            connection: ConnectionKind::Symlink,
        });
        row.exposures.push(SkillExposure {
            agent_id: AgentId("copilot".to_string()),
            path: PathBuf::from("/agents/copilot/skill"),
            connection: ConnectionKind::PhysicalCopy,
        });
        app.inventory = vec![row];
        app.enter_list_mode();
        app.list_table.move_right(5);
        app.list_table.move_right(5);

        app.start_remove_from_selected_list_row()
            .expect("remove action");
        app.advance_remove("all").expect("all selection succeeds");

        let RemoveStep::ConfirmPlan { plan, .. } = &app.remove_step else {
            panic!("expected a remove plan after choosing all");
        };
        let target_paths = plan
            .changes
            .iter()
            .map(|change| match change {
                StagedChange::DetachSkill { target_path, .. }
                | StagedChange::DeletePhysicalCopy { target_path, .. } => target_path.clone(),
                _ => panic!("expected removal changes"),
            })
            .collect::<Vec<_>>();
        assert!(plan.has_physical_deletes());
        assert_eq!(
            target_paths,
            vec![
                PathBuf::from("/agents/claude/skill"),
                PathBuf::from("/agents/copilot/skill"),
            ]
        );
    }

    #[test]
    fn list_summary_reports_imported_and_discovered_rows_separately() {
        let mut app = test_app();
        let exposed = scan_result("repo-a/exposed");
        let mut imported = inventory_row("repo-a/exposed");
        imported.exposures[0].path = exposed.skill_path.clone();
        app.inventory = vec![imported];
        app.scan_results = vec![exposed, scan_result("repo-a/discovered")];

        assert_eq!(
            app.list_summary_message(),
            "Imported: 1 · Discovered not imported: 1"
        );
    }

    #[test]
    fn list_groups_global_rows_by_source_repository_with_privacy_safe_paths() {
        let mut app = test_app();
        let repo_path = PathBuf::from("/Users/alice/pgit/repo-a");
        let skill_path = repo_path.join(".agents/skills/one");
        app.scan_results = vec![ScanResult {
            skill_id: "repo-a/one".to_string(),
            skill_path,
            skill_relative_path: Some(PathBuf::from(".agents/skills/one")),
            repo_name: Some("repo-a".to_string()),
            repo_path: Some(repo_path.clone()),
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        }];
        let mut row = inventory_row("repo-a/one");
        row.source.repo_name = Some("repo-a".to_string());
        row.source.repo_path = Some(repo_path);
        row.exposures[0].path = app.scan_results[0].skill_path.clone();
        row.exposures[0].connection = ConnectionKind::PhysicalCopy;
        app.inventory = vec![row];

        app.enter_list_mode();
        app.list_table.move_right(4);
        let list_path = match &app.list_table.visible_rows()[1] {
            SourceTableRow::Item { display_path, .. } => display_path.clone(),
            _ => panic!("expected list child row"),
        };

        assert_eq!(list_path, ".agents/skills/one");
        assert!(!list_path.contains("alice"));
    }

    #[test]
    fn list_projection_maps_duplicates_within_each_same_name_repository() {
        let mut app = test_app();
        let repo_one = PathBuf::from("/Users/alice/one/skills");
        let repo_two = PathBuf::from("/Users/alice/two/skills");
        app.scan_results = vec![
            ScanResult {
                skill_id: "skills/docs".to_string(),
                skill_path: repo_one.join(".agents/skills/docs"),
                skill_relative_path: Some(PathBuf::from(".agents/skills/docs")),
                repo_name: Some("skills".to_string()),
                repo_path: Some(repo_one.clone()),
                remote_url: None,
                source_kind: SourceKind::CentralDir,
                disambiguation_index: Some(1),
            },
            ScanResult {
                skill_id: "skills/docs".to_string(),
                skill_path: repo_one.join("skills/docs"),
                skill_relative_path: Some(PathBuf::from("skills/docs")),
                repo_name: Some("skills".to_string()),
                repo_path: Some(repo_one.clone()),
                remote_url: None,
                source_kind: SourceKind::CentralDir,
                disambiguation_index: Some(2),
            },
            ScanResult {
                skill_id: "skills/docs".to_string(),
                skill_path: repo_two.join(".agents/skills/docs"),
                skill_relative_path: Some(PathBuf::from(".agents/skills/docs")),
                repo_name: Some("skills".to_string()),
                repo_path: Some(repo_two.clone()),
                remote_url: None,
                source_kind: SourceKind::CentralDir,
                disambiguation_index: Some(3),
            },
            ScanResult {
                skill_id: "skills/docs".to_string(),
                skill_path: repo_two.join("skills/docs"),
                skill_relative_path: Some(PathBuf::from("skills/docs")),
                repo_name: Some("skills".to_string()),
                repo_path: Some(repo_two.clone()),
                remote_url: None,
                source_kind: SourceKind::CentralDir,
                disambiguation_index: Some(4),
            },
        ];
        app.inventory = [repo_one, repo_two]
            .into_iter()
            .flat_map(|repo_path| {
                [1, 2].into_iter().map(move |index| {
                    let mut row = inventory_row("skills/docs");
                    row.source.repo_name = Some("skills".to_string());
                    row.source.repo_path = Some(repo_path.clone());
                    row.disambiguation_index = Some(index);
                    row
                })
            })
            .enumerate()
            .map(|(index, mut row)| {
                row.disambiguation_index = Some(index + 1);
                row
            })
            .collect();

        let paths = app
            .inventory
            .iter()
            .map(|row| {
                app.scan_result_for_inventory_row(row)
                    .and_then(|result| result.skill_relative_path.clone())
            })
            .collect::<Vec<_>>();

        assert_eq!(
            paths,
            vec![
                Some(PathBuf::from(".agents/skills/docs")),
                Some(PathBuf::from("skills/docs")),
                Some(PathBuf::from(".agents/skills/docs")),
                Some(PathBuf::from("skills/docs")),
            ]
        );
    }

    #[test]
    fn handle_command_quit_returns_true() {
        let mut app = test_app();

        let should_quit = app.handle_command("q").expect("command succeeds");

        assert!(should_quit);
    }

    #[test]
    fn handle_command_unknown_sets_error_message() {
        let mut app = test_app();

        app.handle_command("foobar").expect("command succeeds");

        assert!(app.error_message.is_some());
    }

    #[test]
    fn handle_command_help_switches_mode() {
        let mut app = test_app();

        app.handle_command("help").expect("command succeeds");

        assert_eq!(app.mode, Mode::Help);
    }

    #[test]
    fn source_add_decline_leaves_managed_directory_unchanged() {
        let temp = tempdir().expect("tempdir");
        let remote = create_git_repo(temp.path().join("remote-skills"));
        let central = temp.path().join("central");
        let mut config = test_config(temp.path());
        config.skills.central_dir = central.to_string_lossy().into_owned();
        let mut app = App::new(config).unwrap();

        app.handle_command(&format!("/source_add file://{}", remote.display()))
            .unwrap();
        app.advance_source_add("n").unwrap();

        assert!(!central.exists());
        assert!(matches!(app.source_add_step, SourceAddStep::Done { .. }));
    }

    #[test]
    fn source_add_selects_one_skill_and_delegates_to_import_plan() {
        let temp = tempdir().expect("tempdir");
        let remote = create_git_repo(temp.path().join("remote-skills"));
        let central = temp.path().join("central");
        let mut config = test_config(temp.path());
        config.skills.central_dir = central.to_string_lossy().into_owned();
        let mut app = App::new(config).unwrap();

        app.handle_command(&format!("/source_add file://{}", remote.display()))
            .unwrap();
        app.advance_source_add("y").unwrap();
        app.advance_source_add("1").unwrap();

        assert!(central.join("remote-skills").exists());
        assert_eq!(app.mode, Mode::Import);
        assert!(matches!(app.import_step, ImportStep::SelectAgents { .. }));
    }

    #[test]
    fn source_add_multi_skill_selection_delegates_only_selected_skill() {
        let temp = tempdir().expect("tempdir");
        let remote = create_git_repo(temp.path().join("remote-skills"));
        fs::create_dir_all(remote.join("docs")).unwrap();
        fs::write(remote.join("docs/SKILL.md"), "# Docs").unwrap();
        git(&["-C", remote.to_str().unwrap(), "add", "."]);
        git(&[
            "-C",
            remote.to_str().unwrap(),
            "-c",
            "user.name=Skills Manager Tests",
            "-c",
            "user.email=tests@example.com",
            "commit",
            "-m",
            "add docs skill",
        ]);
        let mut config = test_config(temp.path());
        config.skills.central_dir = temp.path().join("central").to_string_lossy().into_owned();
        let mut app = App::new(config).unwrap();

        app.handle_command(&format!("/source_add file://{}", remote.display()))
            .unwrap();
        app.advance_source_add("y").unwrap();
        app.advance_source_add("2").unwrap();

        let ImportStep::SelectAgents { selected, .. } = &app.import_step else {
            panic!("expected selected skill to enter import flow");
        };
        assert_eq!(selected[0].skill_id, "remote-skills/docs");
    }

    #[test]
    fn cancelling_exposure_after_source_add_keeps_source_without_targets() {
        let temp = tempdir().expect("tempdir");
        let remote = create_git_repo(temp.path().join("remote-skills"));
        let central = temp.path().join("central");
        let mut config = test_config(temp.path());
        config.skills.central_dir = central.to_string_lossy().into_owned();
        let target_paths = config
            .agents
            .values()
            .map(|agent| PathBuf::from(&agent.global_dir))
            .collect::<Vec<_>>();
        let mut app = App::new(config).unwrap();

        app.handle_command(&format!("/source_add file://{}", remote.display()))
            .unwrap();
        app.advance_source_add("y").unwrap();
        app.advance_source_add("1").unwrap();
        app.advance_import("").unwrap();
        app.advance_import("n").unwrap();

        assert!(central.join("remote-skills").exists());
        assert!(target_paths.iter().all(|path| !path.exists()));
        assert!(matches!(app.import_step, ImportStep::Done { .. }));
    }

    fn create_git_repo(path: PathBuf) -> PathBuf {
        fs::create_dir_all(path.join("code-review")).unwrap();
        fs::write(path.join("code-review/SKILL.md"), "# Code review").unwrap();
        git(&["init", path.to_str().unwrap()]);
        git(&["-C", path.to_str().unwrap(), "add", "."]);
        git(&[
            "-C",
            path.to_str().unwrap(),
            "-c",
            "user.name=Skills Manager Tests",
            "-c",
            "user.email=tests@example.com",
            "commit",
            "-m",
            "initial",
        ]);
        path
    }

    fn git(args: &[&str]) {
        assert!(Command::new("git").args(args).status().unwrap().success());
    }

    #[test]
    fn handle_command_import_guides_to_table_shortcut() {
        let mut app = test_app();

        app.handle_command("/import repo-a/skill")
            .expect("command succeeds");

        assert_eq!(app.mode, Mode::Home);
        assert!(matches!(app.import_step, ImportStep::Done { .. }));
        assert!(
            app.info_message
                .as_deref()
                .is_some_and(|message| message.contains("press i"))
        );
    }

    #[test]
    fn handle_command_remove_guides_to_table_shortcut() {
        let mut app = test_app();

        app.handle_command("/remove repo-a/skill")
            .expect("command succeeds");

        assert_eq!(app.mode, Mode::Home);
        assert!(matches!(app.remove_step, RemoveStep::Done { .. }));
        assert!(
            app.info_message
                .as_deref()
                .is_some_and(|message| message.contains("press x"))
        );
    }

    #[test]
    fn command_suggestions_include_primary_prompt_commands() {
        let app = test_app();
        let labels = app
            .filtered_command_suggestions()
            .iter()
            .map(|suggestion| suggestion.label)
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec!["/list", "/source_add", "/config", "/help", "/quit"]
        );
        assert!(
            app.filtered_command_suggestions()
                .iter()
                .all(|suggestion| !suggestion.description.is_empty())
        );
    }

    #[test]
    fn source_add_suggestion_mentions_clone_url_formats() {
        let app = test_app();
        let suggestion = app
            .filtered_command_suggestions()
            .into_iter()
            .find(|suggestion| suggestion.label == "/source_add")
            .expect("source_add suggestion exists");

        assert!(suggestion.description.contains("HTTPS"));
        assert!(suggestion.description.contains("SSH"));
        assert!(suggestion.description.contains("clone URL"));
    }

    #[test]
    fn command_suggestions_do_not_offer_scan() {
        let mut app = test_app();
        app.input = "/sc".to_string();
        app.open_command_menu();

        let labels = app
            .filtered_command_suggestions()
            .iter()
            .map(|suggestion| suggestion.label)
            .collect::<Vec<_>>();

        assert!(labels.is_empty());
    }

    #[test]
    fn command_suggestions_filter_by_command_text_before_arguments() {
        let mut app = test_app();
        app.input = "/source_add https://example.com/org/skills.git".to_string();
        app.open_command_menu();

        let labels = app
            .filtered_command_suggestions()
            .iter()
            .map(|suggestion| suggestion.label)
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["/source_add"]);
    }

    #[test]
    fn command_suggestions_can_be_selected() {
        let mut app = test_app();
        app.open_command_menu();

        assert_eq!(
            app.selected_command_suggestion().map(|item| item.label),
            Some("/list")
        );

        app.move_command_suggestion_down();

        assert_eq!(
            app.selected_command_suggestion().map(|item| item.label),
            Some("/source_add")
        );

        app.move_command_suggestion_up();

        assert_eq!(
            app.selected_command_suggestion().map(|item| item.label),
            Some("/list")
        );
    }

    #[test]
    fn advance_import_done_step_resets_to_home() {
        let mut app = test_app();
        app.mode = Mode::Import;
        app.import_step = ImportStep::Done {
            message: "test".to_string(),
        };

        app.advance_import("").expect("advance succeeds");

        assert_eq!(app.mode, Mode::Home);
    }

    #[test]
    fn confirmed_import_returns_home_with_status_message() {
        let temp = tempdir().expect("tempdir");
        let config = test_config(temp.path());
        let source = temp.path().join("skills/repo-a/skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Skill").unwrap();
        let target = PathBuf::from(&config.agents["claude"].global_dir).join("skill");
        let mut app = App::new(config).unwrap();
        app.mode = Mode::Import;
        app.import_step = ImportStep::ConfirmPlan {
            plan: ChangePlan::new(vec![StagedChange::ExposeSkill {
                skill_name: "repo-a/skill".to_string(),
                agent_id: AgentId("Claude".to_string()),
                source_path: source,
                target_path: target.clone(),
            }]),
            selected: vec![scan_result("repo-a/skill")],
            target_agents: vec![],
        };

        app.advance_import("y").expect("import applies");

        assert_eq!(app.mode, Mode::Home);
        assert!(matches!(app.import_step, ImportStep::Done { .. }));
        let expected_message = format!(
            "Applied 1 change(s).\n  ✓ Exposed repo-a/skill at {}",
            target.display()
        );
        assert_eq!(app.info_message.as_deref(), Some(expected_message.as_str()));
    }

    #[test]
    fn move_agent_selection_up_decrements_focus() {
        let mut app = test_app();
        app.import_step = ImportStep::SelectAgents {
            selected: vec![scan_result("repo-a/skill")],
            agents: vec![
                AgentSelectionItem {
                    target: crate::inventory::AgentTarget {
                        agent_id: "claude".to_string(),
                        display_name: "Claude".to_string(),
                        global_dir: None,
                        shared_target_dirs: vec![],
                        enabled: true,
                    },
                    checked: true,
                },
                AgentSelectionItem {
                    target: crate::inventory::AgentTarget {
                        agent_id: "codex".to_string(),
                        display_name: "Codex".to_string(),
                        global_dir: None,
                        shared_target_dirs: vec![],
                        enabled: true,
                    },
                    checked: true,
                },
            ],
            focused: 1,
        };

        app.move_agent_selection_up();

        assert!(matches!(
            app.import_step,
            ImportStep::SelectAgents { focused: 0, .. }
        ));
    }

    #[test]
    fn move_agent_selection_down_increments_focus() {
        let mut app = test_app();
        app.import_step = ImportStep::SelectAgents {
            selected: vec![scan_result("repo-a/skill")],
            agents: vec![
                AgentSelectionItem {
                    target: crate::inventory::AgentTarget {
                        agent_id: "claude".to_string(),
                        display_name: "Claude".to_string(),
                        global_dir: None,
                        shared_target_dirs: vec![],
                        enabled: true,
                    },
                    checked: true,
                },
                AgentSelectionItem {
                    target: crate::inventory::AgentTarget {
                        agent_id: "codex".to_string(),
                        display_name: "Codex".to_string(),
                        global_dir: None,
                        shared_target_dirs: vec![],
                        enabled: true,
                    },
                    checked: true,
                },
            ],
            focused: 0,
        };

        app.move_agent_selection_down();

        assert!(matches!(
            app.import_step,
            ImportStep::SelectAgents { focused: 1, .. }
        ));
    }

    #[test]
    fn toggle_agent_selection_flips_checked() {
        let mut app = test_app();
        app.import_step = ImportStep::SelectAgents {
            selected: vec![scan_result("repo-a/skill")],
            agents: vec![AgentSelectionItem {
                target: crate::inventory::AgentTarget {
                    agent_id: "claude".to_string(),
                    display_name: "Claude".to_string(),
                    global_dir: None,
                    shared_target_dirs: vec![],
                    enabled: true,
                },
                checked: true,
            }],
            focused: 0,
        };

        app.toggle_agent_selection();

        if let ImportStep::SelectAgents { agents, .. } = &app.import_step {
            assert!(!agents[0].checked);
        } else {
            panic!("expected SelectAgents");
        }
    }

    #[test]
    fn import_step_hint_returns_string() {
        let app = App {
            import_step: ImportStep::Disambiguate {
                matches: vec![scan_result("repo-a/skill")],
            },
            ..test_app()
        };
        assert!(!app.import_step_hint().is_empty());

        let app = App {
            import_step: ImportStep::SelectAgents {
                selected: vec![scan_result("repo-a/skill")],
                agents: vec![],
                focused: 0,
            },
            ..test_app()
        };
        assert!(!app.import_step_hint().is_empty());

        let app = App {
            import_step: ImportStep::ConfirmPlan {
                plan: ChangePlan::new(vec![]),
                selected: vec![scan_result("repo-a/skill")],
                target_agents: vec![],
            },
            ..test_app()
        };
        assert!(!app.import_step_hint().is_empty());

        let app = App {
            import_step: ImportStep::ConfirmPhysical {
                plan: ChangePlan::new(vec![]),
            },
            ..test_app()
        };
        assert!(!app.import_step_hint().is_empty());

        let app = App {
            import_step: ImportStep::Done {
                message: "done".to_string(),
            },
            ..test_app()
        };
        assert!(!app.import_step_hint().is_empty());
    }

    #[test]
    fn remove_step_hint_returns_string() {
        let app = App {
            remove_step: RemoveStep::ConfirmPlan {
                plan: ChangePlan::new(vec![]),
                selected: Box::new(inventory_row("repo-a/skill")),
            },
            ..test_app()
        };
        assert!(!app.remove_step_hint().is_empty());

        let app = App {
            remove_step: RemoveStep::ConfirmPhysical {
                plan: ChangePlan::new(vec![]),
            },
            ..test_app()
        };
        assert!(!app.remove_step_hint().is_empty());

        let app = App {
            remove_step: RemoveStep::Done {
                message: "done".to_string(),
            },
            ..test_app()
        };
        assert!(!app.remove_step_hint().is_empty());
    }
}
