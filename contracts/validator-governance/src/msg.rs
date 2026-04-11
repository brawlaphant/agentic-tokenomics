use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::state::{
    AuthorityValidator, Config, ModuleState, PerformanceRecord, ValidatorCategory, ValidatorStatus,
};

// ── Instantiate ────────────────────────────────────────────────────────

#[cw_serde]
pub struct InstantiateMsg {
    /// Minimum active validators (default 15)
    pub min_validators: Option<u32>,
    /// Maximum active validators (default 21)
    pub max_validators: Option<u32>,
    /// Validator term length in seconds (default 12 months)
    pub term_length_seconds: Option<u64>,
    /// Probation observation period in seconds (default 30 days)
    pub probation_period_seconds: Option<u64>,
    /// Minimum uptime in basis points (default 9950 = 99.5%)
    pub min_uptime_bps: Option<u64>,
    /// Performance threshold for probation (default 7000 = 70%)
    pub performance_threshold_bps: Option<u64>,
    /// Uptime weight in composite score (default 4000 = 40%)
    pub uptime_weight_bps: Option<u64>,
    /// Governance participation weight (default 3000 = 30%)
    pub governance_weight_bps: Option<u64>,
    /// Ecosystem contribution weight (default 3000 = 30%)
    pub ecosystem_weight_bps: Option<u64>,
    /// Base compensation share (default 9000 = 90%)
    pub base_compensation_share_bps: Option<u64>,
    /// Performance bonus share (default 1000 = 10%)
    pub performance_bonus_share_bps: Option<u64>,
    /// Minimum validators per category (default 5)
    pub min_per_category: Option<u32>,
    /// Accepted payment denomination
    pub denom: String,
}

// ── Execute ────────────────────────────────────────────────────────────

#[cw_serde]
pub enum ExecuteMsg {
    /// Anyone applies to become a validator in a category
    ApplyForValidator {
        category: ValidatorCategory,
        application_data: String,
    },

    /// Admin approves a candidate (checks composition validity)
    ApproveValidator { applicant: String },

    /// Admin activates an approved validator (sets term)
    ActivateValidator { validator: String },

    /// Admin/agent submits a performance report for a validator
    SubmitPerformanceReport {
        validator: String,
        uptime_bps: u64,
        governance_participation_bps: u64,
        ecosystem_contribution_bps: u64,
    },

    /// Admin initiates probation for underperforming validator
    InitiateProbation { validator: String, reason: String },

    /// Admin restores a validator from probation if performance recovered
    RestoreFromProbation { validator: String },

    /// Admin confirms removal of a probationary validator
    ConfirmRemoval { validator: String },

    /// Admin ends a validator's term when it expires
    EndValidatorTerm { validator: String },

    /// Distribute compensation from fund to all active validators
    DistributeCompensation {},

    /// Validator claims accumulated compensation
    ClaimCompensation {},

    /// Admin deposits into validator fund (must attach funds)
    UpdateValidatorFund {},

    /// Admin updates governance parameters
    UpdateConfig {
        min_validators: Option<u32>,
        max_validators: Option<u32>,
        term_length_seconds: Option<u64>,
        probation_period_seconds: Option<u64>,
        min_uptime_bps: Option<u64>,
        performance_threshold_bps: Option<u64>,
        uptime_weight_bps: Option<u64>,
        governance_weight_bps: Option<u64>,
        ecosystem_weight_bps: Option<u64>,
        base_compensation_share_bps: Option<u64>,
        performance_bonus_share_bps: Option<u64>,
        min_per_category: Option<u32>,
    },
}

// ── Query ──────────────────────────────────────────────────────────────

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the contract configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Returns a single validator by address
    #[returns(ValidatorResponse)]
    Validator { address: String },

    /// Returns all active validators
    #[returns(ValidatorsResponse)]
    ActiveValidators {},

    /// Returns validators filtered by category
    #[returns(ValidatorsResponse)]
    ValidatorsByCategory { category: ValidatorCategory },

    /// Returns validators filtered by status
    #[returns(ValidatorsResponse)]
    ValidatorsByStatus { status: ValidatorStatus },

    /// Returns a performance record for a validator at a given period
    #[returns(PerformanceRecordResponse)]
    PerformanceRecord { validator: String, period: u64 },

    /// Returns category composition breakdown (counts per category)
    #[returns(CompositionBreakdownResponse)]
    CompositionBreakdown {},

    /// Returns module state
    #[returns(ModuleStateResponse)]
    ModuleState {},
}

// ── Query Responses ────────────────────────────────────────────────────

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct ValidatorResponse {
    pub validator: AuthorityValidator,
}

#[cw_serde]
pub struct ValidatorsResponse {
    pub validators: Vec<AuthorityValidator>,
}

#[cw_serde]
pub struct PerformanceRecordResponse {
    pub record: PerformanceRecord,
}

#[cw_serde]
pub struct CompositionBreakdownResponse {
    pub infrastructure_builders: u32,
    pub trusted_refi_partners: u32,
    pub ecological_data_stewards: u32,
    pub total_active: u32,
}

#[cw_serde]
pub struct ModuleStateResponse {
    pub state: ModuleState,
}
