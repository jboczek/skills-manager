use anyhow::{Context, Result};

use crate::commands::helpers;
use crate::config::Config;
use crate::output;

pub fn run() -> Result<()> {
    let path = Config::default_path().context("cannot determine config file path")?;
    if !path.exists() {
        println!(
            "No config found at {}.\nRun `skills-manager config init` to create one.",
            path.display()
        );
        return Ok(());
    }

    let config = Config::load_from(&path)?;
    let current_dir = helpers::current_dir()?;
    let rows = helpers::fresh_inventory(&config, &current_dir)?;
    println!("{}", output::render_inventory(&rows));
    Ok(())
}
