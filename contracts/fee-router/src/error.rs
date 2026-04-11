use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Distribution shares must sum to 1.0, got {sum}")]
    ShareSumNotUnity { sum: String },

    #[error("Unauthorized: only admin can perform this action")]
    Unauthorized {},

    #[error("Transaction value must be greater than zero")]
    ZeroValue {},

    #[error("Fee rate {rate} exceeds maximum cap of 0.10 (10%)")]
    RateExceedsCap { rate: String },
}
