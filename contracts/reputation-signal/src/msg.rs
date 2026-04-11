use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::{Challenge, Config, Evidence, Signal, SubjectType};

// ---------------------------------------------------------------------------
// Instantiate
// ---------------------------------------------------------------------------

#[cw_serde]
pub struct InstantiateMsg {
    /// Admin address — sole resolver and invalidator in v0
    pub admin: String,
    /// Activation delay in seconds (default: 86400 = 24h)
    pub activation_delay_seconds: Option<u64>,
    /// Challenge window in seconds (default: 15_552_000 = 180 days)
    pub challenge_window_seconds: Option<u64>,
    /// Resolution deadline in seconds (default: 1_209_600 = 14 days)
    pub resolution_deadline_seconds: Option<u64>,
    /// Bond denom for challenges (default: "uregen")
    pub challenge_bond_denom: Option<String>,
    /// Bond amount for challenges (default: 0 in v0)
    pub challenge_bond_amount: Option<Uint128>,
    /// Decay half-life in seconds (default: 1_209_600 = 14 days / 336 hours)
    pub decay_half_life_seconds: Option<u64>,
    /// Default minimum stake to submit a signal
    pub default_min_stake: Option<Uint128>,
}

// ---------------------------------------------------------------------------
// Execute
// ---------------------------------------------------------------------------

#[cw_serde]
pub enum ExecuteMsg {
    /// Submit a new reputation signal. Enters SUBMITTED state with activation delay.
    SubmitSignal {
        subject_type: SubjectType,
        subject_id: String,
        category: String,
        endorsement_level: u8,
        evidence: Evidence,
    },

    /// Activate a signal whose activation delay has passed.
    /// Can be called by anyone (permissionless crank).
    ActivateSignal { signal_id: u64 },

    /// Withdraw a signal. Only the original signaler can withdraw.
    /// Cannot withdraw a signal that is currently challenged.
    WithdrawSignal { signal_id: u64 },

    /// Submit a challenge against a signal.
    /// Challenger must not be the signaler, must provide evidence, and
    /// signal must be within the challenge window.
    SubmitChallenge {
        signal_id: u64,
        rationale: String,
        evidence: Evidence,
    },

    /// Resolve a pending challenge (admin or arbiter).
    ResolveChallenge {
        challenge_id: u64,
        outcome_valid: bool,
        rationale: String,
    },

    /// Escalate a challenge whose resolution deadline has passed.
    /// Can be called by anyone (permissionless crank).
    EscalateChallenge { challenge_id: u64 },

    /// Admin-only: invalidate a signal with required rationale.
    InvalidateSignal {
        signal_id: u64,
        rationale: String,
    },

    /// Admin-only: update config parameters.
    UpdateConfig {
        activation_delay_seconds: Option<u64>,
        challenge_window_seconds: Option<u64>,
        resolution_deadline_seconds: Option<u64>,
        challenge_bond_denom: Option<String>,
        challenge_bond_amount: Option<Uint128>,
        decay_half_life_seconds: Option<u64>,
        default_min_stake: Option<Uint128>,
    },

    /// Admin-only: set per-category minimum stake.
    SetCategoryMinStake {
        category: String,
        min_stake: Uint128,
    },

    /// Admin-only: add an arbiter to the resolver set.
    AddArbiter { address: String },

    /// Admin-only: remove an arbiter from the resolver set.
    RemoveArbiter { address: String },
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get contract config.
    #[returns(ConfigResponse)]
    Config {},

    /// Get a single signal by ID.
    #[returns(SignalResponse)]
    Signal { signal_id: u64 },

    /// List signals for a given subject (type + id + category).
    #[returns(SignalsBySubjectResponse)]
    SignalsBySubject {
        subject_type: SubjectType,
        subject_id: String,
        category: String,
    },

    /// Compute the aggregate reputation score for a subject.
    /// Uses v0 decay-weighted average (no stake weighting).
    #[returns(ReputationScoreResponse)]
    ReputationScore {
        subject_type: SubjectType,
        subject_id: String,
        category: String,
    },

    /// Get a single challenge by ID.
    #[returns(ChallengeResponse)]
    Challenge { challenge_id: u64 },

    /// List active (pending) challenges.
    #[returns(ActiveChallengesResponse)]
    ActiveChallenges {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Get the category minimum stake.
    #[returns(CategoryMinStakeResponse)]
    CategoryMinStake { category: String },
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct SignalResponse {
    pub signal: Signal,
}

#[cw_serde]
pub struct SignalsBySubjectResponse {
    pub signals: Vec<Signal>,
}

#[cw_serde]
pub struct ReputationScoreResponse {
    /// v0 score normalized to 0..1000 (integer millibels for precision)
    /// Internally computed as decay-weighted average of endorsement_level/5, scaled to 1000.
    pub score: u64,
    /// Number of signals that contributed to this score
    pub contributing_signals: u32,
    /// Total signals (including non-contributing) for this subject
    pub total_signals: u32,
}

#[cw_serde]
pub struct ChallengeResponse {
    pub challenge: Challenge,
}

#[cw_serde]
pub struct ActiveChallengesResponse {
    pub challenges: Vec<Challenge>,
}

#[cw_serde]
pub struct CategoryMinStakeResponse {
    pub category: String,
    pub min_stake: Uint128,
}
