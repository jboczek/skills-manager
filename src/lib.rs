pub mod agent_dirs;
pub mod cli;
pub mod commands;
pub mod config;
pub mod domain;
pub mod errors;
pub mod git;
pub mod inventory;
pub mod output;
pub mod scanner;
pub mod symlink;
pub mod tui;

use anyhow::Result;
use clap::Parser;

pub fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    dispatch(cli)
}

pub fn dispatch(cli: cli::Cli) -> Result<()> {
    match cli.command {
        Some(cli::Commands::List) => commands::list::run(),
        Some(cli::Commands::Scan) => commands::scan::run(),
        Some(cli::Commands::Import) => commands::import::run(),
        Some(cli::Commands::Remove) => commands::remove::run(),
        Some(cli::Commands::Config(args)) => commands::config::run(args),
        Some(cli::Commands::Doctor) => commands::doctor::run(),
        None => commands::tui::run(),
    }
}
