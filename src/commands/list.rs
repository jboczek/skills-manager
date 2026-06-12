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
    let context = config.resolve_global_context()?;
    let rows = helpers::fresh_global_inventory(&context)?;
    println!("{}", output::render_inventory(&rows));
    Ok(())
}
