use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Timestamp, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    AttestationResponse, AttestationsResponse, BondPoolResponse, ChallengeResponse,
    ChallengesResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use crate::state::{
    Attestation, AttestationStatus, AttestationType, BondPoolState, Challenge,
    ChallengeResolution, Config, ATTESTATIONS, ATTESTATION_CHALLENGES, BOND_POOL, CHALLENGES,
    CONFIG, NEXT_ATTESTATION_ID, NEXT_CHALLENGE_ID,
};

const CONTRACT_NAME: &str = "crates.io:attestation-bonding";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

const SECONDS_PER_DAY: u64 = 86_400;

// ── Instantiate ───────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let challenge_ratio = msg.challenge_deposit_ratio_bps.unwrap_or(1000);
    let arbiter_fee = msg.arbiter_fee_ratio_bps.unwrap_or(500);

    if challenge_ratio > 5000 {
        return Err(ContractError::FeeRateOutOfRange {
            value: challenge_ratio, min: 0, max: 5000,
        });
    }
    if arbiter_fee > 2000 {
        return Err(ContractError::FeeRateOutOfRange {
            value: arbiter_fee, min: 0, max: 2000,
        });
    }

    let config = Config {
        admin: info.sender.clone(),
        arbiter_dao: deps.api.addr_validate(&msg.arbiter_dao)?,
        community_pool: deps.api.addr_validate(&msg.community_pool)?,
        challenge_deposit_ratio_bps: challenge_ratio,
        arbiter_fee_ratio_bps: arbiter_fee,
        activation_delay_seconds: msg.activation_delay_seconds.unwrap_or(172_800), // 48h
        denom: msg.denom,
        // Min bonds (uregen)
        min_bond_project_boundary: Uint128::new(500_000_000),        // 500 REGEN
        min_bond_baseline_measurement: Uint128::new(1_000_000_000),  // 1000 REGEN
        min_bond_credit_issuance: Uint128::new(2_000_000_000),       // 2000 REGEN
        min_bond_methodology_validation: Uint128::new(5_000_000_000), // 5000 REGEN
        // Lock periods (seconds)
        lock_period_project_boundary: 90 * SECONDS_PER_DAY,
        lock_period_baseline_measurement: 180 * SECONDS_PER_DAY,
        lock_period_credit_issuance: 365 * SECONDS_PER_DAY,
        lock_period_methodology_validation: 730 * SECONDS_PER_DAY,
        // Challenge windows (seconds)
        challenge_window_project_boundary: 60 * SECONDS_PER_DAY,
        challenge_window_baseline_measurement: 120 * SECONDS_PER_DAY,
        challenge_window_credit_issuance: 300 * SECONDS_PER_DAY,
        challenge_window_methodology_validation: 600 * SECONDS_PER_DAY,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    NEXT_ATTESTATION_ID.save(deps.storage, &1u64)?;
    NEXT_CHALLENGE_ID.save(deps.storage, &1u64)?;
    BOND_POOL.save(deps.storage, &BondPoolState::default())?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender))
}

// ── Execute ───────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateAttestation {
            attestation_type, iri, beneficiary,
        } => execute_create(deps, env, info, attestation_type, iri, beneficiary),
        ExecuteMsg::ActivateAttestation { attestation_id } => {
            execute_activate(deps, env, attestation_id)
        }
        ExecuteMsg::ChallengeAttestation {
            attestation_id, evidence_iri,
        } => execute_challenge(deps, env, info, attestation_id, evidence_iri),
        ExecuteMsg::ResolveChallenge {
            attestation_id, resolution,
        } => execute_resolve(deps, env, info, attestation_id, resolution),
        ExecuteMsg::ReleaseBond { attestation_id } => {
            execute_release(deps, env, info, attestation_id)
        }
        ExecuteMsg::UpdateConfig {
            arbiter_dao, community_pool, challenge_deposit_ratio_bps,
            arbiter_fee_ratio_bps, activation_delay_seconds,
        } => execute_update_config(
            deps, info, arbiter_dao, community_pool, challenge_deposit_ratio_bps,
            arbiter_fee_ratio_bps, activation_delay_seconds,
        ),
    }
}

