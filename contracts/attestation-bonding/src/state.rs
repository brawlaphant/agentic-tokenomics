use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Attestation Types ─────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AttestationType {
    ProjectBoundary,
    BaselineMeasurement,
    CreditIssuanceClaim,
    MethodologyValidation,
}

impl std::fmt::Display for AttestationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttestationType::ProjectBoundary => write!(f, "ProjectBoundary"),
            AttestationType::BaselineMeasurement => write!(f, "BaselineMeasurement"),
            AttestationType::CreditIssuanceClaim => write!(f, "CreditIssuanceClaim"),
            AttestationType::MethodologyValidation => write!(f, "MethodologyValidation"),
        }
    }
}

// ── Attestation Status ────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AttestationStatus {
    Bonded,
    Active,
    Challenged,
    ResolvedValid,
    Slashed,
    Released,
}

impl std::fmt::Display for AttestationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttestationStatus::Bonded => write!(f, "Bonded"),
            AttestationStatus::Active => write!(f, "Active"),
            AttestationStatus::Challenged => write!(f, "Challenged"),
            AttestationStatus::ResolvedValid => write!(f, "ResolvedValid"),
            AttestationStatus::Slashed => write!(f, "Slashed"),
            AttestationStatus::Released => write!(f, "Released"),
        }
    }
}

// ── Configuration ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub arbiter_dao: Addr,
    pub community_pool: Addr,
    /// Challenge deposit ratio in basis points (default 1000 = 10%)
    pub challenge_deposit_ratio_bps: u64,
    /// Arbiter fee ratio in basis points (default 500 = 5%)
    pub arbiter_fee_ratio_bps: u64,
    /// Activation delay in seconds (default 48h = 172800)
    pub activation_delay_seconds: u64,
    /// Accepted payment denomination
    pub denom: String,

    // Min bonds per type (uregen)
    pub min_bond_project_boundary: Uint128,
    pub min_bond_baseline_measurement: Uint128,
    pub min_bond_credit_issuance: Uint128,
    pub min_bond_methodology_validation: Uint128,

    // Lock periods per type (seconds)
    pub lock_period_project_boundary: u64,
    pub lock_period_baseline_measurement: u64,
    pub lock_period_credit_issuance: u64,
    pub lock_period_methodology_validation: u64,

    // Challenge windows per type (seconds)
    pub challenge_window_project_boundary: u64,
    pub challenge_window_baseline_measurement: u64,
    pub challenge_window_credit_issuance: u64,
    pub challenge_window_methodology_validation: u64,
}

impl Config {
    pub fn min_bond_for(&self, atype: &AttestationType) -> Uint128 {
        match atype {
            AttestationType::ProjectBoundary => self.min_bond_project_boundary,
            AttestationType::BaselineMeasurement => self.min_bond_baseline_measurement,
            AttestationType::CreditIssuanceClaim => self.min_bond_credit_issuance,
            AttestationType::MethodologyValidation => self.min_bond_methodology_validation,
        }
    }

    pub fn lock_period_for(&self, atype: &AttestationType) -> u64 {
        match atype {
            AttestationType::ProjectBoundary => self.lock_period_project_boundary,
            AttestationType::BaselineMeasurement => self.lock_period_baseline_measurement,
            AttestationType::CreditIssuanceClaim => self.lock_period_credit_issuance,
            AttestationType::MethodologyValidation => self.lock_period_methodology_validation,
        }
    }

    pub fn challenge_window_for(&self, atype: &AttestationType) -> u64 {
        match atype {
            AttestationType::ProjectBoundary => self.challenge_window_project_boundary,
            AttestationType::BaselineMeasurement => self.challenge_window_baseline_measurement,
            AttestationType::CreditIssuanceClaim => self.challenge_window_credit_issuance,
            AttestationType::MethodologyValidation => self.challenge_window_methodology_validation,
        }
    }
}

// ── Attestation ───────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Attestation {
    pub id: u64,
    pub attester: Addr,
    pub attestation_type: AttestationType,
    pub status: AttestationStatus,
    pub iri: String,
    pub bond_amount: Uint128,
    pub bonded_at: Timestamp,
    pub activates_at: Timestamp,
    pub lock_expires_at: Timestamp,
    pub challenge_window_closes_at: Timestamp,
    pub beneficiary: Option<Addr>,
}

// ── Challenge ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ChallengeResolution {
    Valid,
    Invalid,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Challenge {
    pub id: u64,
    pub attestation_id: u64,
    pub challenger: Addr,
    pub evidence_iri: String,
    pub deposit: Uint128,
    pub deposited_at: Timestamp,
    pub resolution: Option<ChallengeResolution>,
    pub resolved_at: Option<Timestamp>,
}

// ── Bond Pool ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct BondPoolState {
    pub total_bonded: Uint128,
    pub total_challenge_deposits: Uint128,
    pub total_disbursed: Uint128,
}

// ── Storage Keys ──────────────────────────────────────────────────────

pub const CONFIG: Item<Config> = Item::new("config");
pub const NEXT_ATTESTATION_ID: Item<u64> = Item::new("next_attestation_id");
pub const NEXT_CHALLENGE_ID: Item<u64> = Item::new("next_challenge_id");
pub const ATTESTATIONS: Map<u64, Attestation> = Map::new("attestations");
pub const CHALLENGES: Map<u64, Challenge> = Map::new("challenges");
/// Map attestation_id → challenge_id for active challenges
pub const ATTESTATION_CHALLENGES: Map<u64, u64> = Map::new("attestation_challenges");
pub const BOND_POOL: Item<BondPoolState> = Item::new("bond_pool");
