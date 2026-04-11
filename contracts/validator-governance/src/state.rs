use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Configuration ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract administrator
    pub admin: Addr,
    /// Minimum active validators (default 15)
    pub min_validators: u32,
    /// Maximum active validators (default 21)
    pub max_validators: u32,
    /// Validator term length in seconds (default 12 months = 31_536_000)
    pub term_length_seconds: u64,
    /// Probation observation period in seconds (default 30 days = 2_592_000)
    pub probation_period_seconds: u64,
    /// Minimum uptime in basis points (default 9950 = 99.5%)
    pub min_uptime_bps: u64,
    /// Performance threshold below which probation triggers (default 7000 = 70%)
    pub performance_threshold_bps: u64,
    /// Weight of uptime in composite score (default 4000 = 40%)
    pub uptime_weight_bps: u64,
    /// Weight of governance participation in composite score (default 3000 = 30%)
    pub governance_weight_bps: u64,
    /// Weight of ecosystem contribution in composite score (default 3000 = 30%)
    pub ecosystem_weight_bps: u64,
    /// Base compensation share for equal split (default 9000 = 90%)
    pub base_compensation_share_bps: u64,
    /// Performance bonus share for pro-rata (default 1000 = 10%)
    pub performance_bonus_share_bps: u64,
    /// Minimum validators per category (default 5)
    pub min_per_category: u32,
    /// Accepted payment denomination
    pub denom: String,
}

// ── Validator Category ─────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ValidatorCategory {
    InfrastructureBuilders,
    TrustedRefiPartners,
    EcologicalDataStewards,
}

impl std::fmt::Display for ValidatorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidatorCategory::InfrastructureBuilders => write!(f, "InfrastructureBuilders"),
            ValidatorCategory::TrustedRefiPartners => write!(f, "TrustedRefiPartners"),
            ValidatorCategory::EcologicalDataStewards => write!(f, "EcologicalDataStewards"),
        }
    }
}

// ── Validator Status ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ValidatorStatus {
    Candidate,
    Approved,
    Active,
    Probation,
    Removed,
    TermExpired,
}

impl std::fmt::Display for ValidatorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidatorStatus::Candidate => write!(f, "Candidate"),
            ValidatorStatus::Approved => write!(f, "Approved"),
            ValidatorStatus::Active => write!(f, "Active"),
            ValidatorStatus::Probation => write!(f, "Probation"),
            ValidatorStatus::Removed => write!(f, "Removed"),
            ValidatorStatus::TermExpired => write!(f, "TermExpired"),
        }
    }
}

// ── Authority Validator ────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuthorityValidator {
    pub address: Addr,
    pub category: ValidatorCategory,
    pub status: ValidatorStatus,
    pub term_start: Option<Timestamp>,
    pub term_end: Option<Timestamp>,
    pub probation_start: Option<Timestamp>,
    /// Cached composite performance score (0-10000 bps)
    pub performance_score_bps: u64,
    /// Accumulated compensation due (claimable)
    pub compensation_due: Uint128,
}

// ── Performance Record ─────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PerformanceRecord {
    pub validator_address: Addr,
    pub period: u64,
    pub uptime_bps: u64,
    pub governance_participation_bps: u64,
    pub ecosystem_contribution_bps: u64,
    pub composite_score_bps: u64,
    pub recorded_at: Timestamp,
}

// ── Module State ───────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ModuleState {
    /// Total fund balance available for compensation distribution
    pub validator_fund_balance: Uint128,
    /// Count of currently Active validators
    pub total_active: u32,
    /// Last time compensation was distributed
    pub last_compensation_distribution: Option<Timestamp>,
    /// Last time performance evaluations were recorded
    pub last_performance_evaluation: Option<Timestamp>,
}

// ── Storage keys ───────────────────────────────────────────────────────

pub const CONFIG: Item<Config> = Item::new("config");
pub const MODULE_STATE: Item<ModuleState> = Item::new("module_state");
pub const VALIDATORS: Map<&Addr, AuthorityValidator> = Map::new("validators");
pub const PERFORMANCE_RECORDS: Map<(&Addr, u64), PerformanceRecord> =
    Map::new("performance_records");
pub const NEXT_PERIOD: Item<u64> = Item::new("next_period");

/// Index of currently-active validator addresses.
/// Maintained by activate / probation / removal / term-end / restore handlers
/// so that `distribute_compensation` and similar hot paths never need to scan
/// the entire (unbounded) VALIDATORS map.
pub const ACTIVE_VALIDATORS: Map<&Addr, bool> = Map::new("active_validators");
