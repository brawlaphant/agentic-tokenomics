use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Item;

/// Fee configuration: rates per transaction type, distribution shares, and minimum fee.
///
/// Rates are stored as Decimal values in [0, 0.10] (max 10%).
/// Distribution shares must sum to exactly 1.0 (Share Sum Unity invariant).
#[cw_serde]
pub struct FeeConfig {
    /// Admin address (governance module)
    pub admin: Addr,
    /// Fee rate for credit issuance transactions
    pub issuance_rate: Decimal,
    /// Fee rate for credit transfer transactions
    pub transfer_rate: Decimal,
    /// Fee rate for credit retirement transactions
    pub retirement_rate: Decimal,
    /// Fee rate for marketplace trade transactions
    pub trade_rate: Decimal,
    /// Share of fees routed to the burn pool
    pub burn_share: Decimal,
    /// Share of fees routed to the validator fund
    pub validator_share: Decimal,
    /// Share of fees routed to the community pool
    pub community_share: Decimal,
    /// Share of fees routed to the agent infrastructure fund
    pub agent_share: Decimal,
    /// Minimum fee floor in uregen (1 REGEN = 1,000,000 uregen)
    pub min_fee: Uint128,
}

/// Pool balances tracking accumulated fee distributions.
///
/// All values are in uregen. The Fee Conservation invariant guarantees
/// that total fees collected == burn_pool + validator_fund + community_pool + agent_infra.
///
/// Pools are write-only in this contract: `CollectFee` adds to pools but no
/// execute message withdraws from them. Distribution to downstream mechanisms
/// (M012 burn, M014 validator compensation, M015 contributor rewards) happens
/// via cross-contract calls from those modules, preserving Pool Isolation
/// (SPEC security invariant #5).
#[cw_serde]
pub struct PoolBalances {
    /// Accumulated burn pool balance (tokens queued for burn via M012)
    pub burn_pool: Uint128,
    /// Accumulated validator fund balance (distributed via M014)
    pub validator_fund: Uint128,
    /// Accumulated community pool balance (governance-directed spending)
    pub community_pool: Uint128,
    /// Accumulated agent infrastructure fund balance
    pub agent_infra: Uint128,
}

pub const FEE_CONFIG: Item<FeeConfig> = Item::new("fee_config");
pub const POOL_BALANCES: Item<PoolBalances> = Item::new("pool_balances");