fn execute_create(
    deps: DepsMut, env: Env, info: MessageInfo,
    attestation_type: AttestationType, iri: String, beneficiary: Option<String>,
) -> Result<Response, ContractError> {
    if iri.is_empty() {
        return Err(ContractError::EmptyIri);
    }

    let config = CONFIG.load(deps.storage)?;
    let min_bond = config.min_bond_for(&attestation_type);
    let bond_amount = must_pay(&info, &config.denom)?;

    if bond_amount < min_bond {
        return Err(ContractError::BondBelowMinimum {
            sent: bond_amount.to_string(),
            required: min_bond.to_string(),
            attestation_type: attestation_type.to_string(),
        });
    }

    let beneficiary_addr = beneficiary
        .map(|b| deps.api.addr_validate(&b))
        .transpose()?;

    let lock_period = config.lock_period_for(&attestation_type);
    let challenge_window = config.challenge_window_for(&attestation_type);
    let now = env.block.time;

    let id = NEXT_ATTESTATION_ID.load(deps.storage)?;
    let attestation = Attestation {
        id,
        attester: info.sender.clone(),
        attestation_type: attestation_type.clone(),
        status: AttestationStatus::Bonded,
        iri: iri.clone(),
        bond_amount,
        bonded_at: now,
        activates_at: Timestamp::from_seconds(now.seconds() + config.activation_delay_seconds),
        lock_expires_at: Timestamp::from_seconds(now.seconds() + lock_period),
        challenge_window_closes_at: Timestamp::from_seconds(now.seconds() + challenge_window),
        beneficiary: beneficiary_addr,
    };

    ATTESTATIONS.save(deps.storage, id, &attestation)?;
    NEXT_ATTESTATION_ID.save(deps.storage, &(id + 1))?;

    let mut pool = BOND_POOL.load(deps.storage)?;
    pool.total_bonded += bond_amount;
    BOND_POOL.save(deps.storage, &pool)?;

    Ok(Response::new()
        .add_attribute("action", "create_attestation")
        .add_attribute("attestation_id", id.to_string())
        .add_attribute("attester", info.sender)
        .add_attribute("attestation_type", attestation_type.to_string())
        .add_attribute("bond_amount", bond_amount)
        .add_attribute("iri", iri))
}

fn execute_activate(
    deps: DepsMut, env: Env, attestation_id: u64,
) -> Result<Response, ContractError> {
    let mut attestation = load_attestation(deps.as_ref(), attestation_id)?;

    if attestation.status != AttestationStatus::Bonded {
        return Err(ContractError::InvalidStatus {
            expected: "Bonded".to_string(),
            actual: attestation.status.to_string(),
        });
    }
    if env.block.time < attestation.activates_at {
        return Err(ContractError::InvalidStatus {
            expected: "activation delay passed".to_string(),
            actual: "activation delay not yet passed".to_string(),
        });
    }

    attestation.status = AttestationStatus::Active;
    ATTESTATIONS.save(deps.storage, attestation_id, &attestation)?;

    Ok(Response::new()
        .add_attribute("action", "activate_attestation")
        .add_attribute("attestation_id", attestation_id.to_string()))
}

