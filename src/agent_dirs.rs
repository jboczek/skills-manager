use crate::domain::{AgentDefinition, AgentId};

pub fn default_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            id: AgentId("claude".to_string()),
            display_name: "Claude".to_string(),
            global_dir: None,
            project_dir: None,
            shared_target_ids: vec![".agents".to_string()],
            enabled: true,
        },
        AgentDefinition {
            id: AgentId("codex".to_string()),
            display_name: "Codex".to_string(),
            global_dir: None,
            project_dir: None,
            shared_target_ids: vec![".agents".to_string()],
            enabled: true,
        },
        AgentDefinition {
            id: AgentId("copilot".to_string()),
            display_name: "Copilot".to_string(),
            global_dir: None,
            project_dir: None,
            shared_target_ids: vec![".agents".to_string()],
            enabled: true,
        },
    ]
}
