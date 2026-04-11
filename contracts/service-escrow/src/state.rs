use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Configuration ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract administrator (can update config)
    pub admin: Addr,
    /// Arbiter DAO address for dispute resolution
    pub arbiter_dao: Addr,
    /// Community pool address for fee collection
    pub community_pool: Addr,
    /// Provider bond ratio in basis points (default 1000 = 10%)
    pub provider_bond_ratio_bps: u64,
    /// Platform fee rate in basis points (default 100 = 1%)
    pub platform_fee_rate_bps: u64,
    /// Cancellation fee rate in basis points (default 200 = 2%)
    pub cancellation_fee_rate_bps: u64,
    /// Arbiter fee rate in basis points on disputed amount (default 500 = 5%)
    pub arbiter_fee_rate_bps: u64,
    /// Default review period in seconds
    pub review_period_seconds: u64,
    /// Maximum number of milestones per agreement
    pub max_milestones: u32,
    /// Maximum revision count per milestone
    pub max_revisions: u32,
    /// Accepted payment denomination
    pub denom: String,
}

// ── Agreement ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AgreementStatus {
    Proposed,
    Funded,
    InProgress,
    MilestoneReview,
    Completed,
    Disputed,
    Cancelled,
}

impl std::fmt::Display for AgreementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgreementStatus::Proposed => write!(f, "Proposed"),
            AgreementStatus::Funded => write!(f, "Funded"),
            AgreementStatus::InProgress => write!(f, "InProgress"),
            AgreementStatus::MilestoneReview => write!(f, "MilestoneReview"),
            AgreementStatus::Completed => write!(f, "Completed"),
            AgreementStatus::Disputed => write!(f, "Disputed"),
            AgreementStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ServiceAgreement {
    pub id: u64,
    pub client: Addr,
    pub provider: Addr,
    pub service_type: String,
    pub description: String,
    pub escrow_amount: Uint128,
    pub provider_bond: Uint128,
    pub milestones: Vec<Milestone>,
    pub current_milestone: u32,
    pub status: AgreementStatus,
    pub created_at: Timestamp,
    pub funded_at: Option<Timestamp>,
    pub started_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    /// True once the provider has accepted
    pub provider_accepted: bool,
    /// True once the client has funded
    pub client_funded: bool,
    /// Total amount already released to provider
    pub total_released: Uint128,
    /// Total platform fees collected
    pub total_fees: Uint128,
}

// ── Milestone ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum MilestoneStatus {
    Pending,
    InProgress,
    Submitted,
    Approved,
    Disputed,
    Revised,
}

impl std::fmt::Display for MilestoneStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MilestoneStatus::Pending => write!(f, "Pending"),
            MilestoneStatus::InProgress => write!(f, "InProgress"),
            MilestoneStatus::Submitted => write!(f, "Submitted"),
            MilestoneStatus::Approved => write!(f, "Approved"),
            MilestoneStatus::Disputed => write!(f, "Disputed"),
            MilestoneStatus::Revised => write!(f, "Revised"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Milestone {
    pub index: u32,
    pub description: String,
    pub payment: Uint128,
    pub status: MilestoneStatus,
    pub deliverable_iri: Option<String>,
    pub submitted_at: Option<Timestamp>,
    pub approved_at: Option<Timestamp>,
    pub revision_count: u32,
}

// ── Dispute ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Dispute {
    pub agreement_id: u64,
    pub milestone_index: u32,
    pub reason: String,
    pub raised_by: Addr,
    pub raised_at: Timestamp,
    pub resolved_at: Option<Timestamp>,
    pub resolution: Option<DisputeResolution>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum DisputeResolution {
    ClientWins,
    ProviderWins,
    Split { client_percent: u32 },
}

// ── Storage keys ───────────────────────────────────────────────────────

pub const CONFIG: Item<Config> = Item::new("config");
pub const NEXT_AGREEMENT_ID: Item<u64> = Item::new("next_agreement_id");
pub const AGREEMENTS: Map<u64, ServiceAgreement> = Map::new("agreements");
pub const DISPUTES: Map<u64, Dispute> = Map::new("disputes");