fn execute_challenge(
    deps: DepsMut, env: Env, info: MessageInfo,
    attestation_id: u64, evidence_iri: String,
) -> Result<Response, ContractError> {
    if evidence_iri.is_empty() {
        return Err(ContractError::EmptyIri);
    }

    let config = CONFIG.load(deps.storage)?;
    let mut attestation = load_attestation(deps.as_ref(), attestation_id)?;

    // Must be Bonded or Active
    match attestation.status {
        AttestationStatus::Bonded | AttestationStatus::Active => {}
        _ => {
            return Err(ContractError::InvalidStatus {
                expected: "Bonded or Active".to_string(),
                actual: attestation.status.to_string(),
            });
        }
    }

    // Check challenge window
    if env.block.time > attestation.challenge_window_closes_at {
        return Err(ContractError::ChallengeWindowClosed);
    }

    // Only one active challenge per attestation
    if ATTESTATION_CHALLENGES.may_load(deps.storage, attestation_id)?.is_some() {
        return Err(ContractError::ActiveChallengePending { attestation_id });
    }

    // Verify deposit
    let min_deposit = attestation
        .bond_amount
        .multiply_ratio(config.challenge_deposit_ratio_bps, 10_000u128);
    let deposit = must_pay(&info, &config.denom)?;
    if deposit < min_deposit {
        return Err(ContractError::ChallengeDepositBelowMinimum {
            sent: deposit.to_string(),
            required: min_deposit.to_string(),
        });
    }

    let challenge_id = NEXT_CHALLENGE_ID.load(deps.storage)?;
    let challenge = Challenge {
        id: challenge_id,
        attestation_id,
        challenger: info.sender.clone(),
        evidence_iri: evidence_iri.clone(),
        deposit,
        deposited_at: env.block.time,
        resolution: None,
        resolved_at: None,
    };

    attestation.status = AttestationStatus::Challenged;

    CHALLENGES.save(deps.storage, challenge_id, &challenge)?;
    ATTESTATION_CHALLENGES.save(deps.storage, attestation_id, &challenge_id)?;
    ATTESTATIONS.save(deps.storage, attestation_id, &attestation)?;
    NEXT_CHALLENGE_ID.save(deps.storage, &(challenge_id + 1))?;

    let mut pool = BOND_POOL.load(deps.storage)?;
    pool.total_challenge_deposits += deposit;
    BOND_POOL.save(deps.storage, &pool)?;

    Ok(Response::new()
        .add_attribute("action", "challenge_attestation")
        .add_attribute("attestation_id", attestation_id.to_string())
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("challenger", info.sender)
        .add_attribute("deposit", deposit)
        .add_attribute("evidence_iri", evidence_iri))
}

