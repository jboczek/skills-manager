pub mod cli;
pub mod commands;
pub mod config;
pub mod constants;
pub mod domain;
pub mod git;
pub mod inventory;
pub mod output;
pub mod plan;
pub mod plan_apply;
pub mod scanner;
pub mod source;
pub mod symlink;
pub mod tui;
pub mod update;

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
        Some(cli::Commands::Source(args)) => commands::source::run(args),
        Some(cli::Commands::Import(args)) => commands::import::run(args),
        Some(cli::Commands::Remove(args)) => commands::remove::run(args),
        Some(cli::Commands::Config(args)) => commands::config::run(args),
        Some(cli::Commands::Doctor) => commands::doctor::run(),
        None => commands::tui::run(),
    }
}
