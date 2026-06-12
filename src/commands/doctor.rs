use std::collections::HashSet;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::config::{Config, GlobalContext};

pub fn run() -> Result<()> {
    let mut has_fail = false;
    let mut global_context = None;

    match Config::default_path() {
        Some(path) if path.exists() => match Config::load_from(&path) {
            Ok(config) => match config.resolve_global_context() {
                Ok(context) => {
                    println!("PASS: Config found and valid at {}", path.display());
                    for diagnostic in &context.diagnostics {
                        println!("WARN: {diagnostic}");
                    }
                    global_context = Some(context);
                }
                Err(error) => {
                    println!(
                        "FAIL: Config file at {} is invalid: {error}",
                        path.display()
                    );
                    has_fail = true;
                }
            },
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

    if let Some(context) = global_context.as_ref() {
        for path in std::iter::once(&context.central_dir).chain(context.scan_parent_dirs.iter()) {
            if path.exists() {
                println!("PASS: Source directory exists: {}", path.display());
            } else {
                println!("WARN: Source directory not found: {}", path.display());
            }
        }

        check_target_dirs(context, &mut has_fail);
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

fn check_target_dirs(context: &GlobalContext, has_fail: &mut bool) {
    let mut checked = HashSet::new();
    for agent in &context.agents {
        let target_dirs = agent
            .global_dir
            .iter()
            .chain(agent.shared_target_dirs.iter().map(|(path, _)| path));
        for path in target_dirs {
            if !checked.insert(path.clone()) {
                continue;
            }
            if !path.exists() {
                println!(
                    "WARN: Global target directory does not exist (will be created on import): {}",
                    path.display()
                );
                continue;
            }

            if dir_is_writable(path) {
                println!(
                    "PASS: Global target directory exists and is writable: {}",
                    path.display()
                );
            } else {
                println!(
                    "FAIL: Global target directory exists but is not writable: {}",
                    path.display()
                );
                *has_fail = true;
            }
        }
    }
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
