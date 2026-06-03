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
    Import(ImportArgs),
    Remove(RemoveArgs),
    Config(ConfigArgs),
    Doctor,
}

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
pub struct ImportArgs {
    /// Skill identifier (e.g. repo-a/code-review)
    pub skill: String,
    /// Target agents, comma-separated (e.g. claude,codex). If omitted, import to all enabled agents.
    #[arg(long, value_name = "AGENTS")]
    pub to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
pub struct RemoveArgs {
    /// Skill identifier (e.g. repo-a/code-review)
    pub skill: String,
    /// Source agents, comma-separated (e.g. claude). If omitted, remove from all agents.
    #[arg(long, value_name = "AGENTS")]
    pub from: Option<String>,
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
