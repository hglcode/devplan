use thiserror::Error;

/// Domain errors with user-facing messages; bubbled up through anyhow.
#[derive(Debug, Error)]
pub enum DpError {
    #[error("todo #{0} not found")]
    TaskNotFound(i64),
    #[error("plan #{0} not found")]
    PlanNotFound(i64),
    #[error("decision #{0} not found")]
    DecisionNotFound(i64),
    #[error("cannot supersede a decision with itself (#{0})")]
    SelfSupersede(i64),
    #[error("decision #{0} is already superseded by #{1}")]
    AlreadySuperseded(i64, i64),
    #[error("decision #{0} is not accepted yet — accept it before it supersedes #{1}")]
    NotAccepted(i64, i64),
}
