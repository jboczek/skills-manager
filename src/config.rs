use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillsConfig {
    pub central_dir: String,
    #[serde(default)]
    pub scan_parent_dirs: Vec<String>,
    #[serde(default = "default_max_scan_depth")]
    pub max_scan_depth: u32,
}

fn default_max_scan_depth() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    pub display_name: String,
    pub global_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_dir: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub shared_target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedTargetConfig {
    pub display_name: String,
    pub project_dir: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreferencesConfig {
    pub default_connection: String,
    pub confirm_physical_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub skills: SkillsConfig,
    pub agents: BTreeMap<String, AgentConfig>,
    pub shared_targets: BTreeMap<String, SharedTargetConfig>,
    pub preferences: PreferencesConfig,
}

impl Config {
    pub fn default_config() -> Self {
        let mut agents = BTreeMap::new();
        agents.insert(
            "claude".to_string(),
            AgentConfig {
                display_name: "Claude".to_string(),
                global_dir: "~/.claude/skills".to_string(),
                project_dir: None,
                enabled: true,
                shared_target_ids: vec![],
            },
        );
        agents.insert(
            "codex".to_string(),
            AgentConfig {
                display_name: "Codex".to_string(),
                global_dir: "~/.codex/skills".to_string(),
                project_dir: Some(".codex/skills".to_string()),
                enabled: true,
                shared_target_ids: vec!["agents".to_string()],
            },
        );
        agents.insert(
            "copilot".to_string(),
            AgentConfig {
                display_name: "Copilot".to_string(),
                global_dir: "~/.copilot/skills".to_string(),
                project_dir: Some(".copilot/skills".to_string()),
                enabled: true,
                shared_target_ids: vec!["agents".to_string()],
            },
        );

        let mut shared_targets = BTreeMap::new();
        shared_targets.insert(
            "agents".to_string(),
            SharedTargetConfig {
                display_name: ".agents".to_string(),
                project_dir: ".agents".to_string(),
                enabled: true,
            },
        );

        Self {
            skills: SkillsConfig {
                central_dir: "~/skills".to_string(),
                scan_parent_dirs: vec![],
                max_scan_depth: 10,
            },
            agents,
            shared_targets,
            preferences: PreferencesConfig {
                default_connection: "symlink".to_string(),
                confirm_physical_delete: true,
            },
        }
    }

    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("dev", "github", "skills-manager")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// Parse config from a TOML string without filesystem access (useful in tests).
    pub fn parse(content: &str) -> anyhow::Result<Self> {
        toml::from_str(content).map_err(|e| anyhow::anyhow!("config parse error: {e}"))
    }

    pub fn load_from(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", path.display(), e))
    }

    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Write this config to `path`, creating parent directories as needed.
    /// Fails if the file already exists (use `create_new` semantics).
    pub fn write_new(&self, path: &Path) -> anyhow::Result<WriteOutcome> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
        }
        let toml_str = self.to_toml()?;
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(mut file) => {
                file.write_all(toml_str.as_bytes())
                    .with_context(|| format!("failed to write config: {}", path.display()))?;
                Ok(WriteOutcome::Created)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(WriteOutcome::AlreadyExists),
            Err(e) => Err(e).with_context(|| format!("failed to create config: {}", path.display())),
        }
    }
}

pub enum WriteOutcome {
    Created,
    AlreadyExists,
}

/// Expand a leading `~` or `~/` to the user's home directory.
/// Only expands when `~` appears at the very start of the string.
/// A `~` in the middle of a path is left as-is.
pub fn expand_tilde(s: &str) -> PathBuf {
    if s == "~" {
        return home_dir();
    }
    if let Some(rest) = s.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    PathBuf::from(s)
}

