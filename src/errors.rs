use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum SkillsError {
    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("target already exists: {0}")]
    TargetAlreadyExists(PathBuf),

    #[error("refusing to delete physical copy without explicit confirmation: {0}")]
    PhysicalDeleteRequiresConfirmation(PathBuf),

    #[error("unknown skill origin")]
    UnknownOrigin,
}
