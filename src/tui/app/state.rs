use std::path::PathBuf;

use crate::domain::InventoryRow;
use crate::inventory::AgentTarget;
use crate::plan::ChangePlan;
use crate::scanner::ScanResult;
use crate::source::{AcquireOutcome, SourcePreview};

#[derive(Debug, Clone, Default)]
pub enum SourceAddStep {
    #[default]
    EnterUrl,
    Confirm {
        preview: SourcePreview,
    },
    SelectSkill {
        source_path: PathBuf,
        skills: Vec<ScanResult>,
        outcome: AcquireOutcome,
    },
    Done {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub enum ImportStep {
    Disambiguate {
        matches: Vec<ScanResult>,
    },
    SelectAgents {
        selected: Box<ScanResult>,
        agents: Vec<AgentSelectionItem>,
        focused: usize,
    },
    ConfirmPlan {
        plan: ChangePlan,
        selected: Box<ScanResult>,
        target_agents: Vec<AgentTarget>,
    },
    ConfirmPhysical {
        plan: ChangePlan,
    },
    Done {
        message: String,
    },
}

impl Default for ImportStep {
    fn default() -> Self {
        Self::Done {
            message: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RemoveStep {
    SelectExposure {
        selected: Box<InventoryRow>,
    },
    ConfirmPlan {
        plan: ChangePlan,
        selected: Box<InventoryRow>,
    },
    ConfirmPhysical {
        plan: ChangePlan,
    },
    Done {
        message: String,
    },
}

impl Default for RemoveStep {
    fn default() -> Self {
        Self::Done {
            message: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Home,
    List,
    Scan,
    SourceAdd,
    Config,
    Help,
    Import,
    Remove,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingLoad {
    List,
    Scan,
}

#[derive(Debug, Clone)]
pub struct AgentSelectionItem {
    pub target: AgentTarget,
    pub checked: bool,
}