fn home_dir() -> PathBuf {
    BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_TOML: &str = r#"
[skills]
central_dir = "~/skills"
scan_parent_dirs = []
max_scan_depth = 10

[agents.claude]
display_name = "Claude"
global_dir = "~/.claude/skills"
enabled = true
shared_target_ids = []

[agents.codex]
display_name = "Codex"
global_dir = "~/.codex/skills"
project_dir = ".codex/skills"
enabled = true
shared_target_ids = ["agents"]

[agents.copilot]
display_name = "Copilot"
global_dir = "~/.copilot/skills"
project_dir = ".copilot/skills"
enabled = true
shared_target_ids = ["agents"]

[shared_targets.agents]
display_name = ".agents"
project_dir = ".agents"
enabled = true

[preferences]
default_connection = "symlink"
confirm_physical_delete = true
"#;

    #[test]
    fn default_config_has_expected_agents() {
        let cfg = Config::default_config();
        assert!(cfg.agents.contains_key("claude"));
        assert!(cfg.agents.contains_key("codex"));
        assert!(cfg.agents.contains_key("copilot"));
        assert_eq!(cfg.agents.len(), 3);
    }

    #[test]
    fn default_config_has_skills_section() {
        let cfg = Config::default_config();
        assert_eq!(cfg.skills.central_dir, "~/skills");
        assert!(cfg.skills.scan_parent_dirs.is_empty());
        assert_eq!(cfg.skills.max_scan_depth, 10);
    }

    #[test]
    fn shared_targets_are_not_in_agents() {
        let cfg = Config::default_config();
        assert!(!cfg.agents.contains_key("agents"));
        assert!(cfg.shared_targets.contains_key("agents"));
    }

    #[test]
    fn default_config_preferences() {
        let cfg = Config::default_config();
        assert_eq!(cfg.preferences.default_connection, "symlink");
        assert!(cfg.preferences.confirm_physical_delete);
    }

    #[test]
    fn parse_valid_toml() {
        let cfg = Config::parse(EXAMPLE_TOML).expect("valid TOML should parse");
        assert_eq!(cfg.skills.central_dir, "~/skills");
        assert_eq!(cfg.agents["claude"].display_name, "Claude");
        assert_eq!(cfg.agents["codex"].project_dir, Some(".codex/skills".to_string()));
        assert_eq!(cfg.agents["copilot"].shared_target_ids, vec!["agents"]);
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        let result = Config::parse("not valid toml [[[");
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_skills_section_returns_error() {
        let toml = r#"
[preferences]
default_connection = "symlink"
confirm_physical_delete = true
"#;
        let result = Config::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn to_toml_roundtrip() {
        let original = Config::default_config();
        let toml_str = original.to_toml().expect("serialization succeeds");
        let parsed = Config::parse(&toml_str).expect("re-parsing succeeds");
        assert_eq!(original, parsed);
    }

    #[test]
    fn expand_tilde_leading_slash() {
        let result = expand_tilde("~/skills");
        let home = home_dir();
        assert_eq!(result, home.join("skills"));
    }

    #[test]
    fn expand_tilde_bare() {
        let result = expand_tilde("~");
        assert_eq!(result, home_dir());
    }

    #[test]
    fn expand_tilde_in_middle_not_expanded() {
        let result = expand_tilde("path/~/file");
        assert_eq!(result, PathBuf::from("path/~/file"));
    }

    #[test]
    fn expand_tilde_absolute_path_unchanged() {
        let result = expand_tilde("/absolute/path");
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn agent_map_parsed_with_all_fields() {
        let cfg = Config::parse(EXAMPLE_TOML).unwrap();
        let claude = &cfg.agents["claude"];
        assert_eq!(claude.global_dir, "~/.claude/skills");
        assert!(claude.project_dir.is_none());
        assert!(claude.enabled);
        assert!(claude.shared_target_ids.is_empty());

        let codex = &cfg.agents["codex"];
        assert_eq!(codex.shared_target_ids, vec!["agents"]);
    }

    #[test]
    fn write_new_creates_file_and_write_new_again_returns_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = Config::default_config();

        let outcome = cfg.write_new(&path).unwrap();
        assert!(matches!(outcome, WriteOutcome::Created));
        assert!(path.exists());

        let outcome2 = cfg.write_new(&path).unwrap();
        assert!(matches!(outcome2, WriteOutcome::AlreadyExists));
    }

    #[test]
    fn write_new_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dirs").join("config.toml");
        let cfg = Config::default_config();
        cfg.write_new(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn load_from_returns_error_with_path_context() {
        let result = Config::load_from(Path::new("/nonexistent/path/config.toml"));
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("/nonexistent/path/config.toml"),
            "error should mention the path: {msg}"
        );
    }
}
