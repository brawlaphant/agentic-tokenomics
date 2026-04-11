use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("Attestation {id} not found")]
    AttestationNotFound { id: u64 },

    #[error("Challenge {id} not found")]
    ChallengeNotFound { id: u64 },

    #[error("Invalid attestation status: expected {expected}, got {actual}")]
    InvalidStatus { expected: String, actual: String },

    #[error("Bond amount {sent} below minimum {required} for {attestation_type}")]
    BondBelowMinimum {
        sent: String,
        required: String,
        attestation_type: String,
    },

    #[error("Challenge deposit {sent} below minimum {required}")]
    ChallengeDepositBelowMinimum { sent: String, required: String },

    #[error("Challenge window has closed")]
    ChallengeWindowClosed,

    #[error("Active challenge already exists for attestation {attestation_id}")]
    ActiveChallengePending { attestation_id: u64 },

    #[error("No active challenge on attestation {attestation_id}")]
    NoActiveChallenge { attestation_id: u64 },

    #[error("Lock period has not expired yet")]
    LockPeriodNotExpired,

    #[error("IRI must not be empty")]
    EmptyIri,

    #[error("Insufficient funds: required {required}, sent {sent}")]
    InsufficientFunds { required: String, sent: String },

    #[error("Wrong denomination: expected {expected}, got {got}")]
    WrongDenom { expected: String, got: String },

    #[error("Resolver cannot be the attester")]
    ResolverIsAttester,

    #[error("Resolver cannot be the challenger")]
    ResolverIsChallenger,

    #[error("Fee rate out of range: {value} bps (allowed {min}-{max})")]
    FeeRateOutOfRange { value: u64, min: u64, max: u64 },
}
