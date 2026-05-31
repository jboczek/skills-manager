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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum Commands {
    List,
    Scan,
    Import,
    Remove,
    Config,
    Doctor,
}
