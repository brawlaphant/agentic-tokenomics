use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};

/// Transaction types for ecological credit operations.
#[cw_serde]
pub enum TxType {
    /// Credit issuance (MsgCreateBatch) — default rate 2%
    CreditIssuance,
    /// Credit transfer (MsgSend) — default rate 0.1%
    CreditTransfer,
    /// Credit retirement (MsgRetire) — default rate 0.5%
    CreditRetirement,
    /// Marketplace trade (MsgBuySellOrder) — default rate 1%
    MarketplaceTrade,
}

/// Instantiate the fee router with initial configuration.
///
/// All fee rates must be in [0, 0.10] (max 10%).
/// Distribution shares must sum to exactly 1.0.
#[cw_serde]
pub struct InstantiateMsg {
    /// Fee rate for credit issuance transactions
    pub issuance_rate: Decimal,
    /// Fee rate for credit transfer transactions
    pub transfer_rate: Decimal,
    /// Fee rate for credit retirement transactions
    pub retirement_rate: Decimal,
    /// Fee rate for marketplace trade transactions
    pub trade_rate: Decimal,
    /// Share of fees routed to burn pool (e.g., 0.30 = 30%)
    pub burn_share: Decimal,
    /// Share of fees routed to validator fund (e.g., 0.40 = 40%)
    pub validator_share: Decimal,
    /// Share of fees routed to community pool (e.g., 0.25 = 25%)
    pub community_share: Decimal,
    /// Share of fees routed to agent infrastructure fund (e.g., 0.05 = 5%)
    pub agent_share: Decimal,
    /// Minimum fee floor in uregen
    pub min_fee: Uint128,
}

/// Execute messages for the fee router contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Collect a fee for a credit transaction.
    /// Calculates fee = max(value * rate, min_fee) and distributes to pools.
    CollectFee {
        tx_type: TxType,
        value: Uint128,
    },
    /// Update the fee rate for a specific transaction type (admin only).
    /// Rate must be in [0, 0.10].
    UpdateFeeRate {
        tx_type: TxType,
        rate: Decimal,
    },
    /// Update the distribution shares (admin only).
    /// Shares must sum to exactly 1.0.
    UpdateDistribution {
        burn_share: Decimal,
        validator_share: Decimal,
        community_share: Decimal,
        agent_share: Decimal,
    },
}

/// Query messages for the fee router contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the current fee configuration.
    #[returns(FeeConfigResponse)]
    FeeConfig {},
    /// Returns current pool balances.
    #[returns(PoolBalancesResponse)]
    PoolBalances {},
    /// Calculate fee for a transaction without executing (dry run).
    #[returns(CalculateFeeResponse)]
    CalculateFee {
        tx_type: TxType,
        value: Uint128,
    },
}

/// Response for FeeConfig query.
#[cw_serde]
pub struct FeeConfigResponse {
    pub admin: String,
    pub issuance_rate: Decimal,
    pub transfer_rate: Decimal,
    pub retirement_rate: Decimal,
    pub trade_rate: Decimal,
    pub burn_share: Decimal,
    pub validator_share: Decimal,
    pub community_share: Decimal,
    pub agent_share: Decimal,
    pub min_fee: Uint128,
}

/// Response for PoolBalances query.
#[cw_serde]
pub struct PoolBalancesResponse {
    pub burn_pool: Uint128,
    pub validator_fund: Uint128,
    pub community_pool: Uint128,
    pub agent_infra: Uint128,
}

/// Response for CalculateFee query.
#[cw_serde]
pub struct CalculateFeeResponse {
    pub fee_amount: Uint128,
    pub min_fee_applied: bool,
    pub burn_amount: Uint128,
    pub validator_amount: Uint128,
    pub community_amount: Uint128,
    pub agent_amount: Uint128,
}
