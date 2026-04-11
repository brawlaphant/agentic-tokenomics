use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("unauthorized: only admin can perform this action")]
    Unauthorized {},

    #[error("invalid endorsement level {level}: must be between 1 and 5")]
    InvalidEndorsementLevel { level: u8 },

    #[error("signal not found: {id}")]
    SignalNotFound { id: u64 },

    #[error("challenge not found: {id}")]
    ChallengeNotFound { id: u64 },

    #[error("signal {id} is in terminal state and cannot be modified")]
    SignalTerminal { id: u64 },

    #[error("signal {id} is not in a challengeable state")]
    SignalNotChallengeable { id: u64 },

    #[error("signal {id} already has an active challenge")]
    SignalAlreadyChallenged { id: u64 },

    #[error("cannot challenge your own signal")]
    SelfChallenge {},

    #[error("challenge window expired for signal {id}")]
    ChallengeWindowExpired { id: u64 },

    #[error("insufficient bond: required {required}, sent {sent}")]
    InsufficientBond { required: String, sent: String },

    #[error("wrong bond denomination: expected {expected}, got {got}")]
    WrongBondDenom { expected: String, got: String },

    #[error("evidence must include at least one koi_link or ledger_ref")]
    InsufficientEvidence {},

    #[error("rationale must be at least 50 characters")]
    RationaleTooShort {},

    #[error("only the original signaler can withdraw signal {id}")]
    NotSignalOwner { id: u64 },

    #[error("cannot withdraw signal {id}: currently challenged")]
    WithdrawWhileChallenged { id: u64 },

    #[error("challenge {id} is not pending resolution")]
    ChallengeNotPending { id: u64 },

    #[error("invalidation rationale is required")]
    InvalidationRationaleRequired {},

    #[error("signal {id} is not yet active (still in activation delay)")]
    SignalNotYetActive { id: u64 },

    #[error("challenge {id} resolution deadline has not been exceeded")]
    DeadlineNotExceeded { id: u64 },

    #[error("only admin or arbiter can resolve challenges")]
    NotResolver {},
}
