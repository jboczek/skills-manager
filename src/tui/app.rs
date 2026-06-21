use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::commands::helpers;
use crate::config::{Config, GlobalContext};
use crate::domain::{AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure};
use crate::inventory::{self, AgentTarget};
use crate::plan::{ChangePlan, StagedChange};
use crate::plan_apply;
use crate::scanner::{self, ScanResult};
use crate::source::{self, AcquireOutcome, SourcePreview};
use crate::tui::source_table::{SourceGroupItem, SourceTable, SourceTableRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiCommand {
    List,
    Scan,
    SourceAdd(String),
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

const COMMAND_SUGGESTIONS: [CommandSuggestion; 6] = [
    CommandSuggestion {
        label: "/list",
        description: "Show exposed skills and availability",
    },
    CommandSuggestion {
        label: "/scan",
        description: "Discover skills from configured sources",
    },
    CommandSuggestion {
        label: "/source_add",
        description: "Add new skills from Git repository using HTTPS/SSH clone URL",
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
/// Accepts "list", "/list", "scan", "/scan", "source_add <git-url>", etc.
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
        "source_add" => TuiCommand::SourceAdd(argument),
        "import" => TuiCommand::Import(argument),
        "remove" => TuiCommand::Remove(argument),
        "config" => TuiCommand::Config,
        "help" | "?" => TuiCommand::Help,
        "q" | "quit" => TuiCommand::Quit,
        _ => TuiCommand::Unknown(trimmed.to_string()),
    }
}

#[derive(Debug, Clone, Default)]
pub enum SourceAddStep {
    #[default]
    EnterUrl,
    Confirm {
        preview: SourcePreview,
    },
    SelectSkill {
        source_path: PathBuf,
        skills: Vec<ScanResult>,
        outcome: AcquireOutcome,
    },
    Done {
        message: String,
    },
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
    SourceAdd,
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

pub struct App {
    pub mode: Mode,
    pub input: String,
    pub inventory: Vec<InventoryRow>,
    pub scan_results: Vec<ScanResult>,
    pub status_messages: Vec<String>,
    pub list_table: SourceTable<usize>,
    pub scan_table: SourceTable<usize>,
    pub config: Config,
    pub global_context: GlobalContext,
    pub prompt_label: String,
    pub source_add_step: SourceAddStep,
    pub import_step: ImportStep,
    pub remove_step: RemoveStep,
    pub error_message: Option<String>,
    pub info_message: Option<String>,
    pub loading: bool,
    pub pending_load: Option<PendingLoad>,
    command_menu_selected: Option<usize>,
}

impl App {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let global_context = config.resolve_global_context()?;
        Ok(Self {
            mode: Mode::Home,
            input: String::new(),
            inventory: Vec::new(),
            scan_results: Vec::new(),
            status_messages: Vec::new(),
            list_table: SourceTable::default(),
            scan_table: SourceTable::default(),
            config,
            global_context,
            prompt_label: "Skills".to_string(),
            source_add_step: SourceAddStep::EnterUrl,
            import_step: ImportStep::EnterSkill,
            remove_step: RemoveStep::EnterSkill,
            error_message: None,
            info_message: None,
            loading: false,
            pending_load: None,
            command_menu_selected: None,
        })
    }

    /// Load initial global scan and inventory state.
    pub fn initialize(&mut self) -> anyhow::Result<()> {
        self.reload_scan_results()?;
        self.refresh_inventory()?;
        self.rebuild_status_messages();
        Ok(())
    }

    /// Refresh inventory from filesystem.
    pub fn refresh_inventory(&mut self) -> anyhow::Result<()> {
        self.inventory = helpers::fresh_global_inventory(&self.global_context)?;
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

    pub fn advance_source_add(&mut self, input: &str) -> anyhow::Result<()> {
        self.error_message = None;
        match self.source_add_step.clone() {
            SourceAddStep::EnterUrl => {
                self.mode = Mode::Home;
            }
            SourceAddStep::Confirm { preview } => {
                let normalized = input.trim().to_ascii_lowercase();
                if normalized == "y" {
                    match source::acquire(&preview, self.global_context.max_scan_depth) {
                        Ok(mut acquired) => {
                            acquired
                                .skills
                                .sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
                            self.reload_scan_results()?;
                            self.refresh_inventory()?;
                            self.info_message = Some(match acquired.outcome {
                                AcquireOutcome::Cloned => {
                                    format!("Added source at {}.", acquired.path.display())
                                }
                                AcquireOutcome::Reused => {
                                    format!("Reused source at {}.", acquired.path.display())
                                }
                            });
                            self.source_add_step = SourceAddStep::SelectSkill {
                                source_path: acquired.path,
                                skills: acquired.skills,
                                outcome: acquired.outcome,
                            };
                        }
                        Err(error) => {
                            self.error_message = Some(error.to_string());
                            self.source_add_step = SourceAddStep::Done {
                                message: "Source was not added.".to_string(),
                            };
                        }
                    }
                } else if normalized == "n" || normalized.is_empty() {
                    self.source_add_step = SourceAddStep::Done {
                        message: "Aborted.".to_string(),
                    };
                } else {
                    self.error_message = Some("Add this source? [y/N]".to_string());
                    self.source_add_step = SourceAddStep::Confirm { preview };
                }
            }
            SourceAddStep::SelectSkill {
                source_path,
                skills,
                outcome,
            } => {
                if input.trim().is_empty() {
                    self.source_add_step = SourceAddStep::Done {
                        message: "Source kept without new exposures.".to_string(),
                    };
                    return Ok(());
                }
                let Some(index) = parse_selection(input, skills.len()) else {
                    self.error_message =
                        Some(format!("Enter a number between 1 and {}", skills.len()));
                    self.source_add_step = SourceAddStep::SelectSkill {
                        source_path,
                        skills,
                        outcome,
                    };
                    return Ok(());
                };
                let selected = skills[index].clone();
                let target_agents = self.enabled_agent_targets();
                self.start_import_for_scan_result(selected, target_agents);
            }
            SourceAddStep::Done { .. } => {
                self.mode = Mode::Home;
                self.source_add_step = SourceAddStep::EnterUrl;
            }
        }

        Ok(())
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
        self.list_table = SourceTable::new(self.list_table_items());
    }

    pub fn enter_scan_mode(&mut self) {
        self.mode = Mode::Scan;
        self.scan_table = SourceTable::new(self.scan_table_items());
    }

    pub fn start_import_from_selected_scan_row(&mut self) -> anyhow::Result<()> {
        self.error_message = None;
        self.info_message = None;

        let Some(selected) = self.selected_scan_result() else {
            self.info_message = Some(self.selection_required_message(&self.scan_table));
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
            self.info_message = Some(self.selection_required_message(&self.list_table));
            return Ok(());
        };
        if row.scope == Scope::ProjectLocal {
            self.info_message =
                Some("Project-local exposures are read-only and cannot be imported.".to_string());
            return Ok(());
        }

        let target_agents = self.missing_enabled_agent_targets(&row);
        if target_agents.is_empty() {
            self.info_message =
                Some("Selected skill already has all enabled-agent exposures.".to_string());
            return Ok(());
        }

        let skill_id = display_inventory_row(&row);
        if let Some(selected) = self.scan_result_for_inventory_row(&row).cloned() {
            self.start_import_for_scan_result(selected, target_agents);
            return Ok(());
        }
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
            self.info_message = Some(self.selection_required_message(&self.list_table));
            return Ok(());
        };
        if selected.scope == Scope::ProjectLocal {
            self.info_message =
                Some("Project-local exposures are read-only and cannot be removed.".to_string());
            return Ok(());
        }

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

    pub fn refresh_active_table(&mut self, viewport_height: usize) -> anyhow::Result<()> {
        self.error_message = None;
        self.info_message = None;

        match self.mode {
            Mode::List => {
                self.refresh_inventory()?;
                let items = self.list_table_items();
                self.list_table.refresh(items, viewport_height);
                self.info_message = Some(format!("Loaded {} skill row(s).", self.inventory.len()));
            }
            Mode::Scan => {
                self.reload_scan_results()?;
                let items = self.scan_table_items();
                self.scan_table.refresh(items, viewport_height);
                self.info_message = Some(format!("Found {} skill(s).", self.scan_results.len()));
            }
            _ => {}
        }

        Ok(())
    }

    pub fn sync_active_table(&mut self, viewport_height: usize) {
        match self.mode {
            Mode::List => self.list_table.sync(viewport_height),
            Mode::Scan => self.scan_table.sync(viewport_height),
            _ => {}
        }
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
                        self.apply_plan_and_refresh(&plan)?;
                        self.mode = Mode::Home;
                        self.import_step = ImportStep::EnterSkill;
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
                    self.apply_plan_and_refresh(&plan)?;
                    self.mode = Mode::Home;
                    self.import_step = ImportStep::EnterSkill;
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

    pub fn source_add_step_hint(&self) -> &'static str {
        match self.source_add_step {
            SourceAddStep::EnterUrl => "Enter /source_add <git-url>:",
            SourceAddStep::Confirm { .. } => "Add this source? [y/N]:",
            SourceAddStep::SelectSkill { .. } => {
                "Enter skill number to expose, or Enter to keep source only:"
            }
            SourceAddStep::Done { .. } => "Press Enter to return to home.",
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
            && let Some(item) = agents.get_mut(*focused)
        {
            item.checked = !item.checked;
        }
    }

    fn reload_scan_results(&mut self) -> anyhow::Result<()> {
        self.scan_results = scanner::scan(&helpers::scan_config_from_global(&self.global_context))?;
        scanner::exclude_dot_directory_results(&mut self.scan_results);
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
        match self.scan_table.selected_row()? {
            SourceTableRow::Item { item, .. } => self.scan_results.get(item).cloned(),
            SourceTableRow::Group { .. } => None,
        }
    }

    pub(crate) fn selected_inventory_row(&self) -> Option<InventoryRow> {
        match self.list_table.selected_row()? {
            SourceTableRow::Item { item, .. } => self.inventory.get(item).cloned(),
            SourceTableRow::Group { .. } => None,
        }
    }

    fn selection_required_message(&self, table: &SourceTable<usize>) -> String {
        if matches!(table.selected_row(), Some(SourceTableRow::Group { .. })) {
            "Select a skill inside the group.".to_string()
        } else {
            "No skill row selected.".to_string()
        }
    }

    fn scan_table_items(&self) -> Vec<SourceGroupItem<usize>> {
        self.scan_results
            .iter()
            .enumerate()
            .map(|(index, result)| SourceGroupItem {
                item: index,
                skill_name: scan_skill_label(result),
                skill_path: result.skill_path.clone(),
                repo_name: result.repo_name.clone(),
                repo_path: result.repo_path.clone(),
                relative_path: result.skill_relative_path.clone(),
            })
            .collect()
    }

    fn list_table_items(&self) -> Vec<SourceGroupItem<usize>> {
        self.inventory
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let skill_path = source_path_for_inventory_row(row);
                let (repo_name, repo_path, relative_path) = match row.scope {
                    Scope::Global => {
                        let relative_path = row
                            .source
                            .repo_path
                            .as_ref()
                            .and_then(|root| skill_path.strip_prefix(root).ok())
                            .map(Path::to_path_buf);
                        (
                            row.source.repo_name.clone(),
                            row.source.repo_path.clone(),
                            relative_path,
                        )
                    }
                    Scope::ProjectLocal => {
                        let project_root = row.exposures.first().and_then(|exposure| {
                            inventory::project_root_from_exposure_path(&exposure.path)
                        });
                        let repo_name = project_root
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .map(|name| name.to_string_lossy().into_owned())
                            .or_else(|| Some("project-local".to_string()));
                        let relative_path = project_root
                            .as_ref()
                            .and_then(|root| skill_path.strip_prefix(root).ok())
                            .map(Path::to_path_buf);
                        (repo_name, project_root, relative_path)
                    }
                };
                SourceGroupItem {
                    item: index,
                    skill_name: inventory_skill_label(row),
                    skill_path,
                    repo_name,
                    repo_path,
                    relative_path,
                }
            })
            .collect()
    }

    fn scan_result_for_inventory_row(&self, row: &InventoryRow) -> Option<&ScanResult> {
        let display_id = display_inventory_row(row);
        let local_index = self
            .inventory
            .iter()
            .filter(|candidate| {
                display_inventory_row(candidate) == display_id
                    && candidate.source.repo_path == row.source.repo_path
            })
            .position(|candidate| candidate == row)
            .unwrap_or(0);
        let matches = self
            .scan_results
            .iter()
            .filter(|result| {
                result.skill_id == display_id && result.repo_path == row.source.repo_path
            })
            .collect::<Vec<_>>();
        matches
            .get(local_index)
            .copied()
            .or_else(|| matches.first().copied())
    }

    fn enabled_agent_targets(&self) -> Vec<AgentTarget> {
        helpers::agent_targets_from_global(&self.global_context)
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
        if row.scope == Scope::ProjectLocal {
            return ChangePlan::new(Vec::new());
        }
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
        if row.scope == Scope::ProjectLocal {
            return Vec::new();
        }
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
                "• Global context: {} skills, {} agents",
                self.inventory.len().max(self.scan_results.len()),
                self.global_context
                    .agents
                    .iter()
                    .filter(|agent| agent.enabled)
                    .count()
            ),
            "• Scan: OK".to_string(),
        ];
        self.status_messages.extend(
            self.global_context
                .diagnostics
                .iter()
                .map(|diagnostic| format!("• Warning: {diagnostic}")),
        );
    }
}

