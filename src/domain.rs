use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillId {
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDefinition {
    pub id: AgentId,
    pub display_name: String,
    pub global_dir: Option<PathBuf>,
    pub project_dir: Option<PathBuf>,
    pub shared_target_ids: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    ProjectLocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    Symlink,
    PhysicalCopy,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSource {
    pub repo_name: Option<String>,
    pub repo_path: Option<PathBuf>,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillExposure {
    pub agent_id: AgentId,
    pub path: PathBuf,
    pub connection: ConnectionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryRow {
    pub skill_id: SkillId,
    pub source: SkillSource,
    pub scope: Scope,
    pub exposures: Vec<SkillExposure>,
}
