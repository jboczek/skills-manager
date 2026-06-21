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
    Source(SourceArgs),
    Import(ImportArgs),
    Remove(RemoveArgs),
    Config(ConfigArgs),
    Doctor,
}

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
pub struct SourceArgs {
    #[command(subcommand)]
    pub subcommand: SourceSubcommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum SourceSubcommand {
    /// Add a Git repository to the managed source directory.
    Add(SourceAddArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
pub struct SourceAddArgs {
    /// Git repository URL.
    pub git_url: String,
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Commands, SourceSubcommand};

    #[test]
    fn parses_source_add_command() {
        let cli = Cli::try_parse_from([
            "skills-manager",
            "source",
            "add",
            "https://example.com/org/skills.git",
        ])
        .unwrap();

        let Some(Commands::Source(args)) = cli.command else {
            panic!("expected source command");
        };
        let SourceSubcommand::Add(args) = args.subcommand;
        assert_eq!(args.git_url, "https://example.com/org/skills.git");
    }
}
