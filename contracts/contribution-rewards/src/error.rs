use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("Mechanism is not in expected status: expected {expected}, got {actual}")]
    InvalidMechanismStatus { expected: String, actual: String },

    #[error("Mechanism already initialized")]
    AlreadyInitialized,

    #[error("Commitment {id} not found")]
    CommitmentNotFound { id: u64 },

    #[error("Commitment {id} is not in {expected} status")]
    InvalidCommitmentStatus { id: u64, expected: String },

    #[error("Commitment {id} has not matured yet (matures at {matures_at})")]
    CommitmentNotMatured { id: u64, matures_at: String },

    #[error("Insufficient funds: required {required}, sent {sent}")]
    InsufficientFunds { required: String, sent: String },

    #[error("Wrong denomination: expected {expected}, got {got}")]
    WrongDenom { expected: String, got: String },

    #[error("Lock months {months} out of range ({min}-{max})")]
    InvalidLockMonths { months: u64, min: u64, max: u64 },

    #[error("Amount {amount} below minimum commitment {min}")]
    BelowMinCommitment { amount: String, min: String },

    #[error("Activity weights must sum to 10000 bps, got {sum}")]
    InvalidWeightSum { sum: u64 },

    #[error("Distribution already executed for period {period}")]
    AlreadyDistributed { period: u32 },

    #[error("No activity recorded for period {period}")]
    NoActivityForPeriod { period: u32 },

    #[error("Distribution record not found for period {period}")]
    DistributionNotFound { period: u32 },

    #[error("Invalid period range: from {from} to {to}")]
    InvalidPeriodRange { from: u32, to: u32 },

    #[error("No funds attached")]
    NoFundsAttached,

    #[error("Zero inflow amount")]
    ZeroInflow,

    #[error("No claimable rewards for {address}")]
    NoClaimableRewards { address: String },
}
