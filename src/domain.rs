use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillId {
    pub namespace: String,
    pub name: String,
}

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.namespace.is_empty() {
            f.write_str(&self.name)
        } else {
            write!(f, "{}/{}", self.namespace, self.name)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

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
    pub disambiguation_index: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::SkillId;

    #[test]
    fn skill_id_displays_with_optional_namespace() {
        assert_eq!(
            SkillId {
                namespace: "repo-a".to_string(),
                name: "code-review".to_string(),
            }
            .to_string(),
            "repo-a/code-review"
        );
        assert_eq!(
            SkillId {
                namespace: String::new(),
                name: "code-review".to_string(),
            }
            .to_string(),
            "code-review"
        );
    }
}
