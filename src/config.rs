use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub source_dir: PathBuf,
    pub target_dir: PathBuf,
}

impl Config {
    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("dev", "github", "skills-manager")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    pub fn default_values() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            source_dir: home.join("skills"),
            target_dir: home.join(".agents"),
        }
    }

    pub fn load_from(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}