fn execute_resolve(
    deps: DepsMut, env: Env, info: MessageInfo,
    attestation_id: u64, resolution: ChallengeResolution,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.arbiter_dao && info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "Only arbiter DAO or admin can resolve challenges".to_string(),
        });
    }

    let mut attestation = load_attestation(deps.as_ref(), attestation_id)?;
    if attestation.status != AttestationStatus::Challenged {
        return Err(ContractError::InvalidStatus {
            expected: "Challenged".to_string(),
            actual: attestation.status.to_string(),
        });
    }

    let challenge_id = ATTESTATION_CHALLENGES.load(deps.storage, attestation_id)
        .map_err(|_| ContractError::NoActiveChallenge { attestation_id })?;
    let mut challenge = CHALLENGES.load(deps.storage, challenge_id)?;

    // Conflict of interest checks
    if info.sender == attestation.attester {
        return Err(ContractError::ResolverIsAttester);
    }
    if info.sender == challenge.challenger {
        return Err(ContractError::ResolverIsChallenger);
    }

    let arbiter_fee = attestation
        .bond_amount
        .multiply_ratio(config.arbiter_fee_ratio_bps, 10_000u128);

    let mut msgs = vec![];
    let mut pool = BOND_POOL.load(deps.storage)?;

    match resolution {
        ChallengeResolution::Valid => {
            // Attester wins: gets bond + challenge deposit - arbiter fee
            let attester_receives = attestation.bond_amount + challenge.deposit - arbiter_fee;
            if !attester_receives.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: attestation.attester.to_string(),
                    amount: vec![Coin {
                        denom: config.denom.clone(),
                        amount: attester_receives,
                    }],
                });
            }
            if !arbiter_fee.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: config.community_pool.to_string(),
                    amount: vec![Coin {
                        denom: config.denom.clone(),
                        amount: arbiter_fee,
                    }],
                });
            }
            attestation.status = AttestationStatus::ResolvedValid;
            pool.total_bonded -= attestation.bond_amount;
            pool.total_challenge_deposits -= challenge.deposit;
            pool.total_disbursed += attester_receives + arbiter_fee;
        }
        ChallengeResolution::Invalid => {
            // Challenger wins: 50% of bond + deposit - arbiter fee
            let bond_half = attestation.bond_amount.multiply_ratio(1u128, 2u128);
            let challenger_receives = bond_half + challenge.deposit - arbiter_fee;
            if !challenger_receives.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: challenge.challenger.to_string(),
                    amount: vec![Coin {
                        denom: config.denom.clone(),
                        amount: challenger_receives,
                    }],
                });
            }
            // Community pool gets other half + arbiter fee
            let community_receives = bond_half + arbiter_fee;
            if !community_receives.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: config.community_pool.to_string(),
                    amount: vec![Coin {
                        denom: config.denom.clone(),
                        amount: community_receives,
                    }],
                });
            }
            attestation.status = AttestationStatus::Slashed;
            pool.total_bonded -= attestation.bond_amount;
            pool.total_challenge_deposits -= challenge.deposit;
            pool.total_disbursed += challenger_receives + community_receives;
        }
    }

    challenge.resolution = Some(resolution);
    challenge.resolved_at = Some(env.block.time);

    ATTESTATIONS.save(deps.storage, attestation_id, &attestation)?;
    CHALLENGES.save(deps.storage, challenge_id, &challenge)?;
    ATTESTATION_CHALLENGES.remove(deps.storage, attestation_id);
    BOND_POOL.save(deps.storage, &pool)?;

    let mut resp = Response::new()
        .add_attribute("action", "resolve_challenge")
        .add_attribute("attestation_id", attestation_id.to_string())
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("arbiter_fee", arbiter_fee);
    for msg in msgs {
        resp = resp.add_message(msg);
    }
    Ok(resp)
}

fn execute_release(
    deps: DepsMut, env: Env, info: MessageInfo, attestation_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut attestation = load_attestation(deps.as_ref(), attestation_id)?;

    if info.sender != attestation.attester {
        return Err(ContractError::Unauthorized {
            reason: "Only the attester can release their bond".to_string(),
        });
    }

    match attestation.status {
        AttestationStatus::Active | AttestationStatus::ResolvedValid => {}
        _ => {
            return Err(ContractError::InvalidStatus {
                expected: "Active or ResolvedValid".to_string(),
                actual: attestation.status.to_string(),
            });
        }
    }

    if env.block.time < attestation.lock_expires_at {
        return Err(ContractError::LockPeriodNotExpired);
    }

    // Check no active challenge
    if ATTESTATION_CHALLENGES.may_load(deps.storage, attestation_id)?.is_some() {
        return Err(ContractError::ActiveChallengePending { attestation_id });
    }

    let release_amount = attestation.bond_amount;
    attestation.status = AttestationStatus::Released;

    let mut pool = BOND_POOL.load(deps.storage)?;
    pool.total_bonded -= release_amount;
    pool.total_disbursed += release_amount;

    ATTESTATIONS.save(deps.storage, attestation_id, &attestation)?;
    BOND_POOL.save(deps.storage, &pool)?;

    let msg = BankMsg::Send {
        to_address: attestation.attester.to_string(),
        amount: vec![Coin {
            denom: config.denom,
            amount: release_amount,
        }],
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "release_bond")
        .add_attribute("attestation_id", attestation_id.to_string())
        .add_attribute("amount", release_amount))
}

