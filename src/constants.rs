//! Centralized constants for agent IDs, names, and other well-known strings.
//! This is the single source of truth for magic strings used throughout the application.

// Agent IDs
pub const AGENT_ID_CLAUDE: &str = "claude";
pub const AGENT_ID_CODEX: &str = "codex";
pub const AGENT_ID_COPILOT: &str = "copilot";

// Agent Display Names
pub const AGENT_NAME_CLAUDE: &str = "Claude";
pub const AGENT_NAME_CODEX: &str = "Codex";
pub const AGENT_NAME_COPILOT: &str = "Copilot";

// Agent Configuration Paths
pub const AGENT_GLOBAL_DIR_CLAUDE: &str = "~/.claude/skills";
pub const AGENT_GLOBAL_DIR_CODEX: &str = "~/.codex/skills";
pub const AGENT_GLOBAL_DIR_COPILOT: &str = "~/.copilot/skills";

pub const AGENT_PROJECT_DIR_CLAUDE: &str = ".claude/skills";
pub const AGENT_PROJECT_DIR_CODEX: &str = ".codex/skills";
pub const AGENT_PROJECT_DIR_COPILOT: &str = ".copilot/skills";

// Shared Targets
pub const SHARED_TARGET_AGENTS: &str = "agents";
pub const SHARED_TARGET_DISPLAY_NAME: &str = ".agents";
pub const SHARED_TARGET_GLOBAL_DIR: &str = "~/.agents/skills";
pub const SHARED_TARGET_PROJECT_DIR: &str = ".agents/skills";

// Default Skills Configuration
pub const DEFAULT_SKILLS_CENTRAL_DIR: &str = "~/skills";

/// A mapping of agent IDs to their display names for easy lookup.
pub const AGENT_COLUMNS: [(&str, &str); 3] = [
    (AGENT_ID_CLAUDE, AGENT_NAME_CLAUDE),
    (AGENT_ID_CODEX, AGENT_NAME_CODEX),
    (AGENT_ID_COPILOT, AGENT_NAME_COPILOT),
];

/// List of all known agent IDs.
pub const ALL_AGENT_IDS: &[&str] = &[AGENT_ID_CLAUDE, AGENT_ID_CODEX, AGENT_ID_COPILOT];
