use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Configuration ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract administrator (can resolve challenges, update config)
    pub admin: Addr,
    /// Community pool address for fee/slash collection
    pub community_pool: Addr,
    /// Minimum bond a curator must attach to create a collection (default 1000 REGEN)
    pub min_curation_bond: Uint128,
    /// Fee rate in basis points curators earn on trades (default 50 = 0.5%)
    pub curation_fee_rate_bps: u64,
    /// Deposit required to file a challenge (default 100 REGEN)
    pub challenge_deposit: Uint128,
    /// Percentage of curator bond slashed on lost challenge (default 2000 = 20%)
    pub slash_percentage_bps: u64,
    /// Seconds after creation before a collection can activate (default 172800 = 48h)
    pub activation_delay_seconds: u64,
    /// Seconds a curator must wait after closing before bond is returned (default 1209600 = 14 days)
    pub unbonding_period_seconds: u64,
    /// Window in seconds to top up bond on a suspended collection (default 604800 = 7 days)
    pub bond_top_up_window_seconds: u64,
    /// Minimum quality score for a batch to be added to a collection (default 300, scale 0-1000)
    pub min_quality_score: u32,
    /// Maximum collections a single curator can have (default 5)
    pub max_collections_per_curator: u32,
    /// Accepted payment denomination
    pub denom: String,
}

// ── Collection ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum CollectionStatus {
    Proposed,
    Active,
    UnderReview,
    Suspended,
    Closed,
}

impl std::fmt::Display for CollectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectionStatus::Proposed => write!(f, "Proposed"),
            CollectionStatus::Active => write!(f, "Active"),
            CollectionStatus::UnderReview => write!(f, "UnderReview"),
            CollectionStatus::Suspended => write!(f, "Suspended"),
            CollectionStatus::Closed => write!(f, "Closed"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Collection {
    pub id: u64,
    /// Curator who created and manages this collection
    pub curator: Addr,
    /// Human-readable collection name
    pub name: String,
    /// Curation criteria description
    pub criteria: String,
    /// Current lifecycle status
    pub status: CollectionStatus,
    /// Amount of REGEN bonded by curator
    pub bond_amount: Uint128,
    /// Credit batch denoms included in this collection
    pub batches: Vec<String>,
    /// When the collection was created
    pub created_at: Timestamp,
    /// Earliest time the collection can be activated
    pub activates_at: Timestamp,
    /// If suspended, when the suspension window expires (curator must top up by then)
    pub suspension_expires_at: Option<Timestamp>,
    /// When the collection was closed (for unbonding calculation)
    pub closed_at: Option<Timestamp>,
}

// ── Challenge ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ChallengeResolution {
    CuratorWins,
    ChallengerWins,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Challenge {
    pub id: u64,
    pub collection_id: u64,
    /// Address that filed the challenge
    pub challenger: Addr,
    /// The specific batch being challenged
    pub batch_denom: String,
    /// Deposit attached by challenger
    pub deposit: Uint128,
    /// Evidence supporting the challenge
    pub evidence: String,
    /// When the challenge was filed
    pub filed_at: Timestamp,
    /// Resolution outcome, None if still pending
    pub resolution: Option<ChallengeResolution>,
}

// ── Quality Score ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct QualityScore {
    /// The credit batch denomination
    pub batch_denom: String,
    /// Quality score (0-1000)
    pub score: u32,
    /// Confidence level (0-1000)
    pub confidence: u32,
    /// When the score was computed/submitted
    pub computed_at: Timestamp,
}

// ── Storage keys ───────────────────────────────────────────────────────

pub const CONFIG: Item<Config> = Item::new("config");
pub const NEXT_COLLECTION_ID: Item<u64> = Item::new("next_collection_id");
pub const NEXT_CHALLENGE_ID: Item<u64> = Item::new("next_challenge_id");

/// Collection ID -> Collection
pub const COLLECTIONS: Map<u64, Collection> = Map::new("collections");

/// Challenge ID -> Challenge
pub const CHALLENGES: Map<u64, Challenge> = Map::new("challenges");

/// Collection ID -> Vec of Challenge IDs
pub const COLLECTION_CHALLENGES: Map<u64, Vec<u64>> = Map::new("collection_challenges");

/// Batch denom -> QualityScore
pub const QUALITY_SCORES: Map<&str, QualityScore> = Map::new("quality_scores");

/// Reverse index: batch denom -> list of collection IDs containing that batch
pub const BATCH_COLLECTIONS: Map<&str, Vec<u64>> = Map::new("batch_collections");

/// Curator address -> number of collections they own
pub const CURATOR_COLLECTION_COUNT: Map<&Addr, u32> = Map::new("curator_collection_count");
