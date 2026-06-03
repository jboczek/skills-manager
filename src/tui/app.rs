use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::helpers;
use crate::config::{Config, expand_tilde};
use crate::domain::{AgentId, ConnectionKind, InventoryRow};
use crate::git;
use crate::inventory::AgentTarget;
use crate::plan::{ChangePlan, StagedChange};
use crate::plan_apply;
use crate::scanner::{self, ScanResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiCommand {
    List,
    Scan,
    Import(String),
    Remove(String),
    Config,
    Help,
    Quit,
    Unknown(String),
}

/// Parse a command string typed in the prompt.
/// Accepts "list", "/list", "scan", "/scan", "import <skill>", etc.
pub fn parse_command(input: &str) -> TuiCommand {
    let trimmed = input.trim();
    let normalized = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let mut parts = normalized.split_whitespace();
    let Some(command) = parts.next() else {
        return TuiCommand::Unknown(input.to_string());
    };
    let argument = parts.collect::<Vec<_>>().join(" ");

    match command {
        "list" => TuiCommand::List,
        "scan" => TuiCommand::Scan,
        "import" => TuiCommand::Import(argument),
        "remove" => TuiCommand::Remove(argument),
        "config" => TuiCommand::Config,
        "help" | "?" => TuiCommand::Help,
        "q" | "quit" => TuiCommand::Quit,
        _ => TuiCommand::Unknown(trimmed.to_string()),
    }
}

#[derive(Debug, Clone, Default)]
pub enum ImportStep {
    #[default]
    EnterSkill,
    Disambiguate {
        matches: Vec<ScanResult>,
    },
    SelectAgents {
        selected: Box<ScanResult>,
    },
    ConfirmPlan {
        plan: ChangePlan,
        selected: Box<ScanResult>,
        target_agents: Vec<AgentTarget>,
    },
    ConfirmPhysical {
        plan: ChangePlan,
    },
    Done {
        message: String,
    },
}

#[derive(Debug, Clone, Default)]
pub enum RemoveStep {
    #[default]
    EnterSkill,
    Disambiguate {
        matches: Vec<InventoryRow>,
    },
    ConfirmPlan {
        plan: ChangePlan,
        selected: Box<InventoryRow>,
    },
    ConfirmPhysical {
        plan: ChangePlan,
    },
    Done {
        message: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Home,
    List,
    Scan,
    Config,
    Help,
    Import,
    Remove,
    Quit,
}

pub struct App {
    pub mode: Mode,
    pub input: String,
    pub inventory: Vec<InventoryRow>,
    pub scan_results: Vec<ScanResult>,
    pub status_messages: Vec<String>,
    pub list_scroll: usize,
    pub list_selected: Option<usize>,
    pub config: Config,
    pub current_dir: PathBuf,
    pub git_branch: Option<String>,
    pub prompt_label: String,
    pub import_step: ImportStep,
    pub remove_step: RemoveStep,
    pub error_message: Option<String>,
    pub info_message: Option<String>,
}

impl App {
    pub fn new(config: Config, current_dir: PathBuf) -> Self {
        let prompt_label = format_prompt_label(&current_dir, None);
        Self {
            mode: Mode::Home,
            input: String::new(),
            inventory: Vec::new(),
            scan_results: Vec::new(),
            status_messages: Vec::new(),
            list_scroll: 0,
            list_selected: None,
            config,
            current_dir,
            git_branch: None,
            prompt_label,
            import_step: ImportStep::EnterSkill,
            remove_step: RemoveStep::EnterSkill,
            error_message: None,
            info_message: None,
        }
    }

    /// Load initial state: build prompt_label, detect git branch, run initial scan/inventory.
    pub fn initialize(&mut self) -> anyhow::Result<()> {
        self.git_branch = detect_git_branch(&self.current_dir);
        self.prompt_label = format_prompt_label(&self.current_dir, self.git_branch.as_deref());
        self.reload_scan_results()?;
        self.refresh_inventory()?;
        self.rebuild_status_messages();
        Ok(())
    }

    /// Refresh inventory from filesystem.
    pub fn refresh_inventory(&mut self) -> anyhow::Result<()> {
        self.inventory = helpers::fresh_inventory(&self.config, &self.current_dir)?;
        self.rebuild_status_messages();
        Ok(())
    }

    /// Handle a submitted command string from the prompt.
    /// Returns true if the app should quit.
    pub fn handle_command(&mut self, input: &str) -> anyhow::Result<bool> {
        self.error_message = None;
        self.info_message = None;

        match self.mode {
            Mode::Import => {
                self.advance_import(input)?;
                return Ok(false);
            }
            Mode::Remove => {
                self.advance_remove(input)?;
                return Ok(false);
            }
            _ => {}
        }

        match parse_command(input) {
            TuiCommand::List => {
                self.refresh_inventory()?;
                self.mode = Mode::List;
                self.list_scroll = 0;
                self.list_selected = None;
            }
            TuiCommand::Scan => {
                self.reload_scan_results()?;
                self.mode = Mode::Scan;
                self.list_scroll = 0;
                self.list_selected = None;
                self.info_message = Some(format!("Found {} skill(s).", self.scan_results.len()));
            }
            TuiCommand::Import(skill) => {
                self.refresh_inventory()?;
                self.mode = Mode::Import;
                self.import_step = ImportStep::EnterSkill;
                if !skill.trim().is_empty() {
                    self.advance_import(&skill)?;
                }
            }
            TuiCommand::Remove(skill) => {
                self.refresh_inventory()?;
                self.mode = Mode::Remove;
                self.remove_step = RemoveStep::EnterSkill;
                if !skill.trim().is_empty() {
                    self.advance_remove(&skill)?;
                }
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

    /// Handle import flow step progression.
    pub fn advance_import(&mut self, input: &str) -> anyhow::Result<()> {
        match self.import_step.clone() {
            ImportStep::EnterSkill => {
                let matches = helpers::find_scan_results_by_id(input.trim(), &self.scan_results);
                match matches.len() {
                    0 => {
                        self.error_message = Some(format!(
                            "Skill '{}' not found. Try /scan to see available skills.",
                            input.trim()
                        ));
                    }
                    1 => {
                        self.import_step = ImportStep::SelectAgents {
                            selected: Box::new(matches[0].clone()),
                        };
                    }
                    _ => {
                        self.import_step = ImportStep::Disambiguate {
                            matches: matches.into_iter().cloned().collect(),
                        };
                    }
                }
            }
            ImportStep::Disambiguate { matches } => match parse_selection(input, matches.len()) {
                Some(index) => {
                    self.import_step = ImportStep::SelectAgents {
                        selected: Box::new(matches[index].clone()),
                    };
                }
                None => {
                    self.error_message =
                        Some(format!("Enter a number between 1 and {}", matches.len()));
                    self.import_step = ImportStep::Disambiguate { matches };
                }
            },
            ImportStep::SelectAgents { selected } => {
                let Some(target_agents) = self.resolve_target_agents(input) else {
                    self.import_step = ImportStep::SelectAgents { selected };
                    return Ok(());
                };
                let plan = self.build_import_plan(&selected, &target_agents);
                if plan.is_empty() {
                    self.import_step = ImportStep::Done {
                        message: "Nothing to do.".to_string(),
                    };
                    self.info_message = Some("Nothing to do.".to_string());
                } else {
                    self.import_step = ImportStep::ConfirmPlan {
                        plan,
                        selected,
                        target_agents,
                    };
                }
            }
            ImportStep::ConfirmPlan {
                plan,
                selected,
                target_agents,
            } => {
                let normalized = input.trim().to_ascii_lowercase();
                if normalized == "y" {
                    if plan.has_physical_deletes() {
                        self.import_step = ImportStep::ConfirmPhysical { plan };
                    } else {
                        let message = self.apply_plan_and_refresh(&plan)?;
                        self.import_step = ImportStep::Done { message };
                    }
                } else if normalized == "n" || normalized.is_empty() {
                    self.import_step = ImportStep::Done {
                        message: "Aborted.".to_string(),
                    };
                } else {
                    self.error_message = Some("Apply this plan? [y/N]".to_string());
                    self.import_step = ImportStep::ConfirmPlan {
                        plan,
                        selected,
                        target_agents,
                    };
                }
            }
            ImportStep::ConfirmPhysical { plan } => {
                if input.trim() == "yes" {
                    let message = self.apply_plan_and_refresh(&plan)?;
                    self.import_step = ImportStep::Done { message };
                } else {
                    self.import_step = ImportStep::Done {
                        message: "Aborted.".to_string(),
                    };
                }
            }
            ImportStep::Done { .. } => {
                self.mode = Mode::Home;
                self.import_step = ImportStep::EnterSkill;
            }
        }

        Ok(())
    }

    /// Handle remove flow step progression.
    pub fn advance_remove(&mut self, input: &str) -> anyhow::Result<()> {
        match self.remove_step.clone() {
            RemoveStep::EnterSkill => {
                let matches = helpers::find_inventory_rows_by_id(input.trim(), &self.inventory);
                match matches.len() {
                    0 => {
                        self.error_message = Some(format!(
                            "Skill '{}' not found in inventory. Try /list to see exposed skills.",
                            input.trim()
                        ));
                    }
                    1 => {
                        let selected = matches[0].clone();
                        let plan = self.build_remove_plan(&selected);
                        self.remove_step = if plan.is_empty() {
                            RemoveStep::Done {
                                message: "Nothing to remove.".to_string(),
                            }
                        } else {
                            RemoveStep::ConfirmPlan {
                                plan,
                                selected: Box::new(selected),
                            }
                        };
                    }
                    _ => {
                        self.remove_step = RemoveStep::Disambiguate {
                            matches: matches.into_iter().cloned().collect(),
                        };
                    }
                }
            }
            RemoveStep::Disambiguate { matches } => match parse_selection(input, matches.len()) {
                Some(index) => {
                    let selected = matches[index].clone();
                    let plan = self.build_remove_plan(&selected);
                    self.remove_step = if plan.is_empty() {
                        RemoveStep::Done {
                            message: "Nothing to remove.".to_string(),
                        }
                    } else {
                        RemoveStep::ConfirmPlan {
                            plan,
                            selected: Box::new(selected),
                        }
                    };
                }
                None => {
                    self.error_message =
                        Some(format!("Enter a number between 1 and {}", matches.len()));
                    self.remove_step = RemoveStep::Disambiguate { matches };
                }
            },
            RemoveStep::ConfirmPlan { plan, selected } => {
                let normalized = input.trim().to_ascii_lowercase();
                if normalized == "y" {
                    if plan.has_physical_deletes() {
                        self.remove_step = RemoveStep::ConfirmPhysical { plan };
                    } else {
                        let message = self.apply_plan_and_refresh(&plan)?;
                        self.remove_step = RemoveStep::Done { message };
                    }
                } else if normalized == "n" || normalized.is_empty() {
                    self.remove_step = RemoveStep::Done {
                        message: "Aborted.".to_string(),
                    };
                } else {
                    self.error_message = Some("Apply this plan? [y/N]".to_string());
                    self.remove_step = RemoveStep::ConfirmPlan { plan, selected };
                }
            }
            RemoveStep::ConfirmPhysical { plan } => {
                if input.trim() == "yes" {
                    let message = self.apply_plan_and_refresh(&plan)?;
                    self.remove_step = RemoveStep::Done { message };
                } else {
                    self.remove_step = RemoveStep::Done {
                        message: "Aborted.".to_string(),
                    };
                }
            }
            RemoveStep::Done { .. } => {
                self.mode = Mode::Home;
                self.remove_step = RemoveStep::EnterSkill;
            }
        }

        Ok(())
    }

    /// Return a short one-line hint for the current import step.
    pub fn import_step_hint(&self) -> &'static str {
        match self.import_step {
            ImportStep::EnterSkill => "Enter skill identifier (e.g. repo-a/code-review):",
            ImportStep::Disambiguate { .. } => "Enter number to select:",
            ImportStep::SelectAgents { .. } => {
                "Enter target agents (comma-separated, or Enter for all):"
            }
            ImportStep::ConfirmPlan { .. } => "Apply this plan? [y/N]:",
            ImportStep::ConfirmPhysical { .. } => "Type 'yes' to confirm permanent deletion:",
            ImportStep::Done { .. } => "Press Enter to return to home.",
        }
    }

    /// Return a short one-line hint for the current remove step.
    pub fn remove_step_hint(&self) -> &'static str {
        match self.remove_step {
            RemoveStep::EnterSkill => "Enter skill identifier (e.g. repo-a/code-review):",
            RemoveStep::Disambiguate { .. } => "Enter number to select:",
            RemoveStep::ConfirmPlan { .. } => "Apply this plan? [y/N]:",
            RemoveStep::ConfirmPhysical { .. } => "Type 'yes' to confirm permanent deletion:",
            RemoveStep::Done { .. } => "Press Enter to return to home.",
        }
    }

    fn reload_scan_results(&mut self) -> anyhow::Result<()> {
        self.scan_results =
            scanner::scan(&helpers::scan_config_from(&self.config, &self.current_dir))?;
        scanner::assign_disambiguation_indices(&mut self.scan_results);
        self.rebuild_status_messages();
        Ok(())
    }

    fn resolve_target_agents(&mut self, input: &str) -> Option<Vec<AgentTarget>> {
        let all_agents = helpers::agent_targets_from(&self.config, &self.current_dir);
        if input.trim().is_empty() {
            return Some(
                all_agents
                    .into_iter()
                    .filter(|agent| agent.enabled)
                    .collect(),
            );
        }

        let requested = helpers::parse_agents(input);
        for agent_id in &requested {
            if !self.config.agents.contains_key(agent_id) {
                self.error_message = Some(format!("Unknown agent: {agent_id}"));
                return None;
            }
        }

        let target_agents = all_agents
            .into_iter()
            .filter(|agent| {
                requested
                    .iter()
                    .any(|requested_id| requested_id == &agent.agent_id)
            })
            .collect::<Vec<_>>();
        if target_agents.is_empty() {
            self.error_message = Some("No matching target agents found.".to_string());
            return None;
        }

        Some(target_agents)
    }

    fn build_import_plan(
        &self,
        selected: &ScanResult,
        target_agents: &[AgentTarget],
    ) -> ChangePlan {
        let existing_paths = self
            .inventory
            .iter()
            .flat_map(|row| row.exposures.iter().map(|exposure| exposure.path.clone()))
            .collect::<HashSet<_>>();
        let skill_name = selected
            .skill_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                selected
                    .skill_id
                    .rsplit('/')
                    .next()
                    .unwrap_or(&selected.skill_id)
                    .to_string()
            });

        let changes = target_agents
            .iter()
            .filter_map(|agent| {
                let global_dir = agent.global_dir.as_ref()?;
                let target_path = global_dir.join(&skill_name);
                if existing_paths.contains(&target_path) || target_path.exists() {
                    return None;
                }
                Some(StagedChange::ExposeSkill {
                    skill_name: selected.skill_id.clone(),
                    agent_id: AgentId(agent.display_name.clone()),
                    source_path: selected.skill_path.clone(),
                    target_path,
                    connection: ConnectionKind::Symlink,
                })
            })
            .collect();

        ChangePlan::new(changes)
    }

    fn build_remove_plan(&self, row: &InventoryRow) -> ChangePlan {
        let skill_name = display_inventory_row(row);
        let changes = row
            .exposures
            .iter()
            .filter_map(|exposure| {
                let display_name = self
                    .config
                    .agents
                    .get(&exposure.agent_id.0)
                    .map(|agent| agent.display_name.clone())
                    .unwrap_or_else(|| exposure.agent_id.0.clone());
                match exposure.connection {
                    ConnectionKind::Symlink => Some(StagedChange::DetachSkill {
                        skill_name: skill_name.clone(),
                        agent_id: AgentId(display_name),
                        target_path: exposure.path.clone(),
                    }),
                    ConnectionKind::PhysicalCopy => Some(StagedChange::DeletePhysicalCopy {
                        skill_name: skill_name.clone(),
                        agent_id: AgentId(display_name),
                        target_path: exposure.path.clone(),
                    }),
                    ConnectionKind::Missing | ConnectionKind::Unknown => None,
                }
            })
            .collect();
        ChangePlan::new(changes)
    }

    fn apply_plan_and_refresh(&mut self, plan: &ChangePlan) -> anyhow::Result<String> {
        for change in &plan.changes {
            if let StagedChange::ExposeSkill { target_path, .. } = change
                && let Some(parent) = target_path.parent()
            {
                fs::create_dir_all(parent)?;
            }
        }

        let result = plan_apply::apply_plan(plan);
        let had_failure = result.failed.is_some();
        let message = match &result.failed {
            Some((_, error)) => format!(
                "Applied {} change(s). 1 change failed: {error}",
                result.applied.len()
            ),
            None => format!("Applied {} change(s).", result.applied.len()),
        };

        if had_failure {
            self.error_message = Some(message.clone());
        } else {
            self.info_message = Some(message.clone());
        }

        self.refresh_inventory()?;
        Ok(message)
    }

    fn rebuild_status_messages(&mut self) {
        self.status_messages = vec![
            format!(
                "• Environment loaded: {} skills, {} agents",
                self.inventory.len().max(self.scan_results.len()),
                self.config
                    .agents
                    .values()
                    .filter(|agent| agent.enabled)
                    .count()
            ),
            "• Scan: OK".to_string(),
        ];
    }
}

fn display_inventory_row(row: &InventoryRow) -> String {
    if row.skill_id.namespace.is_empty() {
        row.skill_id.name.clone()
    } else {
        format!("{}/{}", row.skill_id.namespace, row.skill_id.name)
    }
}

fn parse_selection(input: &str, max: usize) -> Option<usize> {
    let index = input.trim().parse::<usize>().ok()?;
    (1..=max).contains(&index).then_some(index - 1)
}

fn detect_git_branch(path: &Path) -> Option<String> {
    let repo_root = git::find_repo_root(path)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["branch", "--show-current"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!branch.is_empty()).then_some(branch)
}

fn format_prompt_label(current_dir: &Path, branch: Option<&str>) -> String {
    let display = display_path_with_tilde(current_dir);
    match branch {
        Some(branch) => format!("{display} [{branch}]"),
        None => display,
    }
}

fn display_path_with_tilde(path: &Path) -> String {
    let home = expand_tilde("~");
    if let Ok(relative) = path.strip_prefix(&home) {
        if relative.as_os_str().is_empty() {
            "~".to_string()
        } else {
            format!("~/{}", relative.display())
        }
    } else {
        path.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;
    use crate::domain::{
        AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure, SkillId, SkillSource,
    };
    use crate::scanner::SourceKind;

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
        App::new(test_config(&path), path)
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

    #[test]
    fn parse_command_list() {
        assert_eq!(parse_command("list"), TuiCommand::List);
    }

    #[test]
    fn parse_command_slash_list() {
        assert_eq!(parse_command("/list"), TuiCommand::List);
    }

    #[test]
    fn parse_command_import_with_arg() {
        assert_eq!(
            parse_command("import repo-a/skill"),
            TuiCommand::Import("repo-a/skill".to_string())
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
    fn handle_command_list_switches_mode() {
        let mut app = test_app();

        app.handle_command("list").expect("command succeeds");

        assert_eq!(app.mode, Mode::List);
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
    fn advance_import_enter_skill_not_found() {
        let mut app = test_app();

        app.advance_import("nonexistent").expect("advance succeeds");

        assert!(app.error_message.is_some());
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
    fn import_step_hint_returns_string() {
        let app = App {
            import_step: ImportStep::EnterSkill,
            ..test_app()
        };
        assert!(!app.import_step_hint().is_empty());

        let app = App {
            import_step: ImportStep::Disambiguate {
                matches: vec![scan_result("repo-a/skill")],
            },
            ..test_app()
        };
        assert!(!app.import_step_hint().is_empty());

        let app = App {
            import_step: ImportStep::SelectAgents {
                selected: Box::new(scan_result("repo-a/skill")),
            },
            ..test_app()
        };
        assert!(!app.import_step_hint().is_empty());

        let app = App {
            import_step: ImportStep::ConfirmPlan {
                plan: ChangePlan::new(vec![]),
                selected: Box::new(scan_result("repo-a/skill")),
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
            remove_step: RemoveStep::EnterSkill,
            ..test_app()
        };
        assert!(!app.remove_step_hint().is_empty());

        let app = App {
            remove_step: RemoveStep::Disambiguate {
                matches: vec![inventory_row("repo-a/skill")],
            },
            ..test_app()
        };
        assert!(!app.remove_step_hint().is_empty());

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
