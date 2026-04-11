use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128};

use crate::state::{
    ActivityScore, DistributionRecord, StabilityCommitment,
};

// ── Instantiate ────────────────────────────────────────────────────────

#[cw_serde]
pub struct InstantiateMsg {
    /// Community pool address (source of distribution funds)
    pub community_pool_addr: String,
    /// Accepted denomination (default "uregen")
    pub denom: String,
    // All other config fields use defaults; override via UpdateConfig
}

// ── Execute ────────────────────────────────────────────────────────────

#[cw_serde]
pub enum ExecuteMsg {
    /// Admin: initialize the mechanism, begin tracking (status -> Tracking)
    InitializeMechanism {},

    /// Admin: activate distribution (status Tracking -> Distributing)
    ActivateDistribution {},

    /// Lock tokens for stability tier (must attach funds).
    /// lock_months must be within min_lock_months..=max_lock_months.
    CommitStability { lock_months: u64 },

    /// Exit stability commitment early — forfeit penalty on accrued rewards
    ExitStabilityEarly { commitment_id: u64 },

    /// Claim matured stability commitment — full tokens + accrued rewards
    ClaimMaturedStability { commitment_id: u64 },

    /// Admin/module: record ecological/governance activity for a participant
    RecordActivity {
        participant: String,
        credit_purchase_value: Uint128,
        credit_retirement_value: Uint128,
        platform_facilitation_value: Uint128,
        governance_votes: u32,
        proposal_credits: u32,
    },

    /// Admin: trigger distribution for current period with community pool inflow
    TriggerDistribution { community_pool_inflow: Uint128 },

    /// Claim accumulated activity rewards (pull model)
    ClaimRewards {},

    /// Admin: update configuration parameters
    UpdateConfig {
        community_pool_addr: Option<String>,
        credit_purchase_weight: Option<u64>,
        credit_retirement_weight: Option<u64>,
        platform_facilitation_weight: Option<u64>,
        governance_voting_weight: Option<u64>,
        proposal_submission_weight: Option<u64>,
        stability_annual_return_bps: Option<u64>,
        max_stability_share_bps: Option<u64>,
        min_commitment_amount: Option<Uint128>,
        min_lock_months: Option<u64>,
        max_lock_months: Option<u64>,
        early_exit_penalty_bps: Option<u64>,
        period_seconds: Option<u64>,
    },
}

// ── Query ──────────────────────────────────────────────────────────────

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns contract configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Returns mechanism lifecycle state
    #[returns(MechanismStateResponse)]
    MechanismState {},

    /// Returns activity score for a participant in a given period
    #[returns(ActivityScoreResponse)]
    ActivityScore { address: String, period: u32 },

    /// Returns a stability commitment by ID
    #[returns(StabilityCommitmentResponse)]
    StabilityCommitment { commitment_id: u64 },

    /// Returns distribution record for a period
    #[returns(DistributionRecordResponse)]
    DistributionRecord { period: u32 },

    /// Returns aggregate participant rewards across a range of periods
    #[returns(ParticipantRewardsResponse)]
    ParticipantRewards {
        address: String,
        period_from: u32,
        period_to: u32,
    },

    /// Returns unclaimed activity rewards for a participant
    #[returns(ClaimableRewardsResponse)]
    ClaimableRewards { address: String },
}

// ── Query responses ────────────────────────────────────────────────────

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
    pub community_pool_addr: String,
    pub denom: String,
    pub credit_purchase_weight: u64,
    pub credit_retirement_weight: u64,
    pub platform_facilitation_weight: u64,
    pub governance_voting_weight: u64,
    pub proposal_submission_weight: u64,
    pub stability_annual_return_bps: u64,
    pub max_stability_share_bps: u64,
    pub min_commitment_amount: Uint128,
    pub min_lock_months: u64,
    pub max_lock_months: u64,
    pub early_exit_penalty_bps: u64,
    pub period_seconds: u64,
}

#[cw_serde]
pub struct MechanismStateResponse {
    pub status: String,
    pub tracking_start: Option<Timestamp>,
    pub current_period: u32,
    pub last_distribution_period: Option<u32>,
}

#[cw_serde]
pub struct ActivityScoreResponse {
    pub score: ActivityScore,
}

#[cw_serde]
pub struct StabilityCommitmentResponse {
    pub commitment: StabilityCommitment,
}

#[cw_serde]
pub struct DistributionRecordResponse {
    pub record: DistributionRecord,
}

#[cw_serde]
pub struct ParticipantRewardsResponse {
    pub address: String,
    pub period_from: u32,
    pub period_to: u32,
    /// Total weighted activity rewards earned across the range
    pub total_activity_rewards: Uint128,
    /// Number of periods with recorded activity
    pub active_periods: u32,
}

#[cw_serde]
pub struct ClaimableRewardsResponse {
    pub address: String,
    /// Accumulated unclaimed activity rewards
    pub amount: Uint128,
}
