use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::{AgreementStatus, DisputeResolution, Dispute, Milestone, ServiceAgreement};

// ── Instantiate ────────────────────────────────────────────────────────

#[cw_serde]
pub struct InstantiateMsg {
    /// Arbiter DAO address for dispute resolution
    pub arbiter_dao: String,
    /// Community pool address for fee collection
    pub community_pool: String,
    /// Provider bond ratio in basis points (default 1000 = 10%)
    pub provider_bond_ratio_bps: Option<u64>,
    /// Platform fee rate in basis points (default 100 = 1%)
    pub platform_fee_rate_bps: Option<u64>,
    /// Cancellation fee rate in basis points (default 200 = 2%)
    pub cancellation_fee_rate_bps: Option<u64>,
    /// Arbiter fee rate in basis points (default 500 = 5%)
    pub arbiter_fee_rate_bps: Option<u64>,
    /// Review period in seconds (default 14 days = 1_209_600)
    pub review_period_seconds: Option<u64>,
    /// Maximum milestones per agreement (default 20)
    pub max_milestones: Option<u32>,
    /// Maximum revisions per milestone (default 3)
    pub max_revisions: Option<u32>,
    /// Accepted payment denomination
    pub denom: String,
}

// ── Execute ────────────────────────────────────────────────────────────

#[cw_serde]
pub enum ExecuteMsg {
    /// Client proposes a new service agreement (sends no funds yet)
    ProposeAgreement {
        provider: String,
        service_type: String,
        description: String,
        milestones: Vec<MilestoneInput>,
    },

    /// Provider accepts and posts bond (must attach bond funds)
    AcceptAgreement {
        agreement_id: u64,
    },

    /// Client funds the escrow (must attach escrow amount)
    FundAgreement {
        agreement_id: u64,
    },

    /// Both parties confirm to start (or auto-starts when both accept+fund)
    StartAgreement {
        agreement_id: u64,
    },

    /// Provider submits a milestone deliverable
    SubmitMilestone {
        agreement_id: u64,
        milestone_index: u32,
        deliverable_iri: String,
    },

    /// Client approves a submitted milestone (releases payment)
    ApproveMilestone {
        agreement_id: u64,
        milestone_index: u32,
    },

    /// Provider revises a milestone deliverable (before max_revisions)
    ReviseMilestone {
        agreement_id: u64,
        milestone_index: u32,
        deliverable_iri: String,
    },

    /// Client or timeout raises a dispute on a milestone
    DisputeMilestone {
        agreement_id: u64,
        milestone_index: u32,
        reason: String,
    },

    /// Arbiter DAO resolves a dispute
    ResolveDispute {
        agreement_id: u64,
        resolution: DisputeResolution,
    },

    /// Cancel agreement (only from Proposed or Funded status)
    CancelAgreement {
        agreement_id: u64,
    },

    /// Admin updates governance parameters
    UpdateConfig {
        arbiter_dao: Option<String>,
        community_pool: Option<String>,
        provider_bond_ratio_bps: Option<u64>,
        platform_fee_rate_bps: Option<u64>,
        cancellation_fee_rate_bps: Option<u64>,
        arbiter_fee_rate_bps: Option<u64>,
        review_period_seconds: Option<u64>,
        max_milestones: Option<u32>,
        max_revisions: Option<u32>,
    },
}

#[cw_serde]
pub struct MilestoneInput {
    pub description: String,
    pub payment_amount: Uint128,
}

// ── Query ──────────────────────────────────────────────────────────────

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the contract configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Returns a single agreement by ID
    #[returns(AgreementResponse)]
    Agreement { agreement_id: u64 },

    /// Returns agreements filtered by status (paginated)
    #[returns(AgreementsResponse)]
    Agreements {
        status: Option<AgreementStatus>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Returns agreements for a specific client
    #[returns(AgreementsResponse)]
    AgreementsByClient {
        client: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Returns agreements for a specific provider
    #[returns(AgreementsResponse)]
    AgreementsByProvider {
        provider: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Returns the escrow balance for an agreement
    #[returns(EscrowBalanceResponse)]
    EscrowBalance { agreement_id: u64 },

    /// Returns milestones for an agreement
    #[returns(MilestonesResponse)]
    Milestones { agreement_id: u64 },

    /// Returns the active dispute for an agreement, if any
    #[returns(DisputeResponse)]
    Dispute { agreement_id: u64 },
}

// ── Query responses ────────────────────────────────────────────────────

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
    pub arbiter_dao: String,
    pub community_pool: String,
    pub provider_bond_ratio_bps: u64,
    pub platform_fee_rate_bps: u64,
    pub cancellation_fee_rate_bps: u64,
    pub arbiter_fee_rate_bps: u64,
    pub review_period_seconds: u64,
    pub max_milestones: u32,
    pub max_revisions: u32,
    pub denom: String,
}

#[cw_serde]
pub struct AgreementResponse {
    pub agreement: ServiceAgreement,
}

#[cw_serde]
pub struct AgreementsResponse {
    pub agreements: Vec<ServiceAgreement>,
}

#[cw_serde]
pub struct EscrowBalanceResponse {
    pub agreement_id: u64,
    pub escrow_amount: Uint128,
    pub provider_bond: Uint128,
    pub total_released: Uint128,
    pub total_fees: Uint128,
    pub remaining_escrow: Uint128,
    pub denom: String,
}

#[cw_serde]
pub struct MilestonesResponse {
    pub agreement_id: u64,
    pub milestones: Vec<Milestone>,
    pub current_milestone: u32,
}

#[cw_serde]
pub struct DisputeResponse {
    pub dispute: Option<Dispute>,
}
