use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};

use crate::state::{M014Phase, MintBurnRecord, SupplyPhase};

/// Instantiate the dynamic supply contract with initial configuration.
///
/// Sets the hard cap, initial supply, regrowth rate, and M014 phase.
/// Ecological multiplier is disabled by default (v0).
#[cw_serde]
pub struct InstantiateMsg {
    /// Hard cap on total supply (uregen). E.g., 221_000_000_000_000 for 221M REGEN.
    pub hard_cap: Uint128,
    /// Initial circulating supply (uregen). Must be <= hard_cap.
    pub initial_supply: Uint128,
    /// Base regrowth rate per period. Must be in [0, 0.10].
    pub base_regrowth_rate: Decimal,
    /// Whether to enable the ecological multiplier (v0: false).
    pub ecological_multiplier_enabled: bool,
    /// Reference value for ecological multiplier (ppm). Default: 50.
    pub ecological_reference_value: Decimal,
    /// Initial M014 phase. Defaults to Inactive.
    pub m014_phase: M014Phase,
    /// Threshold for equilibrium detection (uregen).
    pub equilibrium_threshold: Uint128,
    /// Consecutive near-balance periods required for equilibrium. Default: 12.
    pub equilibrium_periods_required: u64,
}

/// Execute messages for the dynamic supply contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Execute a mint/burn period.
    ///
    /// Computes M[t] = r * (cap - supply), applies the provided burn amount,
    /// updates supply state, and records the period in history.
    ExecutePeriod {
        /// Tokens burned this period (from M013 fee routing aggregate)
        burn_amount: Uint128,
        /// Current staked amount (for staking multiplier)
        staked_amount: Uint128,
        /// Current stability commitment amount (for M014/M015 stability multiplier)
        stability_committed: Uint128,
        /// Ecological metric (delta_co2 ppm). Ignored when ecological multiplier disabled.
        delta_co2: Option<Decimal>,
    },

    /// Update the base regrowth rate (admin only).
    /// Must be in [0, 0.10] (Layer 3 governance).
    UpdateRegrowthRate {
        rate: Decimal,
    },

    /// Update the M014 phase (admin only).
    /// Determines which multiplier is used for regrowth calculation.
    UpdateM014Phase {
        phase: M014Phase,
    },

    /// Toggle the ecological multiplier (admin only, Layer 3).
    SetEcologicalMultiplier {
        enabled: bool,
        /// Optional: update the reference value when enabling
        reference_value: Option<Decimal>,
    },

    /// Update the equilibrium detection parameters (admin only).
    UpdateEquilibriumParams {
        threshold: Option<Uint128>,
        periods_required: Option<u64>,
    },
}

/// Query messages for the dynamic supply contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns current supply state (supply, total minted/burned, phase, period count).
    #[returns(SupplyStateResponse)]
    SupplyState {},

    /// Returns current supply parameters (cap, rates, multiplier config).
    #[returns(SupplyParamsResponse)]
    SupplyParams {},

    /// Returns the mint/burn record for a specific period.
    #[returns(MintBurnRecordResponse)]
    PeriodHistory { period_id: u64 },

    /// Simulate a period without executing (dry run).
    #[returns(SimulatePeriodResponse)]
    SimulatePeriod {
        burn_amount: Uint128,
        staked_amount: Uint128,
        stability_committed: Uint128,
        delta_co2: Option<Decimal>,
    },
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response for SupplyState query.
#[cw_serde]
pub struct SupplyStateResponse {
    pub current_supply: Uint128,
    pub hard_cap: Uint128,
    pub total_minted: Uint128,
    pub total_burned: Uint128,
    pub period_count: u64,
    pub phase: SupplyPhase,
    pub cap_headroom: Uint128,
    pub consecutive_equilibrium_periods: u64,
}

/// Response for SupplyParams query.
#[cw_serde]
pub struct SupplyParamsResponse {
    pub admin: String,
    pub hard_cap: Uint128,
    pub base_regrowth_rate: Decimal,
    pub ecological_multiplier_enabled: bool,
    pub ecological_reference_value: Decimal,
    pub m014_phase: M014Phase,
    pub equilibrium_threshold: Uint128,
    pub equilibrium_periods_required: u64,
}

/// Response for PeriodHistory query.
#[cw_serde]
pub struct MintBurnRecordResponse {
    pub record: MintBurnRecord,
}

/// Response for SimulatePeriod query.
#[cw_serde]
pub struct SimulatePeriodResponse {
    pub mint_amount: Uint128,
    pub burn_amount: Uint128,
    pub supply_before: Uint128,
    pub supply_after: Uint128,
    pub regrowth_rate: Decimal,
    pub effective_multiplier: Decimal,
    pub ecological_multiplier: Decimal,
    pub would_reach_equilibrium: bool,
}