fn execute_update_config(
    deps: DepsMut, info: MessageInfo,
    arbiter_dao: Option<String>, community_pool: Option<String>,
    challenge_deposit_ratio_bps: Option<u64>, arbiter_fee_ratio_bps: Option<u64>,
    activation_delay_seconds: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "Only admin can update config".to_string(),
        });
    }

    if let Some(addr) = arbiter_dao {
        config.arbiter_dao = deps.api.addr_validate(&addr)?;
    }
    if let Some(addr) = community_pool {
        config.community_pool = deps.api.addr_validate(&addr)?;
    }
    if let Some(v) = challenge_deposit_ratio_bps {
        if v > 5000 {
            return Err(ContractError::FeeRateOutOfRange { value: v, min: 0, max: 5000 });
        }
        config.challenge_deposit_ratio_bps = v;
    }
    if let Some(v) = arbiter_fee_ratio_bps {
        if v > 2000 {
            return Err(ContractError::FeeRateOutOfRange { value: v, min: 0, max: 2000 });
        }
        config.arbiter_fee_ratio_bps = v;
    }
    if let Some(v) = activation_delay_seconds {
        config.activation_delay_seconds = v;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// ── Query ─────────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Attestation { attestation_id } => {
            to_json_binary(&query_attestation(deps, attestation_id)?)
        }
        QueryMsg::Attestations {
            status, attester, start_after, limit,
        } => to_json_binary(&query_attestations(deps, status, attester, start_after, limit)?),
        QueryMsg::Challenge { challenge_id } => {
            to_json_binary(&query_challenge(deps, challenge_id)?)
        }
        QueryMsg::Challenges {
            attestation_id, start_after, limit,
        } => to_json_binary(&query_challenges(deps, attestation_id, start_after, limit)?),
        QueryMsg::BondPool {} => to_json_binary(&query_bond_pool(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.to_string(),
        arbiter_dao: config.arbiter_dao.to_string(),
        community_pool: config.community_pool.to_string(),
        challenge_deposit_ratio_bps: config.challenge_deposit_ratio_bps,
        arbiter_fee_ratio_bps: config.arbiter_fee_ratio_bps,
        activation_delay_seconds: config.activation_delay_seconds,
        denom: config.denom,
    })
}

fn query_attestation(deps: Deps, attestation_id: u64) -> StdResult<AttestationResponse> {
    let attestation = ATTESTATIONS.load(deps.storage, attestation_id)?;
    let active_challenge = ATTESTATION_CHALLENGES
        .may_load(deps.storage, attestation_id)?
        .and_then(|cid| CHALLENGES.load(deps.storage, cid).ok());
    Ok(AttestationResponse {
        attestation,
        active_challenge,
    })
}

fn query_attestations(
    deps: Deps, status: Option<AttestationStatus>, attester: Option<String>,
    start_after: Option<u64>, limit: Option<u32>,
) -> StdResult<AttestationsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| cw_storage_plus::Bound::exclusive(s));
    let attester_addr = attester.map(|a| deps.api.addr_validate(&a)).transpose()?;

    let attestations: Vec<_> = ATTESTATIONS
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|item| item.ok())
        .filter(|(_, a)| {
            status.as_ref().map_or(true, |s| a.status == *s)
                && attester_addr.as_ref().map_or(true, |addr| a.attester == *addr)
        })
        .take(limit)
        .map(|(_, a)| a)
        .collect();

    Ok(AttestationsResponse { attestations })
}

fn query_challenge(deps: Deps, challenge_id: u64) -> StdResult<ChallengeResponse> {
    let challenge = CHALLENGES.load(deps.storage, challenge_id)?;
    Ok(ChallengeResponse { challenge })
}

fn query_challenges(
    deps: Deps, attestation_id: Option<u64>, start_after: Option<u64>, limit: Option<u32>,
) -> StdResult<ChallengesResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| cw_storage_plus::Bound::exclusive(s));

    let challenges: Vec<_> = CHALLENGES
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|item| item.ok())
        .filter(|(_, c)| attestation_id.map_or(true, |aid| c.attestation_id == aid))
        .take(limit)
        .map(|(_, c)| c)
        .collect();

    Ok(ChallengesResponse { challenges })
}

