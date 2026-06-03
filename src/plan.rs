use std::fmt::Write;
use std::path::PathBuf;

use crate::domain::{AgentId, ConnectionKind};

/// A single proposed filesystem mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagedChange {
    ExposeSkill {
        skill_name: String,
        agent_id: AgentId,
        source_path: PathBuf,
        target_path: PathBuf,
        connection: ConnectionKind,
    },
    DetachSkill {
        skill_name: String,
        agent_id: AgentId,
        target_path: PathBuf,
    },
    DeletePhysicalCopy {
        skill_name: String,
        agent_id: AgentId,
        target_path: PathBuf,
    },
}

pub struct ChangePlan {
    pub changes: Vec<StagedChange>,
}

impl ChangePlan {
    pub fn new(changes: Vec<StagedChange>) -> Self {
        Self { changes }
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn has_physical_deletes(&self) -> bool {
        self.changes
            .iter()
            .any(|change| matches!(change, StagedChange::DeletePhysicalCopy { .. }))
    }

    pub fn render(&self) -> String {
        let mut rendered = String::from("Planned changes:");

        for change in &self.changes {
            rendered.push_str("\n\n");

            match change {
                StagedChange::ExposeSkill {
                    skill_name,
                    agent_id,
                    source_path,
                    target_path,
                    connection,
                } => {
                    let _ = write!(
                        rendered,
                        "  Expose {skill_name} to {}\n    source: {}\n    target: {}\n    connection: {}",
                        agent_id.0,
                        source_path.display(),
                        target_path.display(),
                        render_connection(*connection),
                    );
                }
                StagedChange::DetachSkill {
                    skill_name,
                    agent_id,
                    target_path,
                } => {
                    let _ = write!(
                        rendered,
                        "  Remove {skill_name} from {} [symlink]\n    target: {}",
                        agent_id.0,
                        target_path.display(),
                    );
                }
                StagedChange::DeletePhysicalCopy {
                    skill_name,
                    agent_id,
                    target_path,
                } => {
                    let _ = write!(
                        rendered,
                        "  ⚠ DELETE physical copy of {skill_name} from {}\n    target: {}\n    This will permanently delete the target directory and all its contents.",
                        agent_id.0,
                        target_path.display(),
                    );
                }
            }
        }

        rendered
    }
}

fn render_connection(connection: ConnectionKind) -> &'static str {
    match connection {
        ConnectionKind::Symlink => "symlink",
        ConnectionKind::PhysicalCopy => "physical copy",
        ConnectionKind::Missing => "missing",
        ConnectionKind::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn render_expose_symlink_plan() {
        let plan = ChangePlan::new(vec![StagedChange::ExposeSkill {
            skill_name: "repo-a/code-review".to_string(),
            agent_id: AgentId("Claude".to_string()),
            source_path: PathBuf::from("/Users/me/skills/repo-a/code-review"),
            target_path: PathBuf::from("/Users/me/.claude/skills/code-review"),
            connection: ConnectionKind::Symlink,
        }]);

        let rendered = plan.render();

        assert!(rendered.contains("source:"));
        assert!(rendered.contains("target:"));
        assert!(rendered.contains("connection: symlink"));
    }

    #[test]
    fn render_delete_physical_plan() {
        let plan = ChangePlan::new(vec![StagedChange::DeletePhysicalCopy {
            skill_name: "repo-a/analysis".to_string(),
            agent_id: AgentId("Copilot".to_string()),
            target_path: PathBuf::from("/Users/me/.copilot/skills/analysis"),
        }]);

        let rendered = plan.render();

        assert!(rendered.contains("⚠"));
        assert!(
            rendered.contains(
                "This will permanently delete the target directory and all its contents."
            )
        );
    }

    #[test]
    fn has_physical_deletes_true() {
        let plan = ChangePlan::new(vec![StagedChange::DeletePhysicalCopy {
            skill_name: "repo-a/analysis".to_string(),
            agent_id: AgentId("Copilot".to_string()),
            target_path: PathBuf::from("/Users/me/.copilot/skills/analysis"),
        }]);

        assert!(plan.has_physical_deletes());
    }

    #[test]
    fn has_physical_deletes_false() {
        let plan = ChangePlan::new(vec![StagedChange::DetachSkill {
            skill_name: "repo-a/docs".to_string(),
            agent_id: AgentId("Codex".to_string()),
            target_path: PathBuf::from("/Users/me/.codex/skills/docs"),
        }]);

        assert!(!plan.has_physical_deletes());
    }
}
