use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::helpers;
use crate::config::{Config, expand_tilde};
use crate::domain::{AgentId, ConnectionKind, InventoryRow, SkillExposure};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSuggestion {
    pub label: &'static str,
    pub description: &'static str,
}

const COMMAND_SUGGESTIONS: [CommandSuggestion; 5] = [
    CommandSuggestion {
        label: "/list",
        description: "Show exposed skills and availability",
    },
    CommandSuggestion {
        label: "/scan",
        description: "Discover skills from configured sources",
    },
    CommandSuggestion {
        label: "/config",
        description: "Show current configuration",
    },
    CommandSuggestion {
        label: "/help",
        description: "Show commands and keybindings",
    },
    CommandSuggestion {
        label: "/quit",
        description: "Exit Skills Manager",
    },
];

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
        agents: Vec<AgentSelectionItem>,
        focused: usize,
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
    SelectExposure {
        selected: Box<InventoryRow>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingLoad {
    List,
    Scan,
}

#[derive(Debug, Clone)]
pub struct AgentSelectionItem {
    pub target: AgentTarget,
    pub checked: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableNavigation {
    pub selected: Option<usize>,
    pub viewport_offset: usize,
}

impl TableNavigation {
    pub fn reset(&mut self, row_count: usize) {
        self.viewport_offset = 0;
        self.selected = (row_count > 0).then_some(0);
    }

    pub fn sync(&mut self, row_count: usize, viewport_height: usize) {
        if row_count == 0 {
            self.selected = None;
            self.viewport_offset = 0;
            return;
        }

        let viewport_height = viewport_height.max(1);
        let selected = self.selected.unwrap_or(0).min(row_count - 1);
        let max_offset = row_count.saturating_sub(viewport_height);
        let mut offset = self.viewport_offset.min(max_offset);

        if selected < offset {
            offset = selected;
        } else if selected >= offset + viewport_height {
            offset = selected + 1 - viewport_height;
        }

        self.selected = Some(selected);
        self.viewport_offset = offset.min(max_offset);
    }

    pub fn move_up(&mut self, row_count: usize, viewport_height: usize) {
        if row_count == 0 {
            self.sync(row_count, viewport_height);
            return;
        }

        let selected = self.selected.unwrap_or(0).saturating_sub(1);
        self.selected = Some(selected);
        self.sync(row_count, viewport_height);
    }

    pub fn move_down(&mut self, row_count: usize, viewport_height: usize) {
        if row_count == 0 {
            self.sync(row_count, viewport_height);
            return;
        }

        let selected = self
            .selected
            .unwrap_or(0)
            .saturating_add(1)
            .min(row_count - 1);
        self.selected = Some(selected);
        self.sync(row_count, viewport_height);
    }
}

pub struct App {
    pub mode: Mode,
    pub input: String,
    pub inventory: Vec<InventoryRow>,
    pub scan_results: Vec<ScanResult>,
    pub status_messages: Vec<String>,
    pub list_scroll: usize,
    pub list_selected: Option<usize>,
    pub list_table: TableNavigation,
    pub scan_table: TableNavigation,
    pub config: Config,
    pub current_dir: PathBuf,
    pub git_branch: Option<String>,
    pub prompt_label: String,
    pub import_step: ImportStep,
    pub remove_step: RemoveStep,
    pub error_message: Option<String>,
    pub info_message: Option<String>,
    pub loading: bool,
    pub pending_load: Option<PendingLoad>,
    command_menu_selected: Option<usize>,
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
            list_table: TableNavigation::default(),
            scan_table: TableNavigation::default(),
            config,
            current_dir,
            git_branch: None,
            prompt_label,
            import_step: ImportStep::EnterSkill,
            remove_step: RemoveStep::EnterSkill,
            error_message: None,
            info_message: None,
            loading: false,
            pending_load: None,
            command_menu_selected: None,
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

    /// Execute a deferred load (set by handle_command for list/scan) after a loading frame renders.
    pub fn execute_pending_load(&mut self) -> anyhow::Result<()> {
        let Some(load) = self.pending_load.take() else {
            return Ok(());
        };
        self.loading = false;
        match load {
            PendingLoad::List => {
                self.refresh_inventory()?;
                self.enter_list_mode();
                self.info_message = Some(format!("Loaded {} skill row(s).", self.inventory.len()));
            }
            PendingLoad::Scan => {
                self.reload_scan_results()?;
                self.enter_scan_mode();
                self.info_message = Some(format!("Found {} skill(s).", self.scan_results.len()));
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
                self.mode = Mode::List;
                self.loading = true;
                self.pending_load = Some(PendingLoad::List);
            }
            TuiCommand::Scan => {
                self.mode = Mode::Scan;
                self.loading = true;
                self.pending_load = Some(PendingLoad::Scan);
            }
            TuiCommand::Import(skill) => {
                let suffix = if skill.trim().is_empty() {
                    ""
                } else {
                    " The typed skill name was not used."
                };
                self.info_message = Some(format!(
                    "Use table shortcuts: run /scan, select a row, then press i. From /list, press i to create missing enabled-agent exposures.{suffix}"
                ));
            }
            TuiCommand::Remove(skill) => {
                let suffix = if skill.trim().is_empty() {
                    ""
                } else {
                    " The typed skill name was not used."
                };
                self.info_message = Some(format!(
                    "Use table shortcuts: run /list, select an exposed row, then press x to remove it.{suffix}"
                ));
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
        self.list_table.reset(self.inventory.len());
        self.sync_legacy_list_navigation();
    }

    pub fn sync_legacy_list_navigation(&mut self) {
        self.list_scroll = self.list_table.viewport_offset;
        self.list_selected = self.list_table.selected;
    }

    pub fn enter_scan_mode(&mut self) {
        self.mode = Mode::Scan;
        self.scan_table.reset(self.scan_results.len());
    }

    pub fn start_import_from_selected_scan_row(&mut self) -> anyhow::Result<()> {
        self.error_message = None;
        self.info_message = None;

        let Some(selected) = self.selected_scan_result() else {
            self.info_message = Some("No scan row selected.".to_string());
            return Ok(());
        };

        let target_agents = self.enabled_agent_targets();
        self.start_import_for_scan_result(selected, target_agents);
        Ok(())
    }

    pub fn start_import_from_selected_list_row(&mut self) -> anyhow::Result<()> {
        self.error_message = None;
        self.info_message = None;

        let Some(row) = self.selected_inventory_row() else {
            self.info_message = Some("No inventory row selected.".to_string());
            return Ok(());
        };

        let target_agents = self.missing_enabled_agent_targets(&row);
        if target_agents.is_empty() {
            self.info_message =
                Some("Selected skill already has all enabled-agent exposures.".to_string());
            return Ok(());
        }

        let skill_id = display_inventory_row(&row);
        let matches = helpers::find_scan_results_by_id(&skill_id, &self.scan_results);
        match matches.len() {
            0 => {
                self.info_message =
                    Some("Selected skill has no scanned source to import from.".to_string());
            }
            1 => {
                self.start_import_for_scan_result(matches[0].clone(), target_agents);
            }
            _ => {
                self.mode = Mode::Import;
                self.import_step = ImportStep::Disambiguate {
                    matches: matches.into_iter().cloned().collect(),
                };
            }
        }

        Ok(())
    }

    pub fn start_remove_from_selected_list_row(&mut self) -> anyhow::Result<()> {
        self.error_message = None;
        self.info_message = None;

        let Some(selected) = self.selected_inventory_row() else {
            self.info_message = Some("No inventory row selected.".to_string());
            return Ok(());
        };

        let removable_exposures = Self::removable_exposures(&selected);
        self.mode = Mode::Remove;
        match removable_exposures.len() {
            0 => {
                self.remove_step = RemoveStep::Done {
                    message: "Selected row has no removable exposures.".to_string(),
                };
                self.info_message = Some("Selected row has no removable exposures.".to_string());
            }
            1 => {
                let plan = self.build_remove_plan_for_exposure(&selected, &removable_exposures[0]);
                self.remove_step = RemoveStep::ConfirmPlan {
                    plan,
                    selected: Box::new(selected),
                };
            }
            _ => {
                self.remove_step = RemoveStep::SelectExposure {
                    selected: Box::new(selected),
                };
            }
        }

        Ok(())
    }

    pub fn refresh_active_table(&mut self) -> anyhow::Result<()> {
        self.error_message = None;
        self.info_message = None;

        match self.mode {
            Mode::List => {
                self.refresh_inventory()?;
                self.enter_list_mode();
                self.info_message = Some(format!("Loaded {} skill row(s).", self.inventory.len()));
            }
            Mode::Scan => {
                self.reload_scan_results()?;
                self.enter_scan_mode();
                self.info_message = Some(format!("Found {} skill(s).", self.scan_results.len()));
            }
            _ => {}
        }

        Ok(())
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
                        let selected = Box::new(matches[0].clone());
                        let target_agents = self.enabled_agent_targets();
                        if target_agents.is_empty() {
                            self.import_step = ImportStep::Done {
                                message: "No enabled agents available.".to_string(),
                            };
                            self.info_message = Some("No enabled agents available.".to_string());
                        } else if target_agents.len() == 1 {
                            let plan = self.build_import_plan(&selected, &target_agents);
                            self.import_step = if plan.is_empty() {
                                self.info_message = Some("Nothing to do.".to_string());
                                ImportStep::Done {
                                    message: "Nothing to do.".to_string(),
                                }
                            } else {
                                ImportStep::ConfirmPlan {
                                    plan,
                                    selected,
                                    target_agents,
                                }
                            };
                        } else {
                            self.import_step = ImportStep::SelectAgents {
                                selected,
                                agents: target_agents
                                    .into_iter()
                                    .map(|t| AgentSelectionItem {
                                        target: t,
                                        checked: true,
                                    })
                                    .collect(),
                                focused: 0,
                            };
                        }
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
                    let selected = Box::new(matches[index].clone());
                    let target_agents = self.enabled_agent_targets();
                    if target_agents.is_empty() {
                        self.import_step = ImportStep::Done {
                            message: "No enabled agents available.".to_string(),
                        };
                        self.info_message = Some("No enabled agents available.".to_string());
                    } else if target_agents.len() == 1 {
                        let plan = self.build_import_plan(&selected, &target_agents);
                        self.import_step = if plan.is_empty() {
                            self.info_message = Some("Nothing to do.".to_string());
                            ImportStep::Done {
                                message: "Nothing to do.".to_string(),
                            }
                        } else {
                            ImportStep::ConfirmPlan {
                                plan,
                                selected,
                                target_agents,
                            }
                        };
                    } else {
                        self.import_step = ImportStep::SelectAgents {
                            selected,
                            agents: target_agents
                                .into_iter()
                                .map(|t| AgentSelectionItem {
                                    target: t,
                                    checked: true,
                                })
                                .collect(),
                            focused: 0,
                        };
                    }
                }
                None => {
                    self.error_message =
                        Some(format!("Enter a number between 1 and {}", matches.len()));
                    self.import_step = ImportStep::Disambiguate { matches };
                }
            },
            ImportStep::SelectAgents {
                selected,
                agents,
                focused,
            } => {
                let target_agents: Vec<AgentTarget> = agents
                    .iter()
                    .filter(|item| item.checked)
                    .map(|item| item.target.clone())
                    .collect();
                if target_agents.is_empty() {
                    self.error_message =
                        Some("Select at least one agent. Use Space to toggle.".to_string());
                    self.import_step = ImportStep::SelectAgents {
                        selected,
                        agents,
                        focused,
                    };
                    return Ok(());
                }
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
            RemoveStep::SelectExposure { selected } => {
                let removable_exposures = Self::removable_exposures(&selected);
                match parse_selection(input, removable_exposures.len()) {
                    Some(index) => {
                        let plan = self
                            .build_remove_plan_for_exposure(&selected, &removable_exposures[index]);
                        self.remove_step = RemoveStep::ConfirmPlan { plan, selected };
                    }
                    None => {
                        self.error_message = Some(format!(
                            "Enter a number between 1 and {}",
                            removable_exposures.len()
                        ));
                        self.remove_step = RemoveStep::SelectExposure { selected };
                    }
                }
            }
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
                "Up/Down to move, Space to toggle, Enter to confirm:"
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
            RemoveStep::SelectExposure { .. } => "Enter exposure number to remove:",
            RemoveStep::ConfirmPlan { .. } => "Apply this plan? [y/N]:",
            RemoveStep::ConfirmPhysical { .. } => "Type 'yes' to confirm permanent deletion:",
            RemoveStep::Done { .. } => "Press Enter to return to home.",
        }
    }

    pub fn move_agent_selection_up(&mut self) {
        if let ImportStep::SelectAgents { focused, .. } = &mut self.import_step {
            *focused = focused.saturating_sub(1);
        }
    }

    pub fn move_agent_selection_down(&mut self) {
        if let ImportStep::SelectAgents {
            agents, focused, ..
        } = &mut self.import_step
        {
            *focused = (*focused + 1).min(agents.len().saturating_sub(1));
        }
    }

    pub fn toggle_agent_selection(&mut self) {
        if let ImportStep::SelectAgents {
            agents, focused, ..
        } = &mut self.import_step
        {
            if let Some(item) = agents.get_mut(*focused) {
                item.checked = !item.checked;
            }
        }
    }

    fn reload_scan_results(&mut self) -> anyhow::Result<()> {
        self.scan_results =
            scanner::scan(&helpers::scan_config_from(&self.config, &self.current_dir))?;
        scanner::assign_disambiguation_indices(&mut self.scan_results);
        self.rebuild_status_messages();
        Ok(())
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

    fn selected_scan_result(&self) -> Option<ScanResult> {
        self.scan_table
            .selected
            .and_then(|index| self.scan_results.get(index))
            .cloned()
    }

    fn selected_inventory_row(&self) -> Option<InventoryRow> {
        self.list_table
            .selected
            .and_then(|index| self.inventory.get(index))
            .cloned()
    }

    fn enabled_agent_targets(&self) -> Vec<AgentTarget> {
        helpers::agent_targets_from(&self.config, &self.current_dir)
            .into_iter()
            .filter(|agent| agent.enabled)
            .collect()
    }

    fn missing_enabled_agent_targets(&self, row: &InventoryRow) -> Vec<AgentTarget> {
        let exposed_agent_ids = row
            .exposures
            .iter()
            .map(|exposure| exposure.agent_id.0.clone())
            .collect::<HashSet<_>>();

        self.enabled_agent_targets()
            .into_iter()
            .filter(|agent| !exposed_agent_ids.contains(&agent.agent_id))
            .collect()
    }

    fn start_import_for_scan_result(
        &mut self,
        selected: ScanResult,
        target_agents: Vec<AgentTarget>,
    ) {
        self.mode = Mode::Import;

        if target_agents.is_empty() {
            self.import_step = ImportStep::Done {
                message: "No enabled agents available.".to_string(),
            };
            self.info_message = Some("No enabled agents available.".to_string());
            return;
        }

        if target_agents.len() == 1 {
            let plan = self.build_import_plan(&selected, &target_agents);
            self.import_step = if plan.is_empty() {
                self.info_message = Some("Nothing to do.".to_string());
                ImportStep::Done {
                    message: "Nothing to do.".to_string(),
                }
            } else {
                ImportStep::ConfirmPlan {
                    plan,
                    selected: Box::new(selected),
                    target_agents,
                }
            };
            return;
        }

        self.import_step = ImportStep::SelectAgents {
            selected: Box::new(selected),
            agents: target_agents
                .into_iter()
                .map(|target| AgentSelectionItem {
                    target,
                    checked: true,
                })
                .collect(),
            focused: 0,
        };
    }

    fn build_remove_plan(&self, row: &InventoryRow) -> ChangePlan {
        let changes = Self::removable_exposures(row)
            .iter()
            .flat_map(|exposure| self.build_remove_plan_for_exposure(row, exposure).changes)
            .collect();
        ChangePlan::new(changes)
    }

    fn build_remove_plan_for_exposure(
        &self,
        row: &InventoryRow,
        exposure: &SkillExposure,
    ) -> ChangePlan {
        let skill_name = display_inventory_row(row);
        let display_name = self
            .config
            .agents
            .get(&exposure.agent_id.0)
            .map(|agent| agent.display_name.clone())
            .unwrap_or_else(|| exposure.agent_id.0.clone());
        let change = match exposure.connection {
            ConnectionKind::Symlink => Some(StagedChange::DetachSkill {
                skill_name,
                agent_id: AgentId(display_name),
                target_path: exposure.path.clone(),
            }),
            ConnectionKind::PhysicalCopy => Some(StagedChange::DeletePhysicalCopy {
                skill_name,
                agent_id: AgentId(display_name),
                target_path: exposure.path.clone(),
            }),
            ConnectionKind::Missing | ConnectionKind::Unknown => None,
        };
        ChangePlan::new(change.into_iter().collect())
    }

    fn removable_exposures(row: &InventoryRow) -> Vec<SkillExposure> {
        row.exposures
            .iter()
            .filter(|exposure| {
                matches!(
                    exposure.connection,
                    ConnectionKind::Symlink | ConnectionKind::PhysicalCopy
                )
            })
            .cloned()
            .collect()
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
    fn enter_list_mode_selects_first_row_when_inventory_exists() {
        let mut app = test_app();
        app.inventory = vec![inventory_row("repo-a/one"), inventory_row("repo-a/two")];

        app.enter_list_mode();

        assert_eq!(app.mode, Mode::List);
        assert_eq!(app.list_table.selected, Some(0));
        assert_eq!(app.list_table.viewport_offset, 0);
    }

    #[test]
    fn enter_scan_mode_selects_first_result_when_results_exist() {
        let mut app = test_app();
        app.scan_results = vec![scan_result("repo-a/one"), scan_result("repo-a/two")];

        app.enter_scan_mode();

        assert_eq!(app.mode, Mode::Scan);
        assert_eq!(app.scan_table.selected, Some(0));
        assert_eq!(app.scan_table.viewport_offset, 0);
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
    fn handle_command_import_guides_to_table_shortcut() {
        let mut app = test_app();

        app.handle_command("/import repo-a/skill")
            .expect("command succeeds");

        assert_eq!(app.mode, Mode::Home);
        assert!(matches!(app.import_step, ImportStep::EnterSkill));
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
        assert!(matches!(app.remove_step, RemoveStep::EnterSkill));
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

        assert_eq!(labels, vec!["/list", "/scan", "/config", "/help", "/quit"]);
        assert!(
            app.filtered_command_suggestions()
                .iter()
                .all(|suggestion| !suggestion.description.is_empty())
        );
    }

    #[test]
    fn command_suggestions_filter_by_command_text() {
        let mut app = test_app();
        app.input = "/sc".to_string();
        app.open_command_menu();

        let labels = app
            .filtered_command_suggestions()
            .iter()
            .map(|suggestion| suggestion.label)
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["/scan"]);
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
            Some("/scan")
        );

        app.move_command_suggestion_up();

        assert_eq!(
            app.selected_command_suggestion().map(|item| item.label),
            Some("/list")
        );
    }

    #[test]
    fn table_navigation_selects_first_row_when_rows_exist() {
        let mut nav = TableNavigation::default();

        nav.reset(3);

        assert_eq!(nav.selected, Some(0));
        assert_eq!(nav.viewport_offset, 0);
    }

    #[test]
    fn table_navigation_keeps_viewport_still_before_bottom() {
        let mut nav = TableNavigation::default();
        nav.reset(5);

        nav.move_down(5, 3);
        nav.move_down(5, 3);

        assert_eq!(nav.selected, Some(2));
        assert_eq!(nav.viewport_offset, 0);
    }

    #[test]
    fn table_navigation_scrolls_after_selection_moves_past_bottom() {
        let mut nav = TableNavigation::default();
        nav.reset(5);

        nav.move_down(5, 3);
        nav.move_down(5, 3);
        nav.move_down(5, 3);

        assert_eq!(nav.selected, Some(3));
        assert_eq!(nav.viewport_offset, 1);
    }

    #[test]
    fn table_navigation_scrolls_after_selection_moves_past_top() {
        let mut nav = TableNavigation {
            selected: Some(2),
            viewport_offset: 2,
        };

        nav.move_up(5, 3);

        assert_eq!(nav.selected, Some(1));
        assert_eq!(nav.viewport_offset, 1);
    }

    #[test]
    fn table_navigation_clears_selection_for_empty_rows() {
        let mut nav = TableNavigation {
            selected: Some(2),
            viewport_offset: 1,
        };

        nav.sync(0, 3);

        assert_eq!(nav.selected, None);
        assert_eq!(nav.viewport_offset, 0);
    }

    #[test]
    fn table_navigation_clamps_viewport_after_resize() {
        let mut nav = TableNavigation {
            selected: Some(4),
            viewport_offset: 3,
        };

        nav.sync(5, 4);

        assert_eq!(nav.selected, Some(4));
        assert_eq!(nav.viewport_offset, 1);
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
    fn move_agent_selection_up_decrements_focus() {
        let mut app = test_app();
        app.import_step = ImportStep::SelectAgents {
            selected: Box::new(scan_result("repo-a/skill")),
            agents: vec![
                AgentSelectionItem {
                    target: crate::inventory::AgentTarget {
                        agent_id: "claude".to_string(),
                        display_name: "Claude".to_string(),
                        global_dir: None,
                        project_dir: None,
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
                        project_dir: None,
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
            selected: Box::new(scan_result("repo-a/skill")),
            agents: vec![
                AgentSelectionItem {
                    target: crate::inventory::AgentTarget {
                        agent_id: "claude".to_string(),
                        display_name: "Claude".to_string(),
                        global_dir: None,
                        project_dir: None,
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
                        project_dir: None,
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
            selected: Box::new(scan_result("repo-a/skill")),
            agents: vec![AgentSelectionItem {
                target: crate::inventory::AgentTarget {
                    agent_id: "claude".to_string(),
                    display_name: "Claude".to_string(),
                    global_dir: None,
                    project_dir: None,
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
                agents: vec![],
                focused: 0,
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
