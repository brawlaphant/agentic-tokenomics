use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("Agreement {id} not found")]
    AgreementNotFound { id: u64 },

    #[error("Invalid agreement status: expected {expected}, got {actual}")]
    InvalidStatus { expected: String, actual: String },

    #[error("Milestone count must be between 1 and {max}, got {got}")]
    InvalidMilestoneCount { max: u32, got: u32 },

    #[error("Milestone payments must sum to escrow amount: payments={payments}, escrow={escrow}")]
    MilestonePaymentMismatch { payments: String, escrow: String },

    #[error("Invalid milestone index: expected {expected}, got {got}")]
    InvalidMilestoneIndex { expected: u32, got: u32 },

    #[error("Milestone {index} is not in {expected_status} status")]
    InvalidMilestoneStatus {
        index: u32,
        expected_status: String,
    },

    #[error("Insufficient funds: required {required}, sent {sent}")]
    InsufficientFunds { required: String, sent: String },

    #[error("Wrong denomination: expected {expected}, got {got}")]
    WrongDenom { expected: String, got: String },

    #[error("Max revisions ({max}) exceeded for milestone {index}")]
    MaxRevisionsExceeded { max: u32, index: u32 },

    #[error("Review period has not expired yet")]
    ReviewPeriodNotExpired,

    #[error("Dispute already exists for this agreement")]
    DisputeAlreadyExists,

    #[error("No active dispute on this agreement")]
    NoActiveDispute,

    #[error("Split percent must be between 1 and 99, got {got}")]
    InvalidSplitPercent { got: u32 },

    #[error("Provider cannot be the same as client")]
    SelfAgreement,

    #[error("Bond ratio out of range: {value} bps (allowed {min}-{max})")]
    BondRatioOutOfRange { value: u64, min: u64, max: u64 },

    #[error("Fee rate out of range: {value} bps (allowed {min}-{max})")]
    FeeRateOutOfRange { value: u64, min: u64, max: u64 },
}
