use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("Composition violation: category {category} would have {count} validators, minimum is {min}")]
    CompositionViolation {
        category: String,
        count: u32,
        min: u32,
    },

    #[error("Cannot remove validator: active count {active} would fall below minimum {min}")]
    BelowMinValidators { active: u32, min: u32 },

    #[error("Maximum validators reached: {max} active validators already")]
    AboveMaxValidators { max: u32 },

    #[error("Validator term has expired")]
    TermExpired,

    #[error("Validator is already in {status} status")]
    AlreadyInStatus { status: String },

    #[error("Invalid validator status: expected {expected}, got {actual}")]
    InvalidStatus { expected: String, actual: String },

    #[error("Validator {address} not found")]
    ValidatorNotFound { address: String },

    #[error("Validator already exists: {address}")]
    ValidatorAlreadyExists { address: String },

    #[error("Performance score {score} is above threshold {threshold}, probation not warranted")]
    ScoreAboveThreshold { score: u64, threshold: u64 },

    #[error("Performance score {score} is still below threshold {threshold}")]
    ScoreBelowThreshold { score: u64, threshold: u64 },

    #[error("Basis points value {value} out of range (0-10000)")]
    InvalidBasisPoints { value: u64 },

    #[error("No compensation due for this validator")]
    NoCompensationDue,

    #[error("Insufficient fund balance: required {required}, available {available}")]
    InsufficientFund { required: String, available: String },

    #[error("Term has not ended yet")]
    TermNotEnded,

    #[error("Weights must sum to 10000 bps, got {total}")]
    InvalidWeightSum { total: u64 },

    #[error("Wrong denomination: expected {expected}, got {got}")]
    WrongDenom { expected: String, got: String },
}
