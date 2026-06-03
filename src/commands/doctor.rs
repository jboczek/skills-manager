use std::env;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::commands::helpers;
use crate::config::Config;

pub fn run() -> Result<()> {
    let mut has_fail = false;
    let current_dir = env::current_dir()?;
    let mut loaded_config = None;

    match Config::default_path() {
        Some(path) if path.exists() => match Config::load_from(&path) {
            Ok(config) => {
                println!("PASS: Config found and valid at {}", path.display());
                loaded_config = Some(config);
            }
            Err(error) => {
                println!(
                    "FAIL: Config file at {} could not be parsed: {error}",
                    path.display()
                );
                has_fail = true;
            }
        },
        Some(path) => println!(
            "WARN: No config file found at {}. Run `skills-manager config init` to create one.",
            path.display()
        ),
        None => {
            println!("FAIL: Could not determine the default config file path.");
            has_fail = true;
        }
    }

    if let Some(config) = loaded_config.as_ref() {
        for source_dir in
            std::iter::once(&config.skills.central_dir).chain(config.skills.scan_parent_dirs.iter())
        {
            let path = helpers::resolve_path(&current_dir, source_dir);
            if path.exists() {
                println!("PASS: Source directory exists: {}", path.display());
            } else {
                println!("WARN: Source directory not found: {}", path.display());
            }
        }

        for agent in config.agents.values() {
            let path = helpers::resolve_path(&current_dir, &agent.global_dir);
            if !path.exists() {
                println!(
                    "WARN: Agent {} target directory does not exist (will be created on import): {}",
                    agent.display_name,
                    path.display()
                );
                continue;
            }

            if dir_is_writable(&path) {
                println!(
                    "PASS: Agent {} target directory exists and is writable: {}",
                    agent.display_name,
                    path.display()
                );
            } else {
                println!(
                    "FAIL: Agent {} target directory exists but is not writable: {}",
                    agent.display_name,
                    path.display()
                );
                has_fail = true;
            }
        }
    }

    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("PASS: Git CLI available: {version}");
        }
        _ => {
            println!(
                "FAIL: Git CLI not found. Git is required for origin detection. Install git and ensure it's on PATH."
            );
            has_fail = true;
        }
    }

    if has_fail {
        println!("Some checks failed.");
    } else {
        println!("All checks passed.");
    }

    Ok(())
}

fn dir_is_writable(path: &std::path::Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let probe = path.join(format!(".skills-manager-doctor-{unique}"));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => fs::remove_file(probe).is_ok(),
        Err(_) => false,
    }
}
