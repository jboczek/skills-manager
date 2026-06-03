use anyhow::{Context, Result};

use crate::cli::{ConfigArgs, ConfigSubcommand};
use crate::config::{Config, WriteOutcome};

pub fn run(args: ConfigArgs) -> Result<()> {
    let path = args
        .config
        .or_else(Config::default_path)
        .context("cannot determine config file path")?;

    match args.subcommand {
        ConfigSubcommand::Init => match Config::default_config().write_new(&path)? {
            WriteOutcome::Created => println!("Created config: {}", path.display()),
            WriteOutcome::AlreadyExists => println!("Config already exists: {}", path.display()),
        },
        ConfigSubcommand::Path => println!("{}", path.display()),
        ConfigSubcommand::Show => {
            if !path.exists() {
                println!(
                    "No config found at {}.\nRun `skills-manager config init` to create one.",
                    path.display()
                );
                return Ok(());
            }
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read config: {}", path.display()))?;
            // Validate before printing so parse errors surface the file path.
            Config::load_from(&path)?;
            print!("{content}");
        }
    }
    Ok(())
}