fn source_path_for_inventory_row(row: &InventoryRow) -> PathBuf {
    let Some(exposure) = row.exposures.first() else {
        return row
            .source
            .repo_path
            .as_ref()
            .map(|path| path.join(&row.skill_id.name))
            .unwrap_or_else(|| PathBuf::from(&row.skill_id.name));
    };
    if exposure.connection == ConnectionKind::Symlink {
        return fs::canonicalize(&exposure.path).unwrap_or_else(|_| exposure.path.clone());
    }
    exposure.path.clone()
}

fn display_inventory_row(row: &InventoryRow) -> String {
    if row.skill_id.namespace.is_empty() {
        row.skill_id.name.clone()
    } else {
        format!("{}/{}", row.skill_id.namespace, row.skill_id.name)
    }
}

fn inventory_skill_label(row: &InventoryRow) -> String {
    match row.disambiguation_index {
        Some(index) => format!("({index}) {}", row.skill_id.name),
        None => row.skill_id.name.clone(),
    }
}

fn scan_skill_label(result: &ScanResult) -> String {
    let name = result
        .skill_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            result
                .skill_id
                .rsplit('/')
                .next()
                .unwrap_or(&result.skill_id)
                .to_string()
        });
    match result.disambiguation_index {
        Some(index) => format!("({index}) {name}"),
        None => name,
    }
}

