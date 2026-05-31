use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "skills-manager",
    version,
    about = "Terminal-first skill exposure manager",
    subcommand_required = false,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Commands {
    List,
    Scan,
    Import,
    Remove,
    Config(ConfigArgs),
    Doctor,
}

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
pub struct ConfigArgs {
    /// Override the config file path.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub subcommand: ConfigSubcommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ConfigSubcommand {
    /// Initialize config file with defaults (does not overwrite existing).
    Init,
    /// Print the resolved config file path.
    Path,
    /// Print the current configuration.
    Show,
}
