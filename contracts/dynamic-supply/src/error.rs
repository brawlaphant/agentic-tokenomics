use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: only admin can perform this action")]
    Unauthorized {},

    #[error("Supply cap must be greater than zero")]
    ZeroCap {},

    #[error("Current supply exceeds the hard cap ({current} > {cap})")]
    SupplyExceedsCap { current: String, cap: String },

    #[error("Regrowth rate {rate} exceeds maximum bound of 0.10 (10%)")]
    RegrowthRateExceedsBound { rate: String },

    #[error("Burn amount must be greater than zero")]
    ZeroBurnAmount {},

    #[error("Mint amount must be greater than zero")]
    ZeroMintAmount {},

    #[error("Ecological reference value must be greater than zero")]
    ZeroReferenceValue {},

    #[error("Invalid M014 phase: {phase}")]
    InvalidPhase { phase: String },

    #[error("Mint would exceed hard cap (supply {supply} + mint {mint} > cap {cap})")]
    MintExceedsCap {
        supply: String,
        mint: String,
        cap: String,
    },

    #[error("Burn amount {burn} exceeds current supply {supply}")]
    BurnExceedsSupply { burn: String, supply: String },
}
