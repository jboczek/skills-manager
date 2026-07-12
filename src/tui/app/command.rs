#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiCommand {
    List,
    SourceAdd(String),
    Import,
    Remove,
    Config,
    Help,
    Quit,
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSuggestion {
    pub label: &'static str,
    pub description: &'static str,
}

pub(crate) const COMMAND_SUGGESTIONS: [CommandSuggestion; 5] = [
    CommandSuggestion {
        label: "/list",
        description: "Show exposed skills and availability",
    },
    CommandSuggestion {
        label: "/source_add",
        description: "Add new skills from Git repository using HTTPS/SSH clone URL",
    },
    CommandSuggestion {
        label: "/config",
        description: "Show current configuration",
    },
    CommandSuggestion {
        label: "/help",
        description: "Show commands and keybindings",
    },
    CommandSuggestion {
        label: "/quit",
        description: "Exit Skills Manager",
    },
];

/// Parse a command string typed in the prompt.
/// Accepts "list", "/list", "source_add <git-url>", etc.
pub fn parse_command(input: &str) -> TuiCommand {
    let trimmed = input.trim();
    let normalized = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let mut parts = normalized.split_whitespace();
    let Some(command) = parts.next() else {
        return TuiCommand::Unknown(input.to_string());
    };
    let argument = parts.collect::<Vec<_>>().join(" ");

    match command {
        "list" => TuiCommand::List,
        "source_add" => TuiCommand::SourceAdd(argument),
        "import" => TuiCommand::Import,
        "remove" => TuiCommand::Remove,
        "config" => TuiCommand::Config,
        "help" | "?" => TuiCommand::Help,
        "q" | "quit" => TuiCommand::Quit,
        _ => TuiCommand::Unknown(trimmed.to_string()),
    }
}
