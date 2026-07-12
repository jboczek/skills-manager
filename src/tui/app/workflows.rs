use std::collections::HashSet;
use std::fs;

use super::tables::{display_inventory_row, parse_selection};
use super::{AgentSelectionItem, App, ImportStep, Mode, RemoveStep, SourceAddStep};
use crate::commands::helpers;
use crate::domain::{AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure};
use crate::inventory::AgentTarget;
use crate::plan::{ChangePlan, StagedChange};
use crate::plan_apply;
use crate::scanner::ScanResult;
use crate::source::{self, AcquireOutcome};

impl App {
    pub fn advance_source_add(&mut self, input: &str) -> anyhow::Result<()> {
        self.error_message = None;
        match self.source_add_step.clone() {
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
                self.source_add_step = SourceAddStep::default();
            }
        }

        Ok(())
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

        let rows = self.actionable_inventory_rows();
        if rows.is_empty() {
            self.info_message = Some(self.selection_required_message(&self.list_table));
            return Ok(());
        }
        if rows.len() == 1 {
            self.start_import_for_inventory_row(rows.into_iter().next().unwrap());
        } else {
            self.start_import_for_inventory_rows(rows);
        }

        Ok(())
    }

    fn start_import_for_inventory_row(&mut self, row: InventoryRow) {
        if row.scope == Scope::ProjectLocal {
            self.info_message =
                Some("Project-local exposures are read-only and cannot be imported.".to_string());
            return;
        }

        let target_agents = self.missing_enabled_agent_targets(&row);
        if target_agents.is_empty() {
            self.info_message =
                Some("Selected skill already has all enabled-agent exposures.".to_string());
            return;
        }

        let skill_id = display_inventory_row(&row);
        if let Some(selected) = self.scan_result_for_inventory_row(&row).cloned() {
            self.start_import_for_scan_result(selected, target_agents);
            return;
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
    }

    fn start_import_for_inventory_rows(&mut self, rows: Vec<InventoryRow>) {
        if rows.iter().any(|row| row.scope == Scope::ProjectLocal) {
            self.info_message =
                Some("Project-local exposures are read-only and cannot be imported.".to_string());
            return;
        }

        let mut changes = Vec::new();
        let mut first_selected = None;
        for row in rows {
            let target_agents = self.missing_enabled_agent_targets(&row);
            if target_agents.is_empty() {
                continue;
            }
            let Some(selected) = self.scan_result_for_import_row(&row) else {
                continue;
            };
            first_selected.get_or_insert_with(|| selected.clone());
            changes.extend(self.build_import_plan(&selected, &target_agents).changes);
        }

        self.mode = Mode::Import;
        let plan = ChangePlan::new(changes);
        if plan.is_empty() {
            self.import_step = ImportStep::Done {
                message: "Nothing to do.".to_string(),
            };
            self.info_message = Some("Nothing to do.".to_string());
        } else if let Some(selected) = first_selected {
            self.import_step = ImportStep::ConfirmPlan {
                plan,
                selected: Box::new(selected),
                target_agents: Vec::new(),
            };
        }
    }

    fn scan_result_for_import_row(&self, row: &InventoryRow) -> Option<ScanResult> {
        if let Some(selected) = self.scan_result_for_inventory_row(row) {
            return Some(selected.clone());
        }
        let skill_id = display_inventory_row(row);
        let matches = helpers::find_scan_results_by_id(&skill_id, &self.scan_results);
        (matches.len() == 1).then(|| matches[0].clone())
    }

    fn actionable_inventory_rows(&self) -> Vec<InventoryRow> {
        let checked = self.checked_inventory_rows();
        if checked.is_empty() {
            self.selected_inventory_row().into_iter().collect()
        } else {
            checked
        }
    }

    pub fn start_remove_from_selected_list_row(&mut self) -> anyhow::Result<()> {
        self.error_message = None;
        self.info_message = None;

        let rows = self.actionable_inventory_rows();
        if rows.is_empty() {
            self.info_message = Some(self.selection_required_message(&self.list_table));
            return Ok(());
        }
        if rows.len() == 1 {
            self.start_remove_for_inventory_row(rows.into_iter().next().unwrap());
        } else {
            self.start_remove_for_inventory_rows(rows);
        }

        Ok(())
    }

    fn start_remove_for_inventory_row(&mut self, selected: InventoryRow) {
        if selected.scope == Scope::ProjectLocal {
            self.info_message =
                Some("Project-local exposures are read-only and cannot be removed.".to_string());
            return;
        }

        let removable_exposures = removable_exposures(&selected);
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
    }

    fn start_remove_for_inventory_rows(&mut self, rows: Vec<InventoryRow>) {
        if rows.iter().any(|row| row.scope == Scope::ProjectLocal) {
            self.info_message =
                Some("Project-local exposures are read-only and cannot be removed.".to_string());
            return;
        }

        let Some(first) = rows.first().cloned() else {
            return;
        };
        let changes = rows
            .iter()
            .flat_map(|row| {
                removable_exposures(row)
                    .iter()
                    .flat_map(|exposure| self.build_remove_plan_for_exposure(row, exposure).changes)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        self.mode = Mode::Remove;
        let plan = ChangePlan::new(changes);
        if plan.is_empty() {
            self.remove_step = RemoveStep::Done {
                message: "Selected rows have no removable exposures.".to_string(),
            };
            self.info_message = Some("Selected rows have no removable exposures.".to_string());
        } else {
            self.remove_step = RemoveStep::ConfirmPlan {
                plan,
                selected: Box::new(first),
            };
        }
    }

    /// Handle import flow step progression.
    pub fn advance_import(&mut self, input: &str) -> anyhow::Result<()> {
        match self.import_step.clone() {
            ImportStep::Disambiguate { matches } => match parse_selection(input, matches.len()) {
                Some(index) => {
                    let target_agents = self.enabled_agent_targets();
                    self.start_import_for_scan_result(matches[index].clone(), target_agents);
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
                        self.mode = Mode::Home;
                        self.import_step = ImportStep::Done { message };
                    }
                } else if normalized == "n" || normalized.is_empty() {
                    self.mode = Mode::Home;
                    self.info_message = Some("Aborted.".to_string());
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
                    self.mode = Mode::Home;
                    self.import_step = ImportStep::Done { message };
                } else {
                    self.mode = Mode::Home;
                    self.info_message = Some("Aborted.".to_string());
                    self.import_step = ImportStep::Done {
                        message: "Aborted.".to_string(),
                    };
                }
            }
            ImportStep::Done { .. } => {
                self.mode = Mode::Home;
                self.import_step = ImportStep::default();
            }
        }

        Ok(())
    }

    /// Handle remove flow step progression.
    pub fn advance_remove(&mut self, input: &str) -> anyhow::Result<()> {
        match self.remove_step.clone() {
            RemoveStep::SelectExposure { selected } => {
                let removable_exposures = removable_exposures(&selected);
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
                        self.mode = Mode::Home;
                        self.remove_step = RemoveStep::Done { message };
                    }
                } else if normalized == "n" || normalized.is_empty() {
                    self.mode = Mode::Home;
                    self.info_message = Some("Aborted.".to_string());
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
                    self.mode = Mode::Home;
                    self.remove_step = RemoveStep::Done { message };
                } else {
                    self.mode = Mode::Home;
                    self.info_message = Some("Aborted.".to_string());
                    self.remove_step = RemoveStep::Done {
                        message: "Aborted.".to_string(),
                    };
                }
            }
            RemoveStep::Done { .. } => {
                self.mode = Mode::Home;
                self.remove_step = RemoveStep::default();
            }
        }

        Ok(())
    }

    /// Return a short one-line hint for the current import step.
    pub fn import_step_hint(&self) -> &'static str {
        match self.import_step {
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
                })
            })
            .collect();

        ChangePlan::new(changes)
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
        let applied = result
            .applied
            .iter()
            .map(|change| change_summary(change, "✓"))
            .collect::<Vec<_>>()
            .join("\n");
        let message = match &result.failed {
            Some((failed, error)) => format!(
                "Applied {} change(s).\n{}\n\n1 change failed:\n{}\n  Reason: {error}",
                result.applied.len(),
                applied,
                change_summary(failed, "✗"),
            ),
            None => format!("Applied {} change(s).\n{applied}", result.applied.len()),
        };

        if had_failure {
            self.error_message = Some(message.clone());
        } else {
            self.info_message = Some(message.clone());
        }

        self.refresh_inventory()?;
        Ok(message)
    }
}

pub(crate) fn removable_exposures(row: &InventoryRow) -> Vec<SkillExposure> {
    if row.scope == Scope::ProjectLocal {
        return Vec::new();
    }
    let mut seen_paths = HashSet::new();
    row.exposures
        .iter()
        .filter(|exposure| {
            matches!(
                exposure.connection,
                ConnectionKind::Symlink | ConnectionKind::PhysicalCopy
            )
        })
        .filter(|exposure| seen_paths.insert(exposure.path.clone()))
        .cloned()
        .collect()
}

fn change_summary(change: &StagedChange, status: &str) -> String {
    match change {
        StagedChange::ExposeSkill {
            skill_name,
            target_path,
            ..
        } => format!(
            "  {status} Exposed {skill_name} at {}",
            target_path.display()
        ),
        StagedChange::DetachSkill {
            skill_name,
            target_path,
            ..
        } => format!(
            "  {status} Removed {skill_name} from {}",
            target_path.display()
        ),
        StagedChange::DeletePhysicalCopy {
            skill_name,
            target_path,
            ..
        } => format!(
            "  {status} Deleted {skill_name} from {}",
            target_path.display()
        ),
    }
}
