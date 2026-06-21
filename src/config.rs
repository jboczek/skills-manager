use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::constants::*;
use crate::domain::Scope;
use crate::inventory::AgentTarget;
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
    #[serde(default, skip_serializing)]
    pub project_dir: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub shared_target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedTargetConfig {
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_dir: Option<String>,
    #[serde(default, skip_serializing)]
    pub project_dir: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub skills: SkillsConfig,
    pub agents: BTreeMap<String, AgentConfig>,
    pub shared_targets: BTreeMap<String, SharedTargetConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalContext {
    pub central_dir: PathBuf,
    pub scan_parent_dirs: Vec<PathBuf>,
    pub max_scan_depth: usize,
    pub agents: Vec<AgentTarget>,
    pub diagnostics: Vec<String>,
}

impl Config {
    pub fn default_config() -> Self {
        let mut agents = BTreeMap::new();
        agents.insert(
            AGENT_ID_CLAUDE.to_string(),
            AgentConfig {
                display_name: AGENT_NAME_CLAUDE.to_string(),
                global_dir: AGENT_GLOBAL_DIR_CLAUDE.to_string(),
                project_dir: None,
                enabled: true,
                shared_target_ids: vec![],
            },
        );
        agents.insert(
            AGENT_ID_CODEX.to_string(),
            AgentConfig {
                display_name: AGENT_NAME_CODEX.to_string(),
                global_dir: AGENT_GLOBAL_DIR_CODEX.to_string(),
                project_dir: None,
                enabled: true,
                shared_target_ids: vec![SHARED_TARGET_AGENTS.to_string()],
            },
        );
        agents.insert(
            AGENT_ID_COPILOT.to_string(),
            AgentConfig {
                display_name: AGENT_NAME_COPILOT.to_string(),
                global_dir: AGENT_GLOBAL_DIR_COPILOT.to_string(),
                project_dir: None,
                enabled: true,
                shared_target_ids: vec![SHARED_TARGET_AGENTS.to_string()],
            },
        );

        let mut shared_targets = BTreeMap::new();
        shared_targets.insert(
            SHARED_TARGET_AGENTS.to_string(),
            SharedTargetConfig {
                display_name: SHARED_TARGET_DISPLAY_NAME.to_string(),
                global_dir: Some(SHARED_TARGET_GLOBAL_DIR.to_string()),
                project_dir: None,
                enabled: true,
            },
        );

        Self {
            skills: SkillsConfig {
                central_dir: DEFAULT_SKILLS_CENTRAL_DIR.to_string(),
                scan_parent_dirs: vec![],
                max_scan_depth: 10,
            },
            agents,
            shared_targets,
        }
    }

    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("dev", "github", "skills-manager")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// Parse config from a TOML string without filesystem access (useful in tests).
    pub fn parse(content: &str) -> anyhow::Result<Self> {
        let mut config: Self =
            toml::from_str(content).map_err(|e| anyhow::anyhow!("config parse error: {e}"))?;
        config.apply_legacy_defaults();
        Ok(config)
    }

    pub fn load_from(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        Self::parse(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", path.display(), e))
    }

    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn resolve_global_context(&self) -> anyhow::Result<GlobalContext> {
        let diagnostics = Vec::new();
        let mut path_errors = Vec::new();

        let central_dir = resolve_active_path(
            "skills.central_dir",
            &self.skills.central_dir,
            &mut path_errors,
        );
        let scan_parent_dirs = self
            .skills
            .scan_parent_dirs
            .iter()
            .enumerate()
            .map(|(index, path)| {
                resolve_active_path(
                    &format!("skills.scan_parent_dirs[{index}]"),
                    path,
                    &mut path_errors,
                )
            })
            .collect::<Vec<_>>();
        let shared_target_paths = self
            .shared_targets
            .iter()
            .filter(|(_, target)| target.enabled)
            .filter_map(|(target_id, target)| {
                target.global_dir.as_deref().map(|path| {
                    (
                        target_id.clone(),
                        resolve_active_path(
                            &format!("shared_targets.{target_id}.global_dir"),
                            path,
                            &mut path_errors,
                        ),
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();

        let agents = self
            .agents
            .iter()
            .map(|(agent_id, agent)| {
                let global_dir = resolve_active_path(
                    &format!("agents.{agent_id}.global_dir"),
                    &agent.global_dir,
                    &mut path_errors,
                );
                let shared_target_dirs = agent
                    .shared_target_ids
                    .iter()
                    .filter_map(|target_id| shared_target_paths.get(target_id))
                    .cloned()
                    .map(|path| (path, Scope::Global))
                    .collect();

                AgentTarget {
                    agent_id: agent_id.clone(),
                    display_name: agent.display_name.clone(),
                    global_dir: Some(global_dir),
                    shared_target_dirs,
                    enabled: agent.enabled,
                }
            })
            .collect();

        if !path_errors.is_empty() {
            let details = diagnostics
                .iter()
                .chain(path_errors.iter())
                .map(|message| format!("- {message}"))
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::bail!("configuration validation failed:\n{details}");
        }

        Ok(GlobalContext {
            central_dir,
            scan_parent_dirs,
            max_scan_depth: self.skills.max_scan_depth as usize,
            agents,
            diagnostics,
        })
    }

    /// Write this config to `path`, creating parent directories as needed.
    /// Fails if the file already exists (use `create_new` semantics).
    pub fn write_new(&self, path: &Path) -> anyhow::Result<WriteOutcome> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory: {}", parent.display())
            })?;
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
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Ok(WriteOutcome::AlreadyExists)
            }
            Err(e) => {
                Err(e).with_context(|| format!("failed to create config: {}", path.display()))
            }
        }
    }

    fn apply_legacy_defaults(&mut self) {
        let Some(agents_target) = self.shared_targets.get_mut(SHARED_TARGET_AGENTS) else {
            return;
        };

        if agents_target.global_dir.is_none() && agents_target.project_dir.is_some() {
            agents_target.global_dir = Some(SHARED_TARGET_GLOBAL_DIR.to_string());
        }
    }
}

fn resolve_active_path(field: &str, raw_path: &str, errors: &mut Vec<String>) -> PathBuf {
    let path = expand_tilde(raw_path);
    if !path.is_absolute() {
        errors.push(format!(
            "{field} must be absolute after leading-tilde expansion; rejected value: {raw_path}"
        ));
    }
    path
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
        assert_eq!(
            cfg.shared_targets["agents"].global_dir.as_deref(),
            Some("~/.agents/skills")
        );
        assert!(cfg.shared_targets["agents"].project_dir.is_none());
    }

    #[test]
    fn parse_valid_toml() {
        let cfg = Config::parse(EXAMPLE_TOML).expect("valid TOML should parse");
        assert_eq!(cfg.skills.central_dir, "~/skills");
        assert_eq!(cfg.agents["claude"].display_name, "Claude");
        assert_eq!(
            cfg.agents["codex"].project_dir,
            Some(".codex/skills".to_string())
        );
        assert_eq!(cfg.agents["copilot"].shared_target_ids, vec!["agents"]);
        assert_eq!(
            cfg.shared_targets["agents"].global_dir.as_deref(),
            Some("~/.agents/skills")
        );
        assert_eq!(
            cfg.shared_targets["agents"].project_dir.as_deref(),
            Some(".agents")
        );
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

    #[test]
    fn resolve_global_context_expands_tilde_paths() {
        let config = Config::default_config();

        let context = config
            .resolve_global_context()
            .expect("default config should resolve");

        assert_eq!(context.central_dir, home_dir().join("skills"));
        assert!(
            context
                .agents
                .iter()
                .filter_map(|agent| agent.global_dir.as_ref())
                .all(|path| path.is_absolute())
        );
        assert!(
            context
                .agents
                .iter()
                .flat_map(|agent| agent.shared_target_dirs.iter())
                .all(|(path, _)| path.is_absolute())
        );
    }

    #[test]
    fn resolve_global_context_rejects_relative_active_path() {
        let mut config = Config::default_config();
        config.skills.central_dir = "relative/skills".to_string();

        let error = config
            .resolve_global_context()
            .expect_err("relative path should fail");
        let message = error.to_string();

        assert!(message.contains("skills.central_dir"));
        assert!(message.contains("relative/skills"));
    }

    #[test]
    fn resolve_global_context_rejects_relative_enabled_shared_target() {
        let mut config = Config::default_config();
        config.shared_targets.insert(
            "unused".to_string(),
            SharedTargetConfig {
                display_name: "Unused".to_string(),
                global_dir: Some("relative/shared".to_string()),
                project_dir: None,
                enabled: true,
            },
        );

        let error = config
            .resolve_global_context()
            .expect_err("relative enabled shared target should fail");
        let message = error.to_string();

        assert!(message.contains("shared_targets.unused.global_dir"));
        assert!(message.contains("relative/shared"));
    }

    #[test]
    fn resolve_global_context_ignores_legacy_project_dirs_without_diagnostics() {
        let config = Config::parse(EXAMPLE_TOML).expect("legacy config should parse");

        let context = config
            .resolve_global_context()
            .expect("legacy project paths should not block global config");

        assert!(context.diagnostics.is_empty());
        assert!(
            context
                .agents
                .iter()
                .flat_map(|agent| agent.shared_target_dirs.iter())
                .all(|(_, scope)| *scope == crate::domain::Scope::Global)
        );
    }

    #[test]
    fn serialization_omits_legacy_project_dirs() {
        let config = Config::parse(EXAMPLE_TOML).expect("legacy config should parse");

        let serialized = config.to_toml().expect("config should serialize");

        assert!(!serialized.contains("project_dir"));
    }

    #[test]
    fn serialization_omits_unused_preferences() {
        let config = Config::parse(EXAMPLE_TOML).expect("legacy config should parse");

        let serialized = config.to_toml().expect("config should serialize");

        assert!(!serialized.contains("[preferences]"));
        assert!(!serialized.contains("default_connection"));
        assert!(!serialized.contains("confirm_physical_delete"));
    }
}
