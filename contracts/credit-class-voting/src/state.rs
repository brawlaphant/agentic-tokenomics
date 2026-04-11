use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Configuration ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract administrator
    pub admin: Addr,
    /// Registry Agent (AGENT-001) address — only this address can submit scores
    pub registry_agent: Addr,
    /// Deposit amount required to submit a proposal (default 1000 REGEN)
    pub deposit_amount: Uint128,
    /// Accepted deposit denomination
    pub denom: String,
    /// Voting period in seconds (default 7 days = 604_800)
    pub voting_period_seconds: u64,
    /// Agent review timeout in seconds (default 24h = 86_400)
    pub agent_review_timeout_seconds: u64,
    /// Override window in seconds after agent auto-reject (default 6h = 21_600)
    pub override_window_seconds: u64,
    /// Address that receives slashed deposit funds (falls back to admin if None)
    pub community_pool: Option<Addr>,
}

// ── Proposal Status ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ProposalStatus {
    Draft,
    AgentReview,
    Voting,
    Approved,
    Rejected,
    Expired,
    AutoRejected,
}

impl std::fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalStatus::Draft => write!(f, "Draft"),
            ProposalStatus::AgentReview => write!(f, "AgentReview"),
            ProposalStatus::Voting => write!(f, "Voting"),
            ProposalStatus::Approved => write!(f, "Approved"),
            ProposalStatus::Rejected => write!(f, "Rejected"),
            ProposalStatus::Expired => write!(f, "Expired"),
            ProposalStatus::AutoRejected => write!(f, "AutoRejected"),
        }
    }
}

// ── Agent Recommendation ──────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AgentRecommendation {
    Approve,
    Conditional,
    Reject,
}

impl std::fmt::Display for AgentRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRecommendation::Approve => write!(f, "Approve"),
            AgentRecommendation::Conditional => write!(f, "Conditional"),
            AgentRecommendation::Reject => write!(f, "Reject"),
        }
    }
}

// ── Proposal ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Proposal {
    pub id: u64,
    /// Address that submitted the proposal
    pub proposer: Addr,
    /// Admin address associated with the credit class
    pub admin_address: Addr,
    /// Credit type abbreviation (e.g. "C", "BIO")
    pub credit_type: String,
    /// IRI pointing to the methodology document
    pub methodology_iri: String,
    /// Current lifecycle status
    pub status: ProposalStatus,
    /// Deposit locked with this proposal
    pub deposit_amount: Uint128,
    /// Block time when proposal was created
    pub created_at: Timestamp,
    /// Agent score (0-1000), set after agent review
    pub agent_score: Option<u32>,
    /// Agent confidence (0-1000), set after agent review
    pub agent_confidence: Option<u32>,
    /// Agent recommendation, set after agent review
    pub agent_recommendation: Option<AgentRecommendation>,
    /// Block time when agent submitted score
    pub agent_scored_at: Option<Timestamp>,
    /// Block time when voting period ends
    pub voting_ends_at: Option<Timestamp>,
    /// Tally of yes votes
    pub yes_votes: u64,
    /// Tally of no votes
    pub no_votes: u64,
    /// Block time when proposal reached terminal state
    pub completed_at: Option<Timestamp>,
}

// ── Storage keys ───────────────────────────────────────────────────────

pub const CONFIG: Item<Config> = Item::new("config");
pub const NEXT_PROPOSAL_ID: Item<u64> = Item::new("next_proposal_id");
pub const PROPOSALS: Map<u64, Proposal> = Map::new("proposals");
/// Tracks votes: (proposal_id, voter_addr) -> voted_yes
pub const VOTES: Map<(u64, &Addr), bool> = Map::new("votes");
