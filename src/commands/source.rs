use std::collections::HashSet;

use anyhow::{Result, bail};

use crate::cli::{ImportArgs, SourceArgs, SourceSubcommand};
use crate::commands::helpers;
use crate::scanner::ScanResult;
use crate::source::{self, AcquireOutcome};

pub fn run(args: SourceArgs) -> Result<()> {
    match args.subcommand {
        SourceSubcommand::Add(args) => add(args.git_url),
    }
}

fn add(git_url: String) -> Result<()> {
    let config = helpers::load_config()?;
    let context = config.resolve_global_context()?;
    let source_preview = source::preview(&git_url, &context.central_dir)?;

    println!("Source URL: {}", source_preview.url);
    println!("Destination: {}", source_preview.destination.display());

    if !helpers::is_interactive() {
        bail!("Cannot add a source in non-interactive mode. Please run interactively.");
    }
    if !helpers::confirm("Add this source? [y/N]: ")? {
        println!("Aborted.");
        return Ok(());
    }

    let mut acquired = source::acquire(&source_preview, context.max_scan_depth)?;
    acquired
        .skills
        .sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
    match acquired.outcome {
        AcquireOutcome::Cloned => println!("Added source at {}.", acquired.path.display()),
        AcquireOutcome::Reused => println!("Reused source at {}.", acquired.path.display()),
    }
    println!("Discovered skills:");
    for (index, skill) in acquired.skills.iter().enumerate() {
        println!(
            "  {}. {}  {}",
            index + 1,
            skill.skill_id,
            skill.skill_path.display()
        );
    }

    let selected = loop {
        let Some(input) = helpers::read_line(
            "Skills to expose (comma-separated numbers, Enter to keep source only): ",
        )?
        else {
            println!("Source kept without new exposures.");
            return Ok(());
        };
        match parse_skill_selection(&input, &acquired.skills) {
            Ok(selected) => break selected,
            Err(error) => println!("Invalid selection: {error}"),
        }
    };

    if selected.is_empty() {
        println!("Source kept without new exposures.");
        return Ok(());
    }

    for skill in selected {
        crate::commands::import::run(ImportArgs { skill, to: None })?;
    }

    Ok(())
}

fn parse_skill_selection(input: &str, skills: &[ScanResult]) -> Result<Vec<String>> {
    if input.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for raw_index in input.split(',') {
        let index = raw_index
            .trim()
            .parse::<usize>()
            .map_err(|_| anyhow::anyhow!("expected comma-separated skill numbers"))?;
        if !(1..=skills.len()).contains(&index) {
            bail!("skill number must be between 1 and {}", skills.len());
        }
        let skill_id = skills[index - 1].skill_id.clone();
        if seen.insert(skill_id.clone()) {
            selected.push(skill_id);
        }
    }

    Ok(selected)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::scanner::{ScanResult, SourceKind};

    use super::parse_skill_selection;

    #[test]
    fn empty_selection_keeps_source_without_exposures() {
        assert!(parse_skill_selection("", &skills()).unwrap().is_empty());
    }

    #[test]
    fn parses_multiple_unique_skill_numbers() {
        assert_eq!(
            parse_skill_selection("2, 1, 2", &skills()).unwrap(),
            vec!["repo/docs", "repo/review"]
        );
    }

    #[test]
    fn rejects_out_of_range_skill_number() {
        let error = parse_skill_selection("3", &skills()).unwrap_err();

        assert!(error.to_string().contains("between 1 and 2"));
    }

    fn skills() -> Vec<ScanResult> {
        vec![
            scan_result("repo/review", "/repo/review"),
            scan_result("repo/docs", "/repo/docs"),
        ]
    }

    fn scan_result(skill_id: &str, path: &str) -> ScanResult {
        ScanResult {
            skill_id: skill_id.to_string(),
            skill_path: PathBuf::from(path),
            skill_relative_path: None,
            repo_name: Some("repo".to_string()),
            repo_path: Some(PathBuf::from("/repo")),
            remote_url: None,
            source_kind: SourceKind::CentralDir,
            disambiguation_index: None,
        }
    }
}
