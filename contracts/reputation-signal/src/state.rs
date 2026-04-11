use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

// ---------------------------------------------------------------------------
// Subject types (spec section 3)
// ---------------------------------------------------------------------------

#[cw_serde]
pub enum SubjectType {
    CreditClass,
    Project,
    Verifier,
    Methodology,
    Address,
}

impl std::fmt::Display for SubjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubjectType::CreditClass => write!(f, "CreditClass"),
            SubjectType::Project => write!(f, "Project"),
            SubjectType::Verifier => write!(f, "Verifier"),
            SubjectType::Methodology => write!(f, "Methodology"),
            SubjectType::Address => write!(f, "Address"),
        }
    }
}

// ---------------------------------------------------------------------------
// Signal lifecycle (spec section 6.1)
// ---------------------------------------------------------------------------

#[cw_serde]
pub enum SignalStatus {
    /// Submitted but within 24h activation delay
    Submitted,
    /// Active and contributing to reputation score
    Active,
    /// Under challenge — score contribution paused
    Challenged,
    /// Challenge unresolved past deadline — escalated to governance
    Escalated,
    /// Challenge resolved: signal found valid — score restored
    ResolvedValid,
    /// Challenge resolved: signal found invalid — permanently removed (terminal)
    ResolvedInvalid,
    /// Voluntarily withdrawn by signaler (terminal)
    Withdrawn,
    /// Admin override invalidation (terminal)
    Invalidated,
}

impl SignalStatus {
    /// Whether the signal is in a terminal state (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SignalStatus::ResolvedInvalid | SignalStatus::Withdrawn | SignalStatus::Invalidated
        )
    }

    /// Whether the signal contributes to reputation score.
    pub fn contributes_to_score(&self) -> bool {
        matches!(self, SignalStatus::Active | SignalStatus::ResolvedValid)
    }
}

// ---------------------------------------------------------------------------
// Evidence
// ---------------------------------------------------------------------------

#[cw_serde]
pub struct Evidence {
    /// KOI knowledge-graph IRIs
    pub koi_links: Vec<String>,
    /// On-chain ledger transaction references
    pub ledger_refs: Vec<String>,
    /// Optional supporting web links
    pub web_links: Vec<String>,
}

impl Evidence {
    /// At least one koi_link or ledger_ref is required (spec section 6.4).
    pub fn has_required_refs(&self) -> bool {
        !self.koi_links.is_empty() || !self.ledger_refs.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Signal
// ---------------------------------------------------------------------------

#[cw_serde]
pub struct Signal {
    pub id: u64,
    pub signaler: Addr,
    pub subject_type: SubjectType,
    pub subject_id: String,
    pub category: String,
    /// Endorsement level 1-5
    pub endorsement_level: u8,
    pub evidence: Evidence,
    pub status: SignalStatus,
    /// Block time at which the signal was submitted
    pub submitted_at: Timestamp,
    /// Block time at which the signal became active (submitted_at + activation_delay)
    pub activates_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Challenge
// ---------------------------------------------------------------------------

#[cw_serde]
pub enum ChallengeOutcome {
    /// Pending resolution
    Pending,
    /// Signal found valid — restored
    Valid,
    /// Signal found invalid — permanently removed
    Invalid,
}

#[cw_serde]
pub struct Challenge {
    pub id: u64,
    pub signal_id: u64,
    pub challenger: Addr,
    pub rationale: String,
    pub evidence: Evidence,
    pub bond_amount: Uint128,
    pub outcome: ChallengeOutcome,
    pub challenged_at: Timestamp,
    /// Deadline for admin/arbiter resolution (challenged_at + resolution_deadline)
    pub resolution_deadline: Timestamp,
    /// Resolution rationale (filled on resolve)
    pub resolution_rationale: Option<String>,
}

// ---------------------------------------------------------------------------
// Config (admin controls)
// ---------------------------------------------------------------------------

#[cw_serde]
pub struct Config {
    /// Admin address (v0: sole resolver + invalidator)
    pub admin: Addr,
    /// Activation delay in seconds (default 24h = 86400s)
    pub activation_delay_seconds: u64,
    /// Challenge window in seconds (default 180 days = 15_552_000s)
    pub challenge_window_seconds: u64,
    /// Resolution deadline in seconds (default 14 days = 1_209_600s)
    pub resolution_deadline_seconds: u64,
    /// Required bond amount for challenges (v0: 0, v1: 10% of stake)
    pub challenge_bond_denom: String,
    pub challenge_bond_amount: Uint128,
    /// Decay half-life in seconds (default 14 days = 1_209_600s)
    pub decay_half_life_seconds: u64,
    /// Minimum stake required to submit a signal (per-category override via CATEGORY_MIN_STAKE)
    pub default_min_stake: Uint128,
    /// Authorized arbiter addresses (v0: admin only; v1: DAO members)
    pub arbiters: Vec<Addr>,
}

// ---------------------------------------------------------------------------
// Storage layout
// ---------------------------------------------------------------------------

/// Contract version info (cw2)
pub const CONTRACT_NAME: &str = "crates.io:reputation-signal";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Global config
pub const CONFIG: Item<Config> = Item::new("config");

/// Auto-incrementing signal ID counter
pub const NEXT_SIGNAL_ID: Item<u64> = Item::new("next_signal_id");

/// Auto-incrementing challenge ID counter
pub const NEXT_CHALLENGE_ID: Item<u64> = Item::new("next_challenge_id");

/// Signal storage: signal_id -> Signal
pub const SIGNALS: Map<u64, Signal> = Map::new("signals");

/// Index: (subject_type_str, subject_id, category) -> Vec<signal_id>
/// We store a composite key as a string: "{subject_type}:{subject_id}:{category}"
pub const SUBJECT_SIGNALS: Map<&str, Vec<u64>> = Map::new("subject_signals");

/// Challenge storage: challenge_id -> Challenge
pub const CHALLENGES: Map<u64, Challenge> = Map::new("challenges");

/// Index: signal_id -> active challenge_id (at most one active challenge per signal)
pub const SIGNAL_CHALLENGE: Map<u64, u64> = Map::new("signal_challenge");

/// Per-category minimum stake overrides: category -> Uint128
pub const CATEGORY_MIN_STAKE: Map<&str, Uint128> = Map::new("category_min_stake");
