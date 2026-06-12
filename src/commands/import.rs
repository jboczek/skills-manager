use std::collections::HashSet;
use std::fs;

use anyhow::{Result, bail};

use crate::cli::ImportArgs;
use crate::commands::helpers;
use crate::domain::{AgentId, ConnectionKind};
use crate::output;
use crate::plan::{ChangePlan, StagedChange};
use crate::plan_apply;
use crate::scanner;

pub fn run(args: ImportArgs) -> Result<()> {
    let config = helpers::load_config()?;
    let context = config.resolve_global_context()?;
    let mut scan_results = scanner::scan(&helpers::scan_config_from_global(&context))?;
    scanner::assign_disambiguation_indices(&mut scan_results);

    let matches = helpers::find_scan_results_by_id(&args.skill, &scan_results);
    let selected = match matches.len() {
        0 => {
            println!(
                "Skill '{}' not found. Run `skills-manager scan` to see available skills.",
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
            println!("Found multiple skills matching '{}':", args.skill);
            helpers::print_scan_disambiguation(&matches);
            let Some(selected) = choose_scan_result(&matches)? else {
                return Ok(());
            };
            selected
        }
    };

    let all_agents = helpers::agent_targets_from_global(&context);
    let target_agents = match args.to.as_deref() {
        Some(raw_agents) => {
            let requested = helpers::parse_agents(raw_agents);
            for agent_id in &requested {
                if !config.agents.contains_key(agent_id) {
                    bail!("Unknown agent: {agent_id}");
                }
            }
            all_agents
                .into_iter()
                .filter(|agent| {
                    requested
                        .iter()
                        .any(|requested| requested == &agent.agent_id)
                })
                .collect::<Vec<_>>()
        }
        None => all_agents
            .into_iter()
            .filter(|agent| agent.enabled)
            .collect::<Vec<_>>(),
    };

    let inventory = helpers::fresh_global_inventory(&context)?;
    let existing_paths = inventory
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

    let mut changes = Vec::new();
    for agent in target_agents {
        let Some(global_dir) = agent.global_dir else {
            continue;
        };
        let target_path = global_dir.join(&skill_name);
        if existing_paths.contains(&target_path) || target_path.exists() {
            println!(
                "  Skill already exposed to {} at {}, skipping.",
                agent.display_name,
                target_path.display()
            );
            continue;
        }

        changes.push(StagedChange::ExposeSkill {
            skill_name: selected.skill_id.clone(),
            agent_id: AgentId(agent.display_name),
            source_path: selected.skill_path.clone(),
            target_path,
            connection: ConnectionKind::Symlink,
        });
    }

    if changes.is_empty() {
        println!("Nothing to do.");
        return Ok(());
    }

    let plan = ChangePlan::new(changes);
    println!("{}", plan.render());

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
            Some(answer) if answer == "yes" => {}
            _ => {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    for change in &plan.changes {
        if let StagedChange::ExposeSkill { target_path, .. } = change
            && let Some(parent) = target_path.parent()
        {
            fs::create_dir_all(parent)?;
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

    let rows = helpers::fresh_global_inventory(&context)?;
    println!("{}", output::render_inventory(&rows));
    Ok(())
}

fn choose_scan_result<'a>(
    matches: &[&'a scanner::ScanResult],
) -> Result<Option<&'a scanner::ScanResult>> {
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
