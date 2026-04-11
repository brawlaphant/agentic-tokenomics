use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Configuration ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract administrator
    pub admin: Addr,
    /// Community pool address (source of distribution funds)
    pub community_pool_addr: Addr,
    /// Accepted denomination
    pub denom: String,

    // ── Activity weight parameters (basis points, must sum to 10_000) ──
    /// Weight for credit purchases (default 3000 = 30%)
    pub credit_purchase_weight: u64,
    /// Weight for credit retirements (default 3000 = 30%)
    pub credit_retirement_weight: u64,
    /// Weight for platform facilitation (default 2000 = 20%)
    pub platform_facilitation_weight: u64,
    /// Weight for governance voting (default 1000 = 10%)
    pub governance_voting_weight: u64,
    /// Weight for proposal submissions (default 1000 = 10%)
    pub proposal_submission_weight: u64,

    // ── Stability tier parameters ──
    /// Annual return for stability commitments in bps (default 600 = 6%)
    pub stability_annual_return_bps: u64,
    /// Maximum share of community pool inflow allocated to stability in bps (default 3000 = 30%)
    pub max_stability_share_bps: u64,
    /// Minimum commitment amount in micro-denom (default 100_000_000 = 100 REGEN)
    pub min_commitment_amount: Uint128,
    /// Minimum lock duration in months (default 6)
    pub min_lock_months: u64,
    /// Maximum lock duration in months (default 24)
    pub max_lock_months: u64,
    /// Penalty on accrued rewards for early exit in bps (default 5000 = 50%)
    pub early_exit_penalty_bps: u64,

    // ── Period parameters ──
    /// Distribution period length in seconds (default 604_800 = 7 days)
    pub period_seconds: u64,
}

// ── Mechanism lifecycle ────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum MechanismStatus {
    /// Contract instantiated but mechanism not yet initialized
    Inactive,
    /// Tracking activity, calibrating weights — no distributions yet
    Tracking,
    /// Fully active: tracking + distributing rewards
    Distributing,
}

impl std::fmt::Display for MechanismStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MechanismStatus::Inactive => write!(f, "Inactive"),
            MechanismStatus::Tracking => write!(f, "Tracking"),
            MechanismStatus::Distributing => write!(f, "Distributing"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MechanismState {
    pub status: MechanismStatus,
    /// When tracking started
    pub tracking_start: Option<Timestamp>,
    /// Current period number (increments with each distribution)
    pub current_period: u32,
    /// Last period that had a distribution executed
    pub last_distribution_period: Option<u32>,
    /// Running total of committed principal across all active stability commitments.
    /// Incremented on CommitStability, decremented on ExitStabilityEarly / ClaimMaturedStability.
    pub total_committed_principal: Uint128,
}

// ── Stability commitments ──────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum CommitmentStatus {
    /// Tokens locked, accruing rewards
    Committed,
    /// Lock period complete, ready to claim
    Matured,
    /// Exited before maturity with penalty
    EarlyExit,
}

impl std::fmt::Display for CommitmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitmentStatus::Committed => write!(f, "Committed"),
            CommitmentStatus::Matured => write!(f, "Matured"),
            CommitmentStatus::EarlyExit => write!(f, "EarlyExit"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StabilityCommitment {
    pub id: u64,
    pub holder: Addr,
    pub amount: Uint128,
    pub lock_months: u64,
    pub committed_at: Timestamp,
    pub matures_at: Timestamp,
    /// Rewards accrued so far (updated on each distribution)
    pub accrued_rewards: Uint128,
    pub status: CommitmentStatus,
}

// ── Activity tracking ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct ActivityScore {
    /// Participant address (denormalized for query convenience)
    pub address: String,
    /// Period number
    pub period: u32,
    /// Value of credits purchased (in micro-denom)
    pub credit_purchase_value: Uint128,
    /// Value of credits retired (in micro-denom)
    pub credit_retirement_value: Uint128,
    /// Value of platform facilitation (in micro-denom)
    pub platform_facilitation_value: Uint128,
    /// Number of governance votes cast
    pub governance_votes: u32,
    /// Proposal credits (u32 scaled by 100 — so 100 = 1.0, 50 = 0.5)
    pub proposal_credits: u32,
}

// ── Distribution records ───────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionRecord {
    pub period: u32,
    /// Amount received from community pool for this period
    pub community_pool_inflow: Uint128,
    /// Amount allocated to stability tier
    pub stability_allocation: Uint128,
    /// Amount allocated to activity-based rewards
    pub activity_pool: Uint128,
    /// Total weighted activity score across all participants
    pub total_score: Uint128,
    /// When this distribution was executed
    pub executed_at: Timestamp,
}

// ── Storage keys ───────────────────────────────────────────────────────

pub const CONFIG: Item<Config> = Item::new("config");
pub const MECHANISM_STATE: Item<MechanismState> = Item::new("mechanism_state");
pub const NEXT_COMMITMENT_ID: Item<u64> = Item::new("next_commitment_id");

/// Stability commitments by ID
pub const COMMITMENTS: Map<u64, StabilityCommitment> = Map::new("commitments");

/// Activity scores indexed by (period, participant address)
pub const ACTIVITY_SCORES: Map<(u32, &Addr), ActivityScore> = Map::new("activity_scores");

/// Distribution records by period
pub const DISTRIBUTIONS: Map<u32, DistributionRecord> = Map::new("distributions");

/// Voter deduplication: (period, voter_address) -> bool
pub const VOTER_DEDUP: Map<(u32, &Addr), bool> = Map::new("voter_dedup");

/// Claimable activity rewards per participant (pull model).
/// Accumulated by trigger_distribution, drained by ClaimRewards.
pub const CLAIMABLE_REWARDS: Map<&Addr, Uint128> = Map::new("claimable_rewards");