fn query_bond_pool(deps: Deps) -> StdResult<BondPoolResponse> {
    let pool = BOND_POOL.load(deps.storage)?;
    Ok(BondPoolResponse {
        total_bonded: pool.total_bonded,
        total_challenge_deposits: pool.total_challenge_deposits,
        total_disbursed: pool.total_disbursed,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────

fn load_attestation(deps: Deps, id: u64) -> Result<Attestation, ContractError> {
    ATTESTATIONS
        .load(deps.storage, id)
        .map_err(|_| ContractError::AttestationNotFound { id })
}

fn must_pay(info: &MessageInfo, expected_denom: &str) -> Result<Uint128, ContractError> {
    if info.funds.len() != 1 {
        return Err(ContractError::InsufficientFunds {
            required: expected_denom.to_string(),
            sent: format!("{} coins", info.funds.len()),
        });
    }
    let coin = &info.funds[0];
    if coin.denom != expected_denom {
        return Err(ContractError::WrongDenom {
            expected: expected_denom.to_string(),
            got: coin.denom.clone(),
        });
    }
    if coin.amount.is_zero() {
        return Err(ContractError::InsufficientFunds {
            required: "non-zero".to_string(),
            sent: "0".to_string(),
        });
    }
    Ok(coin.amount)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
    use cosmwasm_std::{Addr, Timestamp};

    const DENOM: &str = "uregen";

    fn addr(input: &str) -> Addr {
        MockApi::default().addr_make(input)
    }

    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {
            arbiter_dao: addr("arbiter").to_string(),
            community_pool: addr("community").to_string(),
            denom: DENOM.to_string(),
            challenge_deposit_ratio_bps: None,
            arbiter_fee_ratio_bps: None,
            activation_delay_seconds: Some(100),
        };
        let info = message_info(&addr("admin"), &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    fn create_attestation(deps: DepsMut, env: Env) -> u64 {
        let info = message_info(
            &addr("attester"),
            &[Coin::new(500_000_000u128, DENOM)],
        );
        let msg = ExecuteMsg::CreateAttestation {
            attestation_type: AttestationType::ProjectBoundary,
            iri: "regen:attestation/1".to_string(),
            beneficiary: None,
        };
        let res = execute(deps, env, info, msg).unwrap();
        res.attributes
            .iter()
            .find(|a| a.key == "attestation_id")
            .unwrap()
            .value
            .parse()
            .unwrap()
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config.denom, DENOM);
        assert_eq!(config.activation_delay_seconds, 100);
    }

    #[test]
    fn test_create_attestation() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let env = mock_env();
        let id = create_attestation(deps.as_mut(), env);
        assert_eq!(id, 1);

        let attestation = ATTESTATIONS.load(&deps.storage, 1).unwrap();
        assert_eq!(attestation.status, AttestationStatus::Bonded);
        assert_eq!(attestation.bond_amount, Uint128::new(500_000_000));
    }

    #[test]
    fn test_bond_below_minimum() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let info = message_info(
            &addr("attester"),
            &[Coin::new(100u128, DENOM)],
        );
        let msg = ExecuteMsg::CreateAttestation {
            attestation_type: AttestationType::ProjectBoundary,
            iri: "regen:attestation/1".to_string(),
            beneficiary: None,
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::BondBelowMinimum { .. }));
    }

    #[test]
    fn test_activate_attestation() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let mut env = mock_env();
        create_attestation(deps.as_mut(), env.clone());

        // Too early
        let msg = ExecuteMsg::ActivateAttestation { attestation_id: 1 };
        let err = execute(deps.as_mut(), env.clone(), message_info(&addr("anyone"), &[]), msg.clone()).unwrap_err();
        assert!(matches!(err, ContractError::InvalidStatus { .. }));

        // After delay
        env.block.time = Timestamp::from_seconds(env.block.time.seconds() + 101);
        execute(deps.as_mut(), env, message_info(&addr("anyone"), &[]), msg).unwrap();

        let attestation = ATTESTATIONS.load(&deps.storage, 1).unwrap();
        assert_eq!(attestation.status, AttestationStatus::Active);
    }

    #[test]
    fn test_challenge_and_resolve_valid() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let env = mock_env();
        create_attestation(deps.as_mut(), env.clone());

        // Challenge (10% of 500M = 50M)
        let challenger_info = message_info(
            &addr("challenger"),
            &[Coin::new(50_000_000u128, DENOM)],
        );
        let msg = ExecuteMsg::ChallengeAttestation {
            attestation_id: 1,
            evidence_iri: "regen:evidence/1".to_string(),
        };
        execute(deps.as_mut(), env.clone(), challenger_info, msg).unwrap();

        let attestation = ATTESTATIONS.load(&deps.storage, 1).unwrap();
        assert_eq!(attestation.status, AttestationStatus::Challenged);

        // Resolve as Valid (attester wins)
        let arbiter_info = message_info(&addr("arbiter"), &[]);
        let msg = ExecuteMsg::ResolveChallenge {
            attestation_id: 1,
            resolution: ChallengeResolution::Valid,
        };
        let res = execute(deps.as_mut(), env, arbiter_info, msg).unwrap();
        assert!(res.messages.len() >= 1);

        let attestation = ATTESTATIONS.load(&deps.storage, 1).unwrap();
        assert_eq!(attestation.status, AttestationStatus::ResolvedValid);
    }

    #[test]
    fn test_challenge_and_resolve_invalid() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let env = mock_env();
        create_attestation(deps.as_mut(), env.clone());

        let challenger_info = message_info(
            &addr("challenger"),
            &[Coin::new(50_000_000u128, DENOM)],
        );
        let msg = ExecuteMsg::ChallengeAttestation {
            attestation_id: 1,
            evidence_iri: "regen:evidence/1".to_string(),
        };
        execute(deps.as_mut(), env.clone(), challenger_info, msg).unwrap();

        let arbiter_info = message_info(&addr("arbiter"), &[]);
        let msg = ExecuteMsg::ResolveChallenge {
            attestation_id: 1,
            resolution: ChallengeResolution::Invalid,
        };
        let res = execute(deps.as_mut(), env, arbiter_info, msg).unwrap();
        assert!(res.messages.len() >= 2);

        let attestation = ATTESTATIONS.load(&deps.storage, 1).unwrap();
        assert_eq!(attestation.status, AttestationStatus::Slashed);
    }

    #[test]
    fn test_release_bond() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let mut env = mock_env();
        create_attestation(deps.as_mut(), env.clone());

        // Activate
        env.block.time = Timestamp::from_seconds(env.block.time.seconds() + 101);
        execute(
            deps.as_mut(), env.clone(),
            message_info(&addr("anyone"), &[]),
            ExecuteMsg::ActivateAttestation { attestation_id: 1 },
        ).unwrap();

        // Try release before lock expires
        let attester_info = message_info(&addr("attester"), &[]);
        let msg = ExecuteMsg::ReleaseBond { attestation_id: 1 };
        let err = execute(deps.as_mut(), env.clone(), attester_info.clone(), msg.clone()).unwrap_err();
        assert!(matches!(err, ContractError::LockPeriodNotExpired));

        // After lock period (90 days)
        env.block.time = Timestamp::from_seconds(env.block.time.seconds() + 90 * 86_400 + 1);
        execute(deps.as_mut(), env, attester_info, msg).unwrap();

        let attestation = ATTESTATIONS.load(&deps.storage, 1).unwrap();
        assert_eq!(attestation.status, AttestationStatus::Released);
    }
}
