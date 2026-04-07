use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::{
    Attestation, AttestationStatus, AttestationType, Challenge, ChallengeResolution,
};

// ── Instantiate ───────────────────────────────────────────────────────

#[cw_serde]
pub struct InstantiateMsg {
    pub arbiter_dao: String,
    pub community_pool: String,
    pub denom: String,
    pub challenge_deposit_ratio_bps: Option<u64>,
    pub arbiter_fee_ratio_bps: Option<u64>,
    pub activation_delay_seconds: Option<u64>,
}

// ── Execute ───────────────────────────────────────────────────────────

#[cw_serde]
pub enum ExecuteMsg {
    /// Submit a new attestation backed by a REGEN bond
    CreateAttestation {
        attestation_type: AttestationType,
        iri: String,
        beneficiary: Option<String>,
    },

    /// Activate a bonded attestation after the activation delay
    ActivateAttestation { attestation_id: u64 },

    /// Challenge a bonded or active attestation
    ChallengeAttestation {
        attestation_id: u64,
        evidence_iri: String,
    },

    /// Arbiter DAO resolves a challenge
    ResolveChallenge {
        attestation_id: u64,
        resolution: ChallengeResolution,
    },

    /// Attester releases their bond after lock period expires
    ReleaseBond { attestation_id: u64 },

    /// Admin updates governance parameters
    UpdateConfig {
        arbiter_dao: Option<String>,
        community_pool: Option<String>,
        challenge_deposit_ratio_bps: Option<u64>,
        arbiter_fee_ratio_bps: Option<u64>,
        activation_delay_seconds: Option<u64>,
    },
}

// ── Query ─────────────────────────────────────────────────────────────

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},

    #[returns(AttestationResponse)]
    Attestation { attestation_id: u64 },

    #[returns(AttestationsResponse)]
    Attestations {
        status: Option<AttestationStatus>,
        attester: Option<String>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(ChallengeResponse)]
    Challenge { challenge_id: u64 },

    #[returns(ChallengesResponse)]
    Challenges {
        attestation_id: Option<u64>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(BondPoolResponse)]
    BondPool {},
}

// ── Responses ─────────────────────────────────────────────────────────

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
    pub arbiter_dao: String,
    pub community_pool: String,
    pub challenge_deposit_ratio_bps: u64,
    pub arbiter_fee_ratio_bps: u64,
    pub activation_delay_seconds: u64,
    pub denom: String,
}

#[cw_serde]
pub struct AttestationResponse {
    pub attestation: Attestation,
    pub active_challenge: Option<Challenge>,
}

#[cw_serde]
pub struct AttestationsResponse {
    pub attestations: Vec<Attestation>,
}

#[cw_serde]
pub struct ChallengeResponse {
    pub challenge: Challenge,
}

#[cw_serde]
pub struct ChallengesResponse {
    pub challenges: Vec<Challenge>,
}

#[cw_serde]
pub struct BondPoolResponse {
    pub total_bonded: Uint128,
    pub total_challenge_deposits: Uint128,
    pub total_disbursed: Uint128,
}
