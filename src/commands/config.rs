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
            let config = Config::load_from(&path)?;
            let context = config.resolve_global_context()?;
            for diagnostic in context.diagnostics {
                eprintln!("WARN: {diagnostic}");
            }
            print!("{}", config.to_toml()?);
        }
    }
    Ok(())
}
