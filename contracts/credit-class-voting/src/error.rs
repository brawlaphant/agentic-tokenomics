use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("Proposal {id} not found")]
    ProposalNotFound { id: u64 },

    #[error("Invalid proposal status: expected {expected}, got {actual}")]
    InvalidStatus { expected: String, actual: String },

    #[error("Insufficient funds: required {required}, sent {sent}")]
    InsufficientFunds { required: String, sent: String },

    #[error("Wrong denomination: expected {expected}, got {got}")]
    WrongDenom { expected: String, got: String },

    #[error("Credit type must not be empty")]
    EmptyCreditType,

    #[error("Methodology IRI must not be empty")]
    EmptyMethodologyIri,

    #[error("Agent score must be between 0 and 1000, got {score}")]
    InvalidAgentScore { score: u32 },

    #[error("Agent confidence must be between 0 and 1000, got {confidence}")]
    InvalidAgentConfidence { confidence: u32 },

    #[error("Voting period has not ended yet")]
    VotingPeriodNotEnded,

    #[error("Voting period has ended")]
    VotingPeriodEnded,

    #[error("Already voted on proposal {id}")]
    AlreadyVoted { id: u64 },

    #[error("Override window has expired")]
    OverrideWindowExpired,

    #[error("Agent review has timed out")]
    AgentReviewTimedOut,
}
