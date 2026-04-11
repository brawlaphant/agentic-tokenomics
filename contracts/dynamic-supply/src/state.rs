use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

/// M014 phase state — determines which multiplier is used for regrowth.
///
/// - INACTIVE: only staking_multiplier (pre-PoA)
/// - TRANSITION: max(staking, stability) — prevents regrowth discontinuity
/// - ACTIVE: only stability_multiplier (full PoA)
/// - EQUILIBRIUM: only stability_multiplier (steady-state PoA)
#[cw_serde]
pub enum M014Phase {
    Inactive,
    Transition,
    Active,
    Equilibrium,
}

/// Supply mechanism state machine.
///
/// INFLATIONARY -> TRANSITION -> DYNAMIC -> EQUILIBRIUM (-> DYNAMIC on shock)
#[cw_serde]
pub enum SupplyPhase {
    /// Legacy inflationary PoS (before M012 activation)
    Inflationary,
    /// M012 activated, waiting for first successful burn period
    Transition,
    /// Active algorithmic mint/burn cycles
    Dynamic,
    /// Mint ~= burn for N consecutive periods (self-sustaining)
    Equilibrium,
}

/// Core supply state tracking.
///
/// All values are in uregen (1 REGEN = 1,000,000 uregen).
#[cw_serde]
pub struct SupplyState {
    /// Current circulating supply (uregen)
    pub current_supply: Uint128,
    /// Cumulative tokens minted since activation
    pub total_minted: Uint128,
    /// Cumulative tokens burned since activation
    pub total_burned: Uint128,
    /// Number of completed mint/burn periods
    pub period_count: u64,
    /// Current supply mechanism phase
    pub phase: SupplyPhase,
    /// Number of consecutive near-equilibrium periods (for DYNAMIC -> EQUILIBRIUM)
    pub consecutive_equilibrium_periods: u64,
}

/// Configurable parameters for the supply algorithm.
///
/// All rates are Decimal values. The hard cap is in uregen.
#[cw_serde]
pub struct SupplyParams {
    /// Admin address (governance module)
    pub admin: Addr,
    /// Absolute upper bound on supply (uregen). Constitutional parameter (Layer 4).
    pub hard_cap: Uint128,
    /// Base regrowth rate per period, bounded to [0, 0.10]. Layer 3.
    pub base_regrowth_rate: Decimal,
    /// Whether the ecological multiplier is enabled (v0: false)
    pub ecological_multiplier_enabled: bool,
    /// Reference value for ecological multiplier (ppm). Layer 3.
    pub ecological_reference_value: Decimal,
    /// Current M014 phase (determines multiplier selection)
    pub m014_phase: M014Phase,
    /// Threshold for equilibrium detection: abs(mint - burn) < threshold
    pub equilibrium_threshold: Uint128,
    /// Number of consecutive near-balance periods required for equilibrium
    pub equilibrium_periods_required: u64,
}

/// Record of a single mint/burn period.
///
/// Stored per period_id for history queries.
#[cw_serde]
pub struct MintBurnRecord {
    /// Period sequence number
    pub period_id: u64,
    /// Block height at which this period was executed
    pub block_height: u64,
    /// Tokens minted (regrowth) this period (uregen)
    pub minted: Uint128,
    /// Tokens burned this period (uregen)
    pub burned: Uint128,
    /// Supply before this period's adjustment
    pub supply_before: Uint128,
    /// Supply after this period's adjustment
    pub supply_after: Uint128,
    /// Regrowth rate applied (r = r_base * effective_multiplier * ecological_multiplier)
    pub regrowth_rate: Decimal,
    /// Effective multiplier used (staking or stability, phase-gated)
    pub effective_multiplier: Decimal,
    /// Ecological multiplier used (1.0 when disabled)
    pub ecological_multiplier: Decimal,
}

pub const SUPPLY_STATE: Item<SupplyState> = Item::new("supply_state");
pub const SUPPLY_PARAMS: Item<SupplyParams> = Item::new("supply_params");
/// Per-period mint/burn history, keyed by period_id.
pub const MINT_BURN_HISTORY: Map<u64, MintBurnRecord> = Map::new("mint_burn_history");
