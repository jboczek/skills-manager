use std::fs;
use std::path::Path;

use crate::constants::AGENT_ID_CODEX;
use crate::domain::{InventoryRow, Scope, SkillExposure};
use crate::inventory::AgentTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextEstimate {
    pub agent_id: String,
    pub display_name: String,
    pub token_count: usize,
    pub skill_count: usize,
}

pub fn estimate_global_context(
    rows: &[InventoryRow],
    agents: &[AgentTarget],
) -> Vec<ContextEstimate> {
    agents
        .iter()
        .filter(|agent| agent.enabled)
        .map(|agent| {
            let (token_count, skill_count) = rows
                .iter()
                .filter(|row| row.scope == Scope::Global)
                .filter_map(|row| {
                    selected_exposure(row, &agent.agent_id).map(|exposure| (row, exposure))
                })
                .map(|(row, exposure)| {
                    let metadata = read_metadata(&exposure.path, &row.skill_id.name);
                    let text = discovery_text(&metadata, &agent.agent_id, &exposure.path);
                    tiktoken_rs::o200k_base_singleton()
                        .encode_ordinary(&text)
                        .len()
                })
                .fold((0usize, 0usize), |(tokens, skills), count| {
                    (tokens + count, skills + 1)
                });

            ContextEstimate {
                agent_id: agent.agent_id.clone(),
                display_name: agent.display_name.clone(),
                token_count,
                skill_count,
            }
        })
        .collect()
}

