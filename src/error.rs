use thiserror::Error;

/// Domain errors with user-facing messages; bubbled up through anyhow.
#[derive(Debug, Error)]
pub enum DpError {
    #[error("task #{0} not found")]
    TaskNotFound(i64),
    #[error("plan #{0} not found")]
    PlanNotFound(i64),
    #[error("ADR #{0} not found")]
    AdrNotFound(i64),
    #[error("cannot supersede a ADR with itself (#{0})")]
    SelfSupersede(i64),
    #[error("ADR #{0} is already superseded by #{1}")]
    AlreadySuperseded(i64, i64),
    #[error("ADR #{0} is not accepted yet — accept it before it supersedes #{1}")]
    NotAccepted(i64, i64),
    #[error("ADR #{0}: rationale is required when changing a verdict (accept↔reject)")]
    RationaleRequired(i64),
}
