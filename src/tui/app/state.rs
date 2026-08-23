use std::path::PathBuf;

use crate::domain::InventoryRow;
use crate::git::RepositoryUpdate;
use crate::inventory::AgentTarget;
use crate::plan::ChangePlan;
use crate::scanner::ScanResult;
use crate::source::{AcquireOutcome, SourcePreview};

#[derive(Debug, Clone)]
pub enum SourceAddStep {
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

impl Default for SourceAddStep {
    fn default() -> Self {
        Self::Done {
            message: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ImportStep {
    Disambiguate {
        matches: Vec<ScanResult>,
    },
    SelectAgents {
        selected: Vec<ScanResult>,
        agents: Vec<AgentSelectionItem>,
        focused: usize,
    },
    ConfirmPlan {
        plan: ChangePlan,
        selected: Vec<ScanResult>,
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

#[derive(Debug, Clone)]
pub enum RepositoryUpdateStep {
    Preview {
        update: RepositoryUpdate,
        scroll: usize,
    },
    Done {
        message: String,
    },
}

impl Default for RepositoryUpdateStep {
    fn default() -> Self {
        Self::Done {
            message: String::new(),
        }
    }
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
    SourceAdd,
    Config,
    Help,
    Import,
    Remove,
    RepositoryUpdate,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingLoad {
    List,
}

#[derive(Debug, Clone)]
pub struct AgentSelectionItem {
    pub target: AgentTarget,
    pub checked: bool,
}
