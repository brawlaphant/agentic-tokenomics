use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("Collection {id} not found")]
    CollectionNotFound { id: u64 },

    #[error("Challenge {id} not found")]
    ChallengeNotFound { id: u64 },

    #[error("Invalid collection status: expected {expected}, got {actual}")]
    InvalidCollectionStatus { expected: String, actual: String },

    #[error("Insufficient bond: required {required}, sent {sent}")]
    InsufficientBond { required: String, sent: String },

    #[error("Insufficient deposit: required {required}, sent {sent}")]
    InsufficientDeposit { required: String, sent: String },

    #[error("Wrong denomination: expected {expected}, got {got}")]
    WrongDenom { expected: String, got: String },

    #[error("Curator has reached maximum collections ({max})")]
    MaxCollectionsExceeded { max: u32 },

    #[error("Activation delay has not elapsed yet")]
    ActivationDelayNotElapsed,

    #[error("Collection has pending challenges")]
    PendingChallenges,

    #[error("Batch {batch_denom} quality score {score} is below minimum {min}")]
    QualityScoreTooLow {
        batch_denom: String,
        score: u32,
        min: u32,
    },

    #[error("Batch {batch_denom} has no quality score on record")]
    NoQualityScore { batch_denom: String },

    #[error("Batch {batch_denom} is already in collection {collection_id}")]
    BatchAlreadyInCollection {
        batch_denom: String,
        collection_id: u64,
    },

    #[error("Batch {batch_denom} is not in collection {collection_id}")]
    BatchNotInCollection {
        batch_denom: String,
        collection_id: u64,
    },

    #[error("Challenge already resolved")]
    ChallengeAlreadyResolved,

    #[error("Quality score must be between 0 and 1000, got {value}")]
    InvalidScore { value: u32 },

    #[error("Confidence must be between 0 and 1000, got {value}")]
    InvalidConfidence { value: u32 },

    #[error("Bond is below minimum after slash — collection suspended")]
    BondBelowMinimum,

    #[error("Unbonding period has not elapsed yet")]
    UnbondingNotComplete,
}