fn parse_selection(input: &str, max: usize) -> Option<usize> {
    let index = input.trim().parse::<usize>().ok()?;
    (1..=max).contains(&index).then_some(index - 1)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

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
    fn parse_command_import_with_arg() {
        assert_eq!(
            parse_command("import repo-a/skill"),
            TuiCommand::Import("repo-a/skill".to_string())
        );
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
    fn enter_scan_mode_selects_first_result_when_results_exist() {
        let mut app = test_app();
        app.scan_results = vec![scan_result("repo-a/one"), scan_result("repo-a/two")];

        app.enter_scan_mode();

        assert_eq!(app.mode, Mode::Scan);
        assert_eq!(app.scan_table.selected_index(), Some(0));
        assert_eq!(app.scan_table.viewport_offset(), 0);
        assert_eq!(app.scan_table.visible_rows().len(), 1);
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
    fn list_and_scan_group_global_rows_by_source_repository() {
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
        let list_key = app.list_table.groups()[0].key.clone();
        app.list_table.move_right(4);
        let list_path = match &app.list_table.visible_rows()[1] {
            SourceTableRow::Item { display_path, .. } => display_path.clone(),
            _ => panic!("expected list child row"),
        };

        app.enter_scan_mode();
        let scan_key = app.scan_table.groups()[0].key.clone();
        app.scan_table.move_right(4);
        let scan_path = match &app.scan_table.visible_rows()[1] {
            SourceTableRow::Item { display_path, .. } => display_path.clone(),
            _ => panic!("expected scan child row"),
        };

        assert_eq!(list_key, scan_key);
        assert_eq!(list_path, ".agents/skills/one");
        assert_eq!(scan_path, list_path);
        assert!(!scan_path.contains("alice"));
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
        assert_eq!(selected.skill_id, "remote-skills/docs");
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

        assert_eq!(
            labels,
            vec!["/list", "/scan", "/source_add", "/config", "/help", "/quit"]
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
            Some("/scan")
        );

        app.move_command_suggestion_up();

        assert_eq!(
            app.selected_command_suggestion().map(|item| item.label),
            Some("/list")
        );
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
                target_path: target,
                connection: ConnectionKind::Symlink,
            }]),
            selected: Box::new(scan_result("repo-a/skill")),
            target_agents: vec![],
        };

        app.advance_import("y").expect("import applies");

        assert_eq!(app.mode, Mode::Home);
        assert!(matches!(app.import_step, ImportStep::EnterSkill));
        assert_eq!(app.info_message.as_deref(), Some("Applied 1 change(s)."));
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
            selected: Box::new(scan_result("repo-a/skill")),
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
            selected: Box::new(scan_result("repo-a/skill")),
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
