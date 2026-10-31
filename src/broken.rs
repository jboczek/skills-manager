use crate::{
    domain::{AgentId, ConnectionKind, InventoryRow},
    plan::{ChangePlan, StagedChange},
    scanner::ScanResult,
};
use std::{collections::HashSet, path::PathBuf};

pub fn replacements(row: &InventoryRow, catalog: &[ScanResult]) -> Vec<ScanResult> {
    let Some(repo) = &row.source.repo_path else {
        return vec![];
    };
    catalog
        .iter()
        .filter(|s| {
            s.repo_path.as_ref() == Some(repo)
                && s.skill_path
                    .file_name()
                    .is_some_and(|n| n == row.skill_id.name.as_str())
                && s.skill_path.join("SKILL.md").is_file()
        })
        .cloned()
        .collect()
}

pub fn connection_label(row: &InventoryRow, catalog: &[ScanResult]) -> &'static str {
    if replacements(row, catalog).is_empty() {
        "broken (removed)"
    } else {
        "broken (folder changed)"
    }
}

pub fn plan(row: &InventoryRow, replacement: Option<PathBuf>) -> ChangePlan {
    let mut seen = HashSet::new();
    ChangePlan::new(
        row.exposures
            .iter()
            .filter(|e| e.connection == ConnectionKind::Broken && seen.insert(e.path.clone()))
            .filter_map(|e| {
                Some(StagedChange::FixBroken {
                    skill_name: row.skill_id.to_string(),
                    agent_id: AgentId(e.agent_id.0.clone()),
                    target_path: e.path.clone(),
                    old_source: crate::inventory::link_target(&e.path)?,
                    replacement: replacement.clone(),
                })
            })
            .collect(),
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::{
        domain::{Scope, SkillExposure, SkillId, SkillSource},
        plan_apply::apply_plan,
    };
    use std::fs;
    #[test]
    fn repairs_all_links_or_removes_without_touching_source_and_refuses_changed_link() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("new/code");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "test").unwrap();
        let old = temp.path().join("old/code");
        let links = [temp.path().join("codex"), temp.path().join("claude")];
        for link in &links {
            std::os::unix::fs::symlink(&old, link).unwrap();
        }
        let row = InventoryRow {
            skill_id: SkillId {
                namespace: "repo".into(),
                name: "code".into(),
            },
            source: SkillSource {
                repo_path: Some(temp.path().into()),
                repo_name: Some("repo".into()),
                remote_url: None,
            },
            scope: Scope::Global,
            disambiguation_index: None,
            exposures: links
                .iter()
                .map(|p| SkillExposure {
                    agent_id: AgentId("codex".into()),
                    path: p.clone(),
                    connection: ConnectionKind::Broken,
                })
                .collect(),
        };
        let repair = plan(&row, Some(source.clone()));
        assert_eq!(repair.changes.len(), 2);
        assert!(repair.render().contains(&old.display().to_string()));
        assert!(apply_plan(&repair).failed.is_none());
        for link in &links {
            assert_eq!(
                fs::canonicalize(link).unwrap(),
                fs::canonicalize(&source).unwrap()
            );
        }
        assert!(apply_plan(&repair).failed.is_some());
        for link in &links {
            fs::remove_file(link).unwrap();
            std::os::unix::fs::symlink(&old, link).unwrap();
        }
        fs::remove_file(source.join("SKILL.md")).unwrap();
        assert!(apply_plan(&repair).failed.is_some());
        assert_eq!(fs::read_link(&links[0]).unwrap(), old);
        fs::write(source.join("SKILL.md"), "test").unwrap();
        assert!(apply_plan(&plan(&row, None)).failed.is_none());
        assert!(source.join("SKILL.md").exists());
        for link in &links {
            assert!(fs::symlink_metadata(link).is_err());
        }
    }
}
