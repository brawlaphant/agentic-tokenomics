use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::{Challenge, ChallengeResolution, Collection, CollectionStatus, QualityScore};

// ── Instantiate ────────────────────────────────────────────────────────

#[cw_serde]
pub struct InstantiateMsg {
    /// Community pool address for fee/slash collection
    pub community_pool: String,
    /// Minimum bond to create a collection (default 1_000_000_000 uregen = 1000 REGEN)
    pub min_curation_bond: Option<Uint128>,
    /// Curation fee rate in basis points (default 50 = 0.5%)
    pub curation_fee_rate_bps: Option<u64>,
    /// Challenge deposit amount (default 100_000_000 uregen = 100 REGEN)
    pub challenge_deposit: Option<Uint128>,
    /// Slash percentage on lost challenge in bps (default 2000 = 20%)
    pub slash_percentage_bps: Option<u64>,
    /// Seconds before a proposed collection can activate (default 172800 = 48h)
    pub activation_delay_seconds: Option<u64>,
    /// Unbonding period in seconds (default 1209600 = 14 days)
    pub unbonding_period_seconds: Option<u64>,
    /// Window to top up bond after suspension in seconds (default 604800 = 7 days)
    pub bond_top_up_window_seconds: Option<u64>,
    /// Minimum quality score for batch inclusion (default 300)
    pub min_quality_score: Option<u32>,
    /// Max collections per curator (default 5)
    pub max_collections_per_curator: Option<u32>,
    /// Accepted payment denomination
    pub denom: String,
}

// ── Execute ────────────────────────────────────────────────────────────

#[cw_serde]
pub enum ExecuteMsg {
    /// Create a new curated collection (must attach bond >= min_curation_bond)
    CreateCollection {
        name: String,
        criteria: String,
    },

    /// Activate a Proposed collection after activation delay has elapsed
    ActivateCollection {
        collection_id: u64,
    },

    /// Add a batch to an Active collection (curator only, batch must meet quality threshold)
    AddToCollection {
        collection_id: u64,
        batch_denom: String,
    },

    /// Remove a batch from a collection (curator only)
    RemoveFromCollection {
        collection_id: u64,
        batch_denom: String,
    },

    /// Close a collection and begin unbonding (no pending challenges allowed)
    CloseCollection {
        collection_id: u64,
    },

    /// Top up bond on a Suspended collection (attach funds); restores to Active if bond >= min
    TopUpBond {
        collection_id: u64,
    },

    /// Challenge a batch's inclusion in a collection (must attach challenge deposit)
    ChallengeBatchInclusion {
        collection_id: u64,
        batch_denom: String,
        evidence: String,
    },

    /// Admin resolves a challenge
    ResolveChallenge {
        challenge_id: u64,
        resolution: ChallengeResolution,
    },

    /// Submit a quality score for a batch (admin/agent only)
    SubmitQualityScore {
        batch_denom: String,
        score: u32,
        confidence: u32,
    },

    /// Withdraw bond after collection is Closed and unbonding period has elapsed
    WithdrawBond {
        collection_id: u64,
    },

    /// Admin updates governance parameters
    UpdateConfig {
        community_pool: Option<String>,
        min_curation_bond: Option<Uint128>,
        curation_fee_rate_bps: Option<u64>,
        challenge_deposit: Option<Uint128>,
        slash_percentage_bps: Option<u64>,
        activation_delay_seconds: Option<u64>,
        unbonding_period_seconds: Option<u64>,
        bond_top_up_window_seconds: Option<u64>,
        min_quality_score: Option<u32>,
        max_collections_per_curator: Option<u32>,
    },
}

// ── Query ──────────────────────────────────────────────────────────────

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the contract configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Returns a single collection by ID
    #[returns(CollectionResponse)]
    Collection { collection_id: u64 },

    /// Returns collections filtered by status and/or curator (paginated)
    #[returns(CollectionsResponse)]
    Collections {
        status: Option<CollectionStatus>,
        curator: Option<String>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Returns the quality score for a batch denom
    #[returns(QualityScoreResponse)]
    QualityScore { batch_denom: String },

    /// Returns a single challenge by ID
    #[returns(ChallengeResponse)]
    Challenge { challenge_id: u64 },

    /// Returns stats for a curator
    #[returns(CuratorStatsResponse)]
    CuratorStats { curator: String },

    /// Returns bond status for a collection (amount, min required, is_sufficient)
    #[returns(BondStatusResponse)]
    BondStatus { collection_id: u64 },
}

// ── Query responses ────────────────────────────────────────────────────

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
    pub community_pool: String,
    pub min_curation_bond: Uint128,
    pub curation_fee_rate_bps: u64,
    pub challenge_deposit: Uint128,
    pub slash_percentage_bps: u64,
    pub activation_delay_seconds: u64,
    pub unbonding_period_seconds: u64,
    pub bond_top_up_window_seconds: u64,
    pub min_quality_score: u32,
    pub max_collections_per_curator: u32,
    pub denom: String,
}

#[cw_serde]
pub struct CollectionResponse {
    pub collection: Collection,
}

#[cw_serde]
pub struct CollectionsResponse {
    pub collections: Vec<Collection>,
}

#[cw_serde]
pub struct QualityScoreResponse {
    pub quality_score: Option<QualityScore>,
}

#[cw_serde]
pub struct ChallengeResponse {
    pub challenge: Challenge,
}

#[cw_serde]
pub struct CuratorStatsResponse {
    pub curator: String,
    pub collection_count: u32,
    pub max_collections: u32,
}

#[cw_serde]
pub struct BondStatusResponse {
    pub collection_id: u64,
    pub bond_amount: Uint128,
    pub min_required: Uint128,
    pub is_sufficient: bool,
    pub denom: String,
}
