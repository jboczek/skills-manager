use std::path::Path;

use crate::domain::ConnectionKind;
use crate::plan::{ChangePlan, StagedChange};
use crate::symlink;

pub struct ApplyResult {
    pub applied: Vec<StagedChange>,
    pub failed: Option<(StagedChange, anyhow::Error)>,
}

/// Apply all changes in the plan sequentially.
/// Stops on first failure and reports what was applied before the failure.
pub fn apply_plan(plan: &ChangePlan) -> ApplyResult {
    let mut applied = Vec::new();
    for change in &plan.changes {
        match apply_change(change) {
            Ok(()) => applied.push(change.clone()),
            Err(err) => {
                return ApplyResult {
                    applied,
                    failed: Some((change.clone(), err)),
                };
            }
        }
    }
    ApplyResult {
        applied,
        failed: None,
    }
}

fn apply_change(change: &StagedChange) -> anyhow::Result<()> {
    match change {
        StagedChange::ExposeSkill {
            source_path,
            target_path,
            connection,
            ..
        } => match connection {
            ConnectionKind::Symlink => symlink::create_symlink(source_path, target_path),
            ConnectionKind::PhysicalCopy => copy_dir_all(source_path, target_path),
            _ => Err(anyhow::anyhow!("unsupported connection kind for expose")),
        },
        StagedChange::DetachSkill { target_path, .. } => symlink::remove_symlink(target_path),
        StagedChange::DeletePhysicalCopy { target_path, .. } => {
            symlink::remove_physical_copy(target_path)
        }
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::{Builder, TempDir};

    use super::*;
    use crate::constants::{AGENT_NAME_CLAUDE, AGENT_NAME_CODEX, AGENT_NAME_COPILOT};
    use crate::domain::AgentId;

    fn test_dir(name: &str) -> TempDir {
        let root = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test-artifacts")
            .join("plan-apply");
        fs::create_dir_all(&root).unwrap();
        Builder::new().prefix(name).tempdir_in(root).unwrap()
    }

    #[test]
    fn apply_expose_symlink_creates_link() {
        let temp = test_dir("apply-expose-");
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        fs::create_dir_all(&source).unwrap();

        let result = apply_plan(&ChangePlan::new(vec![StagedChange::ExposeSkill {
            skill_name: "repo-a/code-review".to_string(),
            agent_id: AgentId(AGENT_NAME_CLAUDE.to_string()),
            source_path: source.clone(),
            target_path: target.clone(),
            connection: ConnectionKind::Symlink,
        }]));

        assert!(result.failed.is_none());
        assert!(symlink::is_symlink(&target).unwrap());
        assert_eq!(symlink::read_symlink_target(&target).unwrap(), source);
    }

    #[test]
    fn apply_detach_removes_symlink_not_source() {
        let temp = test_dir("apply-detach-");
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        fs::create_dir_all(&source).unwrap();
        symlink::create_symlink(&source, &target).unwrap();

        let result = apply_plan(&ChangePlan::new(vec![StagedChange::DetachSkill {
            skill_name: "repo-a/docs".to_string(),
            agent_id: AgentId(AGENT_NAME_CODEX.to_string()),
            target_path: target.clone(),
        }]));

        assert!(result.failed.is_none());
        assert!(!target.exists());
        assert!(source.exists());
    }

    #[test]
    fn apply_delete_physical_removes_dir() {
        let temp = test_dir("apply-delete-");
        let target = temp.path().join("target");
        fs::create_dir_all(target.join("nested")).unwrap();
        fs::write(target.join("nested").join("SKILL.md"), "# skill").unwrap();

        let result = apply_plan(&ChangePlan::new(vec![StagedChange::DeletePhysicalCopy {
            skill_name: "repo-a/analysis".to_string(),
            agent_id: AgentId(AGENT_NAME_COPILOT.to_string()),
            target_path: target.clone(),
        }]));

        assert!(result.failed.is_none());
        assert!(!target.exists());
    }

    #[test]
    fn apply_stops_on_failure_and_reports_partial() {
        let temp = test_dir("apply-partial-");
        let source_one = temp.path().join("source-one");
        let target_one = temp.path().join("target-one");
        let source_two = temp.path().join("source-two");
        let target_two = temp.path().join("target-two");
        fs::create_dir_all(&source_one).unwrap();
        fs::create_dir_all(&source_two).unwrap();
        fs::create_dir_all(&target_two).unwrap();

        let first = StagedChange::ExposeSkill {
            skill_name: "repo-a/code-review".to_string(),
            agent_id: AgentId(AGENT_NAME_CLAUDE.to_string()),
            source_path: source_one,
            target_path: target_one.clone(),
            connection: ConnectionKind::Symlink,
        };
        let second = StagedChange::ExposeSkill {
            skill_name: "repo-a/docs".to_string(),
            agent_id: AgentId(AGENT_NAME_CODEX.to_string()),
            source_path: source_two,
            target_path: target_two.clone(),
            connection: ConnectionKind::Symlink,
        };

        let result = apply_plan(&ChangePlan::new(vec![first.clone(), second.clone()]));

        assert_eq!(result.applied, vec![first]);
        let (failed_change, _) = result.failed.unwrap();
        assert_eq!(failed_change, second);
        assert!(symlink::is_symlink(&target_one).unwrap());
        assert!(target_two.is_dir());
    }
}