fn selected_exposure<'a>(row: &'a InventoryRow, agent_id: &str) -> Option<&'a SkillExposure> {
    row.exposures
        .iter()
        .filter(|exposure| {
            exposure.agent_id.0 == agent_id
                && exposure.connection != crate::domain::ConnectionKind::Broken
        })
        .min_by_key(|exposure| exposure.path.join("SKILL.md"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SkillMetadata {
    name: String,
    description: String,
}

fn read_metadata(skill_path: &Path, fallback_name: &str) -> SkillMetadata {
    let fallback = || SkillMetadata {
        name: fallback_name.to_string(),
        description: String::new(),
    };

    let Ok(contents) = fs::read_to_string(skill_path.join("SKILL.md")) else {
        return fallback();
    };
    let Some(frontmatter) = extract_frontmatter(&contents) else {
        return fallback();
    };
    let Ok(value) = yaml_serde::from_str::<yaml_serde::Value>(frontmatter) else {
        return fallback();
    };

    let name = value
        .get("name")
        .and_then(yaml_serde::Value::as_str)
        .filter(|name| !name.is_empty())
        .unwrap_or(fallback_name)
        .to_string();
    let description = value
        .get("description")
        .and_then(yaml_serde::Value::as_str)
        .unwrap_or_default()
        .to_string();

    SkillMetadata { name, description }
}

fn extract_frontmatter(contents: &str) -> Option<&str> {
    let mut lines = contents.split_inclusive('\n');
    let first_line = lines.next()?;
    if first_line.trim_end_matches(&['\r', '\n'][..]) != "---" {
        return None;
    }

    let start = first_line.len();
    let mut offset = start;
    for line in lines {
        if line.trim_end_matches(&['\r', '\n'][..]) == "---" {
            return Some(&contents[start..offset]);
        }
        offset += line.len();
    }
    None
}

fn discovery_text(metadata: &SkillMetadata, agent_id: &str, skill_path: &Path) -> String {
    let mut text = if metadata.description.is_empty() {
        metadata.name.clone()
    } else {
        format!("{}\n{}", metadata.name, metadata.description)
    };

    if agent_id == AGENT_ID_CODEX {
        text.push('\n');
        text.push_str(&skill_path.join("SKILL.md").display().to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::*;
    use crate::domain::{
        AgentId, ConnectionKind, InventoryRow, Scope, SkillExposure, SkillId, SkillSource,
    };
    use crate::inventory::AgentTarget;

    fn agent(agent_id: &str, display_name: &str, enabled: bool) -> AgentTarget {
        AgentTarget {
            agent_id: agent_id.to_string(),
            display_name: display_name.to_string(),
            global_dir: None,
            shared_target_dirs: Vec::new(),
            enabled,
        }
    }

    fn row<'a>(
        name: &str,
        scope: Scope,
        exposures: impl IntoIterator<Item = (&'a str, PathBuf)>,
    ) -> InventoryRow {
        InventoryRow {
            skill_id: SkillId {
                namespace: "repo".to_string(),
                name: name.to_string(),
            },
            source: SkillSource {
                repo_name: Some("repo".to_string()),
                repo_path: None,
                remote_url: None,
            },
            scope,
            exposures: exposures
                .into_iter()
                .map(|(agent_id, path)| SkillExposure {
                    agent_id: AgentId(agent_id.to_string()),
                    path,
                    connection: ConnectionKind::Symlink,
                })
                .collect(),
            disambiguation_index: None,
        }
    }

    fn skill(root: &Path, name: &str, content: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir_all(&path).expect("skill directory is created");
        fs::write(path.join("SKILL.md"), content).expect("skill metadata is written");
        path
    }

    fn estimate<'a>(estimates: &'a [ContextEstimate], agent_id: &str) -> &'a ContextEstimate {
        estimates
            .iter()
            .find(|estimate| estimate.agent_id == agent_id)
            .expect("agent estimate")
    }

    #[test]
    fn estimates_only_enabled_global_rows_in_configured_order() {
        let temp = tempdir().expect("temporary directory");
        let first = skill(
            temp.path(),
            "a-first",
            "---\nname: 'Quoted name'\ndescription: >\n  first folded line\n  second folded line\n---\n",
        );
        let later = skill(
            temp.path(),
            "z-later",
            "---\nname: wrong path\ndescription: wrong metadata\n---\n",
        );
        let claude_path = skill(
            temp.path(),
            "claude",
            "---\nname: Claude skill\ndescription: 'Opis z emoji 🚀'\n---\n",
        );
        let copilot_path = skill(
            temp.path(),
            "copilot",
            "---\nname: Claude skill\ndescription: 'Opis z emoji 🚀'\n---\n",
        );
        let project_local = skill(
            temp.path(),
            "project-local",
            "---\nname: Must not count\ndescription: project metadata\n---\n",
        );
        let disabled = skill(
            temp.path(),
            "disabled",
            "---\nname: Disabled skill\ndescription: disabled metadata\n---\n",
        );

        let rows = vec![
            row(
                "fallback-name",
                Scope::Global,
                [
                    ("codex", first.clone()),
                    ("codex", later),
                    ("claude", claude_path.clone()),
                    ("copilot", copilot_path.clone()),
                ],
            ),
            row(
                "local-name",
                Scope::ProjectLocal,
                [("codex", project_local)],
            ),
            row("disabled-name", Scope::Global, [("disabled", disabled)]),
        ];
        let agents = vec![
            agent("codex", "Codex", true),
            agent("claude", "Claude Code", true),
            agent("copilot", "GitHub Copilot", true),
            agent("disabled", "Disabled", false),
        ];

        let estimates = estimate_global_context(&rows, &agents);

        assert_eq!(
            estimates
                .iter()
                .map(|estimate| estimate.agent_id.as_str())
                .collect::<Vec<_>>(),
            vec!["codex", "claude", "copilot"]
        );
        assert_eq!(estimate(&estimates, "codex").skill_count, 1);
        assert_eq!(estimate(&estimates, "claude").skill_count, 1);
        assert_eq!(estimate(&estimates, "copilot").skill_count, 1);

        let codex_text = format!(
            "Quoted name\nfirst folded line second folded line\n{}",
            first.join("SKILL.md").display()
        );
        let claude_text = "Claude skill\nOpis z emoji 🚀";
        assert_eq!(
            estimate(&estimates, "codex").token_count,
            tiktoken_rs::o200k_base_singleton()
                .encode_ordinary(&codex_text)
                .len()
        );
        assert_eq!(
            estimate(&estimates, "claude").token_count,
            tiktoken_rs::o200k_base_singleton()
                .encode_ordinary(claude_text)
                .len()
        );
        assert_eq!(
            estimate(&estimates, "copilot").token_count,
            tiktoken_rs::o200k_base_singleton()
                .encode_ordinary(claude_text)
                .len()
        );
    }

    #[test]
    fn literal_multiline_description_is_parsed_without_reading_the_body() {
        let temp = tempdir().expect("temporary directory");
        let path = skill(
            temp.path(),
            "literal",
            "---\nname: Literal name\ndescription: |\n  first literal line\n  second literal line\n---\n\nbody: not metadata\n",
        );

        let metadata = read_metadata(&path, "fallback");

        assert_eq!(metadata.name, "Literal name");
        assert_eq!(
            metadata.description,
            "first literal line\nsecond literal line\n"
        );
        assert_eq!(
            discovery_text(&metadata, "claude", &path),
            "Literal name\nfirst literal line\nsecond literal line\n"
        );
    }

    #[test]
    fn empty_name_and_non_string_description_use_their_fallbacks() {
        let temp = tempdir().expect("temporary directory");
        let path = skill(
            temp.path(),
            "typed-values",
            "---\nname: ''\ndescription: 42\n---\n",
        );

        let metadata = read_metadata(&path, "inventory-name");

        assert_eq!(metadata.name, "inventory-name");
        assert!(metadata.description.is_empty());
        assert_eq!(
            discovery_text(&metadata, "codex", &path),
            format!("inventory-name\n{}", path.join("SKILL.md").display())
        );
    }

    #[test]
    fn invalid_or_missing_frontmatter_falls_back_to_inventory_name() {
        let temp = tempdir().expect("temporary directory");
        let invalid = skill(temp.path(), "invalid", "---\nname: [unterminated\n---\n");
        let unterminated = skill(temp.path(), "unterminated", "---\nname: ignored\n");
        let missing = temp.path().join("missing");

        let rows = vec![
            row("invalid-name", Scope::Global, [("claude", invalid)]),
            row(
                "unterminated-name",
                Scope::Global,
                [("claude", unterminated)],
            ),
            row("missing-name", Scope::Global, [("claude", missing)]),
        ];
        let estimates = estimate_global_context(&rows, &[agent("claude", "Claude", true)]);

        assert_eq!(estimate(&estimates, "claude").skill_count, 3);
        let expected = ["invalid-name", "unterminated-name", "missing-name"]
            .into_iter()
            .map(|name| {
                tiktoken_rs::o200k_base_singleton()
                    .encode_ordinary(name)
                    .len()
            })
            .sum::<usize>();
        assert_eq!(estimate(&estimates, "claude").token_count, expected);
    }
}
