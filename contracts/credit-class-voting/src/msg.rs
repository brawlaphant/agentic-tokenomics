use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::{AgentRecommendation, Proposal, ProposalStatus};

// ── Instantiate ────────────────────────────────────────────────────────

#[cw_serde]
pub struct InstantiateMsg {
    /// Registry Agent (AGENT-001) address
    pub registry_agent: String,
    /// Deposit amount in uregen (default 1_000_000_000 = 1000 REGEN)
    pub deposit_amount: Option<Uint128>,
    /// Accepted denomination (default "uregen")
    pub denom: Option<String>,
    /// Voting period in seconds (default 604_800 = 7 days)
    pub voting_period_seconds: Option<u64>,
    /// Agent review timeout in seconds (default 86_400 = 24h)
    pub agent_review_timeout_seconds: Option<u64>,
    /// Override window in seconds (default 21_600 = 6h)
    pub override_window_seconds: Option<u64>,
    /// Address that receives slashed deposit funds (defaults to admin)
    pub community_pool: Option<String>,
}

// ── Execute ────────────────────────────────────────────────────────────

#[cw_serde]
pub enum ExecuteMsg {
    /// Submit a new credit class proposal (must attach deposit)
    SubmitProposal {
        admin_address: String,
        credit_type: String,
        methodology_iri: String,
    },

    /// Agent submits score and recommendation (registry_agent only)
    SubmitAgentScore {
        proposal_id: u64,
        score: u32,
        confidence: u32,
        recommendation: AgentRecommendation,
    },

    /// Admin overrides an agent auto-reject within the override window
    OverrideAgentReject {
        proposal_id: u64,
    },

    /// Cast a vote on a proposal in Voting status
    CastVote {
        proposal_id: u64,
        vote_yes: bool,
    },

    /// Finalize a proposal after the voting period ends
    FinalizeProposal {
        proposal_id: u64,
    },

    /// Admin updates contract configuration
    UpdateConfig {
        registry_agent: Option<String>,
        deposit_amount: Option<Uint128>,
        voting_period_seconds: Option<u64>,
        agent_review_timeout_seconds: Option<u64>,
        override_window_seconds: Option<u64>,
        community_pool: Option<String>,
    },
}

// ── Query ──────────────────────────────────────────────────────────────

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the contract configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Returns a single proposal by ID
    #[returns(ProposalResponse)]
    Proposal { proposal_id: u64 },

    /// Returns proposals filtered by status (paginated)
    #[returns(ProposalsResponse)]
    Proposals {
        status: Option<ProposalStatus>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Returns proposals for a specific admin address
    #[returns(ProposalsResponse)]
    ProposalsByAdmin {
        admin_address: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

// ── Query responses ────────────────────────────────────────────────────

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
    pub registry_agent: String,
    pub deposit_amount: Uint128,
    pub denom: String,
    pub voting_period_seconds: u64,
    pub agent_review_timeout_seconds: u64,
    pub override_window_seconds: u64,
    pub community_pool: Option<String>,
}

#[cw_serde]
pub struct ProposalResponse {
    pub proposal: Proposal,
}

#[cw_serde]
pub struct ProposalsResponse {
    pub proposals: Vec<Proposal>,
}
