use anyhow::{Result, bail};

use crate::cli::RemoveArgs;
use crate::commands::helpers;
use crate::domain::{AgentId, ConnectionKind, Scope};
use crate::output;
use crate::plan::{ChangePlan, StagedChange};
use crate::plan_apply;

pub fn run(args: RemoveArgs) -> Result<()> {
    let config = helpers::load_config()?;
    let context = config.resolve_global_context()?;
    let rows = helpers::fresh_global_inventory(&context)?;

    let matches = helpers::find_inventory_rows_by_id(&args.skill, &rows);
    let selected = match matches.len() {
        0 => {
            println!(
                "Skill '{}' not found in inventory. Run `skills-manager list` to see exposed skills.",
                args.skill
            );
            return Ok(());
        }
        1 => matches[0],
        _ if !helpers::is_interactive() => {
            println!(
                "Skill '{}' is ambiguous. Run in interactive mode or use a more specific identifier.",
                args.skill
            );
            return Ok(());
        }
        _ => {
            println!(
                "Found multiple inventory entries matching '{}':",
                args.skill
            );
            helpers::print_inventory_disambiguation(&matches);
            let Some(selected) = choose_inventory_row(&matches)? else {
                return Ok(());
            };
            selected
        }
    };
    if selected.scope == Scope::ProjectLocal {
        println!("Project-local exposures are read-only and cannot be removed.");
        return Ok(());
    }

    let agent_filter = match args.from.as_deref() {
        Some(raw_agents) => {
            let requested = helpers::parse_agents(raw_agents);
            for agent_id in &requested {
                if !config.agents.contains_key(agent_id) {
                    bail!("Unknown agent: {agent_id}");
                }
            }
            Some(requested)
        }
        None => None,
    };

    let display_skill = if selected.skill_id.namespace.is_empty() {
        selected.skill_id.name.clone()
    } else {
        format!("{}/{}", selected.skill_id.namespace, selected.skill_id.name)
    };
    let mut changes = Vec::new();
    for exposure in &selected.exposures {
        if agent_filter.as_ref().is_some_and(|requested| {
            !requested
                .iter()
                .any(|agent_id| agent_id == &exposure.agent_id.0)
        }) {
            continue;
        }

        let display_name = config
            .agents
            .get(&exposure.agent_id.0)
            .map(|agent| agent.display_name.clone())
            .unwrap_or_else(|| exposure.agent_id.0.clone());

        match exposure.connection {
            ConnectionKind::Symlink => changes.push(StagedChange::DetachSkill {
                skill_name: display_skill.clone(),
                agent_id: AgentId(display_name),
                target_path: exposure.path.clone(),
            }),
            ConnectionKind::PhysicalCopy => changes.push(StagedChange::DeletePhysicalCopy {
                skill_name: display_skill.clone(),
                agent_id: AgentId(display_name),
                target_path: exposure.path.clone(),
            }),
            ConnectionKind::Missing => println!(
                "  No exposure found for {} (already missing), skipping.",
                display_name
            ),
            ConnectionKind::Unknown => println!(
                "  Unknown connection type for {} at {}, skipping.",
                display_name,
                exposure.path.display()
            ),
        }
    }

    if changes.is_empty() {
        println!("Nothing to remove.");
        return Ok(());
    }

    let plan = ChangePlan::new(changes);
    println!("{}", plan.render());
    if plan.has_physical_deletes() {
        println!("⚠ WARNING: This will permanently delete directories.");
    }

    if !helpers::is_interactive() {
        bail!("Cannot apply plan in non-interactive mode. Please run interactively.");
    }
    if !helpers::confirm("Apply this plan? [y/N]: ")? {
        println!("Aborted.");
        return Ok(());
    }
    if plan.has_physical_deletes() {
        match helpers::read_line("This plan includes permanent deletion. Type 'yes' to confirm: ")?
        {
            Some(answer) if physical_delete_confirmation_allows(&answer) => {}
            _ => {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    let result = plan_apply::apply_plan(&plan);
    match result.failed {
        Some((_, error)) => println!(
            "Applied {} change(s). 1 change failed: {error}",
            result.applied.len()
        ),
        None => println!("Applied {} change(s).", result.applied.len()),
    }

    let refreshed = helpers::fresh_global_inventory(&context)?;
    println!("{}", output::render_inventory(&refreshed));
    Ok(())
}

fn choose_inventory_row<'a>(
    matches: &[&'a crate::domain::InventoryRow],
) -> Result<Option<&'a crate::domain::InventoryRow>> {
    loop {
        let Some(input) = helpers::read_line(&format!("Enter number [1-{}]: ", matches.len()))?
        else {
            println!("Aborted.");
            return Ok(None);
        };
        if let Ok(index) = input.parse::<usize>()
            && (1..=matches.len()).contains(&index)
        {
            return Ok(Some(matches[index - 1]));
        }
        println!("Invalid selection.");
    }
}

fn physical_delete_confirmation_allows(answer: &str) -> bool {
    answer == "yes"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_delete_confirmation_requires_exact_yes() {
        assert!(physical_delete_confirmation_allows("yes"));
        assert!(!physical_delete_confirmation_allows("y"));
        assert!(!physical_delete_confirmation_allows("YES"));
        assert!(!physical_delete_confirmation_allows(""));
    }
}
