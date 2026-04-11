use cosmwasm_std::{
    entry_point, to_json_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    CompositionBreakdownResponse, ConfigResponse, ExecuteMsg, InstantiateMsg,
    ModuleStateResponse, PerformanceRecordResponse, QueryMsg, ValidatorResponse,
    ValidatorsResponse,
};
use crate::state::{
    AuthorityValidator, Config, ModuleState, PerformanceRecord, ValidatorCategory, ValidatorStatus,
    ACTIVE_VALIDATORS, CONFIG, MODULE_STATE, NEXT_PERIOD, PERFORMANCE_RECORDS, VALIDATORS,
};

const CONTRACT_NAME: &str = "crates.io:validator-governance";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Instantiate ────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let uptime_w = msg.uptime_weight_bps.unwrap_or(4000);
    let gov_w = msg.governance_weight_bps.unwrap_or(3000);
    let eco_w = msg.ecosystem_weight_bps.unwrap_or(3000);

    let weight_sum = uptime_w + gov_w + eco_w;
    if weight_sum != 10_000 {
        return Err(ContractError::InvalidWeightSum { total: weight_sum });
    }

    let base_share = msg.base_compensation_share_bps.unwrap_or(9000);
    let bonus_share = msg.performance_bonus_share_bps.unwrap_or(1000);
    let share_sum = base_share + bonus_share;
    if share_sum != 10_000 {
        return Err(ContractError::InvalidWeightSum { total: share_sum });
    }

    validate_bps(msg.min_uptime_bps.unwrap_or(9950))?;
    validate_bps(msg.performance_threshold_bps.unwrap_or(7000))?;
    validate_bps(uptime_w)?;
    validate_bps(gov_w)?;
    validate_bps(eco_w)?;
    validate_bps(base_share)?;
    validate_bps(bonus_share)?;

    let config = Config {
        admin: info.sender.clone(),
        min_validators: msg.min_validators.unwrap_or(15),
        max_validators: msg.max_validators.unwrap_or(21),
        term_length_seconds: msg.term_length_seconds.unwrap_or(31_536_000), // 12 months
        probation_period_seconds: msg.probation_period_seconds.unwrap_or(2_592_000), // 30 days
        min_uptime_bps: msg.min_uptime_bps.unwrap_or(9950),
        performance_threshold_bps: msg.performance_threshold_bps.unwrap_or(7000),
        uptime_weight_bps: uptime_w,
        governance_weight_bps: gov_w,
        ecosystem_weight_bps: eco_w,
        base_compensation_share_bps: base_share,
        performance_bonus_share_bps: bonus_share,
        min_per_category: msg.min_per_category.unwrap_or(5),
        denom: msg.denom,
    };

    let module_state = ModuleState {
        validator_fund_balance: Uint128::zero(),
        total_active: 0,
        last_compensation_distribution: None,
        last_performance_evaluation: None,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    MODULE_STATE.save(deps.storage, &module_state)?;
    NEXT_PERIOD.save(deps.storage, &1u64)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender))
}

// ── Execute ────────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ApplyForValidator {
            category,
            application_data,
        } => execute_apply(deps, env, info, category, application_data),
        ExecuteMsg::ApproveValidator { applicant } => {
            execute_approve(deps, env, info, applicant)
        }
        ExecuteMsg::ActivateValidator { validator } => {
            execute_activate(deps, env, info, validator)
        }
        ExecuteMsg::SubmitPerformanceReport {
            validator,
            uptime_bps,
            governance_participation_bps,
            ecosystem_contribution_bps,
        } => execute_submit_performance(
            deps,
            env,
            info,
            validator,
            uptime_bps,
            governance_participation_bps,
            ecosystem_contribution_bps,
        ),
        ExecuteMsg::InitiateProbation { validator, reason } => {
            execute_initiate_probation(deps, env, info, validator, reason)
        }
        ExecuteMsg::RestoreFromProbation { validator } => {
            execute_restore_from_probation(deps, env, info, validator)
        }
        ExecuteMsg::ConfirmRemoval { validator } => {
            execute_confirm_removal(deps, env, info, validator)
        }
        ExecuteMsg::EndValidatorTerm { validator } => {
            execute_end_term(deps, env, info, validator)
        }
        ExecuteMsg::DistributeCompensation {} => execute_distribute_compensation(deps, env, info),
        ExecuteMsg::ClaimCompensation {} => execute_claim_compensation(deps, env, info),
        ExecuteMsg::UpdateValidatorFund {} => execute_update_fund(deps, env, info),
        ExecuteMsg::UpdateConfig {
            min_validators,
            max_validators,
            term_length_seconds,
            probation_period_seconds,
            min_uptime_bps,
            performance_threshold_bps,
            uptime_weight_bps,
            governance_weight_bps,
            ecosystem_weight_bps,
            base_compensation_share_bps,
            performance_bonus_share_bps,
            min_per_category,
        } => execute_update_config(
            deps,
            info,
            min_validators,
            max_validators,
            term_length_seconds,
            probation_period_seconds,
            min_uptime_bps,
            performance_threshold_bps,
            uptime_weight_bps,
            governance_weight_bps,
            ecosystem_weight_bps,
            base_compensation_share_bps,
            performance_bonus_share_bps,
            min_per_category,
        ),
    }
}

// ── Execute handlers ───────────────────────────────────────────────────

fn execute_apply(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    category: ValidatorCategory,
    application_data: String,
) -> Result<Response, ContractError> {
    let applicant = info.sender.clone();

    if VALIDATORS.may_load(deps.storage, &applicant)?.is_some() {
        return Err(ContractError::ValidatorAlreadyExists {
            address: applicant.to_string(),
        });
    }

    let validator = AuthorityValidator {
        address: applicant.clone(),
        category: category.clone(),
        status: ValidatorStatus::Candidate,
        term_start: None,
        term_end: None,
        probation_start: None,
        performance_score_bps: 0,
        compensation_due: Uint128::zero(),
    };

    VALIDATORS.save(deps.storage, &applicant, &validator)?;

    Ok(Response::new()
        .add_attribute("action", "apply_for_validator")
        .add_attribute("applicant", applicant)
        .add_attribute("category", category.to_string())
        .add_attribute("application_data", application_data))
}

fn execute_approve(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    applicant: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let applicant_addr = deps.api.addr_validate(&applicant)?;
    let mut validator = load_validator(deps.as_ref(), &applicant_addr)?;

    if validator.status != ValidatorStatus::Candidate {
        return Err(ContractError::InvalidStatus {
            expected: "Candidate".to_string(),
            actual: validator.status.to_string(),
        });
    }

    validator.status = ValidatorStatus::Approved;
    VALIDATORS.save(deps.storage, &applicant_addr, &validator)?;

    Ok(Response::new()
        .add_attribute("action", "approve_validator")
        .add_attribute("applicant", applicant))
}

fn execute_activate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validator_addr: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let addr = deps.api.addr_validate(&validator_addr)?;
    let mut validator = load_validator(deps.as_ref(), &addr)?;

    if validator.status != ValidatorStatus::Approved {
        return Err(ContractError::InvalidStatus {
            expected: "Approved".to_string(),
            actual: validator.status.to_string(),
        });
    }

    // Check max validators
    let mut state = MODULE_STATE.load(deps.storage)?;
    if state.total_active >= config.max_validators {
        return Err(ContractError::AboveMaxValidators {
            max: config.max_validators,
        });
    }

    // Set term
    let term_start = env.block.time;
    let term_end = term_start.plus_seconds(config.term_length_seconds);

    validator.status = ValidatorStatus::Active;
    validator.term_start = Some(term_start);
    validator.term_end = Some(term_end);

    state.total_active += 1;

    VALIDATORS.save(deps.storage, &addr, &validator)?;
    ACTIVE_VALIDATORS.save(deps.storage, &addr, &true)?;
    MODULE_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "activate_validator")
        .add_attribute("validator", validator_addr)
        .add_attribute("term_start", term_start.to_string())
        .add_attribute("term_end", term_end.to_string()))
}

fn execute_submit_performance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validator_addr: String,
    uptime_bps: u64,
    governance_participation_bps: u64,
    ecosystem_contribution_bps: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    validate_bps(uptime_bps)?;
    validate_bps(governance_participation_bps)?;
    validate_bps(ecosystem_contribution_bps)?;

    let addr = deps.api.addr_validate(&validator_addr)?;
    let mut validator = load_validator(deps.as_ref(), &addr)?;

    if validator.status != ValidatorStatus::Active && validator.status != ValidatorStatus::Probation
    {
        return Err(ContractError::InvalidStatus {
            expected: "Active or Probation".to_string(),
            actual: validator.status.to_string(),
        });
    }

    // Calculate composite score: uptime*0.4 + gov*0.3 + ecosystem*0.3
    let composite = (uptime_bps * config.uptime_weight_bps
        + governance_participation_bps * config.governance_weight_bps
        + ecosystem_contribution_bps * config.ecosystem_weight_bps)
        / 10_000;

    let period = NEXT_PERIOD.load(deps.storage)?;

    let record = PerformanceRecord {
        validator_address: addr.clone(),
        period,
        uptime_bps,
        governance_participation_bps,
        ecosystem_contribution_bps,
        composite_score_bps: composite,
        recorded_at: env.block.time,
    };

    PERFORMANCE_RECORDS.save(deps.storage, (&addr, period), &record)?;
    NEXT_PERIOD.save(deps.storage, &(period + 1))?;

    // Update cached score
    validator.performance_score_bps = composite;
    VALIDATORS.save(deps.storage, &addr, &validator)?;

    // Update module state
    let mut state = MODULE_STATE.load(deps.storage)?;
    state.last_performance_evaluation = Some(env.block.time);
    MODULE_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "submit_performance_report")
        .add_attribute("validator", validator_addr)
        .add_attribute("composite_score", composite.to_string())
        .add_attribute("period", period.to_string()))
}

fn execute_initiate_probation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validator_addr: String,
    reason: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let addr = deps.api.addr_validate(&validator_addr)?;
    let mut validator = load_validator(deps.as_ref(), &addr)?;

    if validator.status != ValidatorStatus::Active {
        return Err(ContractError::InvalidStatus {
            expected: "Active".to_string(),
            actual: validator.status.to_string(),
        });
    }

    if validator.performance_score_bps >= config.performance_threshold_bps {
        return Err(ContractError::ScoreAboveThreshold {
            score: validator.performance_score_bps,
            threshold: config.performance_threshold_bps,
        });
    }

    validator.status = ValidatorStatus::Probation;
    validator.probation_start = Some(env.block.time);

    // Decrement active count since probation is not Active
    let mut state = MODULE_STATE.load(deps.storage)?;
    state.total_active = state.total_active.saturating_sub(1);
    MODULE_STATE.save(deps.storage, &state)?;

    VALIDATORS.save(deps.storage, &addr, &validator)?;
    ACTIVE_VALIDATORS.remove(deps.storage, &addr);

    Ok(Response::new()
        .add_attribute("action", "initiate_probation")
        .add_attribute("validator", validator_addr)
        .add_attribute("reason", reason))
}

fn execute_restore_from_probation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    validator_addr: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let addr = deps.api.addr_validate(&validator_addr)?;
    let mut validator = load_validator(deps.as_ref(), &addr)?;

    if validator.status != ValidatorStatus::Probation {
        return Err(ContractError::InvalidStatus {
            expected: "Probation".to_string(),
            actual: validator.status.to_string(),
        });
    }

    if validator.performance_score_bps < config.performance_threshold_bps {
        return Err(ContractError::ScoreBelowThreshold {
            score: validator.performance_score_bps,
            threshold: config.performance_threshold_bps,
        });
    }

    validator.status = ValidatorStatus::Active;
    validator.probation_start = None;

    // Re-increment active count
    let mut state = MODULE_STATE.load(deps.storage)?;
    state.total_active += 1;
    MODULE_STATE.save(deps.storage, &state)?;

    VALIDATORS.save(deps.storage, &addr, &validator)?;
    ACTIVE_VALIDATORS.save(deps.storage, &addr, &true)?;

    Ok(Response::new()
        .add_attribute("action", "restore_from_probation")
        .add_attribute("validator", validator_addr))
}

fn execute_confirm_removal(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    validator_addr: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let addr = deps.api.addr_validate(&validator_addr)?;
    let mut validator = load_validator(deps.as_ref(), &addr)?;

    if validator.status != ValidatorStatus::Probation {
        return Err(ContractError::InvalidStatus {
            expected: "Probation".to_string(),
            actual: validator.status.to_string(),
        });
    }

    // Check composition — count active validators in this category excluding this one
    // (This validator is already not Active, so just check the remaining active set)
    let category_active = count_active_in_category(deps.as_ref(), &validator.category)?;
    // Since the validator is Probation (not Active), category_active doesn't include them.
    // We only need to ensure the category still has enough Active validators.
    if category_active < config.min_per_category {
        return Err(ContractError::CompositionViolation {
            category: validator.category.to_string(),
            count: category_active,
            min: config.min_per_category,
        });
    }

    validator.status = ValidatorStatus::Removed;
    VALIDATORS.save(deps.storage, &addr, &validator)?;
    ACTIVE_VALIDATORS.remove(deps.storage, &addr); // defensive: already removed at probation

    Ok(Response::new()
        .add_attribute("action", "confirm_removal")
        .add_attribute("validator", validator_addr))
}

fn execute_end_term(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validator_addr: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let addr = deps.api.addr_validate(&validator_addr)?;
    let mut validator = load_validator(deps.as_ref(), &addr)?;

    if validator.status != ValidatorStatus::Active {
        return Err(ContractError::InvalidStatus {
            expected: "Active".to_string(),
            actual: validator.status.to_string(),
        });
    }

    let term_end = validator.term_end.ok_or(ContractError::TermNotEnded)?;
    if env.block.time < term_end {
        return Err(ContractError::TermNotEnded);
    }

    validator.status = ValidatorStatus::TermExpired;

    let mut state = MODULE_STATE.load(deps.storage)?;
    state.total_active = state.total_active.saturating_sub(1);
    MODULE_STATE.save(deps.storage, &state)?;

    VALIDATORS.save(deps.storage, &addr, &validator)?;
    ACTIVE_VALIDATORS.remove(deps.storage, &addr);

    Ok(Response::new()
        .add_attribute("action", "end_validator_term")
        .add_attribute("validator", validator_addr))
}

fn execute_distribute_compensation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let mut state = MODULE_STATE.load(deps.storage)?;

    if state.validator_fund_balance.is_zero() {
        return Err(ContractError::InsufficientFund {
            required: "non-zero".to_string(),
            available: "0".to_string(),
        });
    }

    // Collect active validators via the ACTIVE_VALIDATORS index (bounded)
    let active_validators: Vec<AuthorityValidator> = ACTIVE_VALIDATORS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(addr, _)| VALIDATORS.load(deps.storage, &addr))
        .filter_map(|r| r.ok())
        .collect();

    let active_count = active_validators.len() as u128;
    if active_count == 0 {
        return Err(ContractError::InsufficientFund {
            required: "at least 1 active validator".to_string(),
            available: "0 active validators".to_string(),
        });
    }

    let fund = state.validator_fund_balance;

    // 90% base — equal split
    let base_pool = fund.multiply_ratio(config.base_compensation_share_bps as u128, 10_000u128);
    let base_per_validator = base_pool.multiply_ratio(1u128, active_count);

    // 10% bonus — pro-rata by performance score
    let bonus_pool = fund.multiply_ratio(config.performance_bonus_share_bps as u128, 10_000u128);
    let total_score: u128 = active_validators
        .iter()
        .map(|v| v.performance_score_bps as u128)
        .sum();

    let mut total_distributed = Uint128::zero();

    for av in &active_validators {
        let bonus = if total_score > 0 {
            bonus_pool.multiply_ratio(av.performance_score_bps as u128, total_score)
        } else {
            // Equal split if no scores
            bonus_pool.multiply_ratio(1u128, active_count)
        };

        let total_comp = base_per_validator + bonus;
        total_distributed += total_comp;

        let mut v = VALIDATORS.load(deps.storage, &av.address)?;
        v.compensation_due += total_comp;
        VALIDATORS.save(deps.storage, &av.address, &v)?;
    }

    state.validator_fund_balance = state
        .validator_fund_balance
        .saturating_sub(total_distributed);
    state.last_compensation_distribution = Some(env.block.time);
    MODULE_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "distribute_compensation")
        .add_attribute("total_distributed", total_distributed)
        .add_attribute("active_validators", active_count.to_string()))
}

fn execute_claim_compensation(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let addr = info.sender.clone();

    let mut validator = load_validator(deps.as_ref(), &addr)?;

    if validator.compensation_due.is_zero() {
        return Err(ContractError::NoCompensationDue);
    }

    let amount = validator.compensation_due;
    validator.compensation_due = Uint128::zero();
    VALIDATORS.save(deps.storage, &addr, &validator)?;

    let msg = BankMsg::Send {
        to_address: addr.to_string(),
        amount: vec![Coin {
            denom: config.denom,
            amount,
        }],
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "claim_compensation")
        .add_attribute("validator", addr)
        .add_attribute("amount", amount))
}

fn execute_update_fund(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let sent = info
        .funds
        .iter()
        .find(|c| c.denom == config.denom)
        .map(|c| c.amount)
        .unwrap_or(Uint128::zero());

    if sent.is_zero() {
        return Err(ContractError::WrongDenom {
            expected: config.denom,
            got: "nothing or wrong denom".to_string(),
        });
    }

    let mut state = MODULE_STATE.load(deps.storage)?;
    state.validator_fund_balance += sent;
    MODULE_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "update_validator_fund")
        .add_attribute("deposited", sent)
        .add_attribute("new_balance", state.validator_fund_balance))
}

#[allow(clippy::too_many_arguments)]
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
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
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    if let Some(v) = min_validators {
        config.min_validators = v;
    }
    if let Some(v) = max_validators {
        config.max_validators = v;
    }
    if let Some(v) = term_length_seconds {
        config.term_length_seconds = v;
    }
    if let Some(v) = probation_period_seconds {
        config.probation_period_seconds = v;
    }
    if let Some(v) = min_uptime_bps {
        validate_bps(v)?;
        config.min_uptime_bps = v;
    }
    if let Some(v) = performance_threshold_bps {
        validate_bps(v)?;
        config.performance_threshold_bps = v;
    }

    // If any weight is updated, validate the sum
    let new_uptime_w = uptime_weight_bps.unwrap_or(config.uptime_weight_bps);
    let new_gov_w = governance_weight_bps.unwrap_or(config.governance_weight_bps);
    let new_eco_w = ecosystem_weight_bps.unwrap_or(config.ecosystem_weight_bps);
    if uptime_weight_bps.is_some() || governance_weight_bps.is_some() || ecosystem_weight_bps.is_some() {
        let sum = new_uptime_w + new_gov_w + new_eco_w;
        if sum != 10_000 {
            return Err(ContractError::InvalidWeightSum { total: sum });
        }
    }
    config.uptime_weight_bps = new_uptime_w;
    config.governance_weight_bps = new_gov_w;
    config.ecosystem_weight_bps = new_eco_w;

    // If any compensation share is updated, validate the sum
    let new_base = base_compensation_share_bps.unwrap_or(config.base_compensation_share_bps);
    let new_bonus = performance_bonus_share_bps.unwrap_or(config.performance_bonus_share_bps);
    if base_compensation_share_bps.is_some() || performance_bonus_share_bps.is_some() {
        let sum = new_base + new_bonus;
        if sum != 10_000 {
            return Err(ContractError::InvalidWeightSum { total: sum });
        }
    }
    config.base_compensation_share_bps = new_base;
    config.performance_bonus_share_bps = new_bonus;

    if let Some(v) = min_per_category {
        config.min_per_category = v;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// ── Query ──────────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Validator { address } => to_json_binary(&query_validator(deps, address)?),
        QueryMsg::ActiveValidators {} => to_json_binary(&query_active_validators(deps)?),
        QueryMsg::ValidatorsByCategory { category } => {
            to_json_binary(&query_validators_by_category(deps, category)?)
        }
        QueryMsg::ValidatorsByStatus { status } => {
            to_json_binary(&query_validators_by_status(deps, status)?)
        }
        QueryMsg::PerformanceRecord { validator, period } => {
            to_json_binary(&query_performance_record(deps, validator, period)?)
        }
        QueryMsg::CompositionBreakdown {} => to_json_binary(&query_composition_breakdown(deps)?),
        QueryMsg::ModuleState {} => to_json_binary(&query_module_state(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_validator(deps: Deps, address: String) -> StdResult<ValidatorResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let validator = VALIDATORS.load(deps.storage, &addr)?;
    Ok(ValidatorResponse { validator })
}

fn query_active_validators(deps: Deps) -> StdResult<ValidatorsResponse> {
    let validators: Vec<AuthorityValidator> = ACTIVE_VALIDATORS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(addr, _)| VALIDATORS.load(deps.storage, &addr))
        .filter_map(|r| r.ok())
        .collect();
    Ok(ValidatorsResponse { validators })
}

fn query_validators_by_category(
    deps: Deps,
    category: ValidatorCategory,
) -> StdResult<ValidatorsResponse> {
    let validators: Vec<AuthorityValidator> = VALIDATORS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(_, v)| v)
        .filter(|v| v.category == category)
        .collect();
    Ok(ValidatorsResponse { validators })
}

fn query_validators_by_status(
    deps: Deps,
    status: ValidatorStatus,
) -> StdResult<ValidatorsResponse> {
    let validators: Vec<AuthorityValidator> = VALIDATORS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(_, v)| v)
        .filter(|v| v.status == status)
        .collect();
    Ok(ValidatorsResponse { validators })
}

fn query_performance_record(
    deps: Deps,
    validator: String,
    period: u64,
) -> StdResult<PerformanceRecordResponse> {
    let addr = deps.api.addr_validate(&validator)?;
    let record = PERFORMANCE_RECORDS.load(deps.storage, (&addr, period))?;
    Ok(PerformanceRecordResponse { record })
}

fn query_composition_breakdown(deps: Deps) -> StdResult<CompositionBreakdownResponse> {
    let mut infra = 0u32;
    let mut refi = 0u32;
    let mut eco = 0u32;

    let all: Vec<AuthorityValidator> = ACTIVE_VALIDATORS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(addr, _)| VALIDATORS.load(deps.storage, &addr))
        .filter_map(|r| r.ok())
        .collect();

    for v in &all {
        match v.category {
            ValidatorCategory::InfrastructureBuilders => infra += 1,
            ValidatorCategory::TrustedRefiPartners => refi += 1,
            ValidatorCategory::EcologicalDataStewards => eco += 1,
        }
    }

    Ok(CompositionBreakdownResponse {
        infrastructure_builders: infra,
        trusted_refi_partners: refi,
        ecological_data_stewards: eco,
        total_active: infra + refi + eco,
    })
}

fn query_module_state(deps: Deps) -> StdResult<ModuleStateResponse> {
    let state = MODULE_STATE.load(deps.storage)?;
    Ok(ModuleStateResponse { state })
}

// ── Helpers ────────────────────────────────────────────────────────────

fn require_admin(config: &Config, info: &MessageInfo) -> Result<(), ContractError> {
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "Only admin can perform this action".to_string(),
        });
    }
    Ok(())
}

fn load_validator(deps: Deps, addr: &Addr) -> Result<AuthorityValidator, ContractError> {
    VALIDATORS
        .load(deps.storage, addr)
        .map_err(|_| ContractError::ValidatorNotFound {
            address: addr.to_string(),
        })
}

fn validate_bps(value: u64) -> Result<(), ContractError> {
    if value > 10_000 {
        return Err(ContractError::InvalidBasisPoints { value });
    }
    Ok(())
}

fn count_active_in_category(
    deps: Deps,
    category: &ValidatorCategory,
) -> Result<u32, ContractError> {
    let count = ACTIVE_VALIDATORS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(addr, _)| VALIDATORS.load(deps.storage, &addr))
        .filter_map(|r| r.ok())
        .filter(|v| v.category == *category)
        .count() as u32;
    Ok(count)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
    use cosmwasm_std::{Addr, Coin, Uint128};

    const DENOM: &str = "uregen";

    fn addr(input: &str) -> Addr {
        MockApi::default().addr_make(input)
    }

    fn setup_contract(deps: DepsMut) -> MessageInfo {
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            min_validators: Some(2), // Low for testing
            max_validators: Some(6),
            term_length_seconds: None,
            probation_period_seconds: None,
            min_uptime_bps: None,
            performance_threshold_bps: None,
            uptime_weight_bps: None,
            governance_weight_bps: None,
            ecosystem_weight_bps: None,
            base_compensation_share_bps: None,
            performance_bonus_share_bps: None,
            min_per_category: Some(1), // Low for testing
            denom: DENOM.to_string(),
        };
        instantiate(deps, mock_env(), info.clone(), msg).unwrap();
        info
    }

    fn apply_validator(
        deps: DepsMut,
        sender: &Addr,
        category: ValidatorCategory,
    ) {
        let info = message_info(sender, &[]);
        let msg = ExecuteMsg::ApplyForValidator {
            category,
            application_data: "test application".to_string(),
        };
        execute(deps, mock_env(), info, msg).unwrap();
    }

    fn approve_validator(deps: DepsMut, admin: &Addr, applicant: &Addr) {
        let info = message_info(admin, &[]);
        let msg = ExecuteMsg::ApproveValidator {
            applicant: applicant.to_string(),
        };
        execute(deps, mock_env(), info, msg).unwrap();
    }

    fn activate_validator(deps: DepsMut, admin: &Addr, validator: &Addr) {
        let info = message_info(admin, &[]);
        let msg = ExecuteMsg::ActivateValidator {
            validator: validator.to_string(),
        };
        execute(deps, mock_env(), info, msg).unwrap();
    }

    // ── Test 1: Instantiate ────────────────────────────────────────────

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let info = setup_contract(deps.as_mut());

        let res: ConfigResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap(),
        )
        .unwrap();
        let c = res.config;
        assert_eq!(c.admin, info.sender);
        assert_eq!(c.min_validators, 2);
        assert_eq!(c.max_validators, 6);
        assert_eq!(c.term_length_seconds, 31_536_000);
        assert_eq!(c.probation_period_seconds, 2_592_000);
        assert_eq!(c.min_uptime_bps, 9950);
        assert_eq!(c.performance_threshold_bps, 7000);
        assert_eq!(c.uptime_weight_bps, 4000);
        assert_eq!(c.governance_weight_bps, 3000);
        assert_eq!(c.ecosystem_weight_bps, 3000);
        assert_eq!(c.base_compensation_share_bps, 9000);
        assert_eq!(c.performance_bonus_share_bps, 1000);
        assert_eq!(c.min_per_category, 1);
        assert_eq!(c.denom, DENOM);

        let state_res: ModuleStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::ModuleState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state_res.state.total_active, 0);
        assert_eq!(state_res.state.validator_fund_balance, Uint128::zero());
    }

    // ── Test 2: Apply + Approve + Activate lifecycle ───────────────────

    #[test]
    fn test_apply_approve_activate() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let val1 = addr("validator1");

        // Apply
        apply_validator(
            deps.as_mut(),
            &val1,
            ValidatorCategory::InfrastructureBuilders,
        );

        // Check candidate status
        let vr: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vr.validator.status, ValidatorStatus::Candidate);
        assert_eq!(
            vr.validator.category,
            ValidatorCategory::InfrastructureBuilders
        );

        // Approve
        approve_validator(deps.as_mut(), &admin, &val1);

        let vr: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vr.validator.status, ValidatorStatus::Approved);

        // Activate
        activate_validator(deps.as_mut(), &admin, &val1);

        let vr: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vr.validator.status, ValidatorStatus::Active);
        assert!(vr.validator.term_start.is_some());
        assert!(vr.validator.term_end.is_some());

        // Module state should reflect 1 active
        let state_res: ModuleStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::ModuleState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state_res.state.total_active, 1);

        // Active validators query
        let active_res: ValidatorsResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::ActiveValidators {}).unwrap(),
        )
        .unwrap();
        assert_eq!(active_res.validators.len(), 1);
        assert_eq!(active_res.validators[0].address, val1);
    }

    // ── Test 3: Performance report + score calculation ─────────────────

    #[test]
    fn test_performance_report() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let val1 = addr("validator1");

        apply_validator(
            deps.as_mut(),
            &val1,
            ValidatorCategory::TrustedRefiPartners,
        );
        approve_validator(deps.as_mut(), &admin, &val1);
        activate_validator(deps.as_mut(), &admin, &val1);

        // Submit performance: uptime=9800, gov=8000, eco=7500
        // Composite = (9800*4000 + 8000*3000 + 7500*3000) / 10000
        //           = (39200000 + 24000000 + 22500000) / 10000
        //           = 85700000 / 10000 = 8570
        let info = message_info(&admin, &[]);
        let msg = ExecuteMsg::SubmitPerformanceReport {
            validator: val1.to_string(),
            uptime_bps: 9800,
            governance_participation_bps: 8000,
            ecosystem_contribution_bps: 7500,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let score_attr = res
            .attributes
            .iter()
            .find(|a| a.key == "composite_score")
            .unwrap();
        assert_eq!(score_attr.value, "8570");

        // Verify cached score on validator
        let vr: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vr.validator.performance_score_bps, 8570);

        // Verify performance record
        let pr: PerformanceRecordResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PerformanceRecord {
                    validator: val1.to_string(),
                    period: 1,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(pr.record.uptime_bps, 9800);
        assert_eq!(pr.record.governance_participation_bps, 8000);
        assert_eq!(pr.record.ecosystem_contribution_bps, 7500);
        assert_eq!(pr.record.composite_score_bps, 8570);
    }

    // ── Test 4: Probation flow ─────────────────────────────────────────

    #[test]
    fn test_probation_flow() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let val1 = addr("validator1");

        apply_validator(
            deps.as_mut(),
            &val1,
            ValidatorCategory::EcologicalDataStewards,
        );
        approve_validator(deps.as_mut(), &admin, &val1);
        activate_validator(deps.as_mut(), &admin, &val1);

        // Submit low performance score to enable probation
        // uptime=5000, gov=5000, eco=5000
        // Composite = (5000*4000 + 5000*3000 + 5000*3000) / 10000 = 5000
        let info = message_info(&admin, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::SubmitPerformanceReport {
                validator: val1.to_string(),
                uptime_bps: 5000,
                governance_participation_bps: 5000,
                ecosystem_contribution_bps: 5000,
            },
        )
        .unwrap();

        // Initiate probation (score 5000 < threshold 7000)
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::InitiateProbation {
                validator: val1.to_string(),
                reason: "Low performance".to_string(),
            },
        )
        .unwrap();

        let vr: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vr.validator.status, ValidatorStatus::Probation);

        // Active count decremented
        let state_res: ModuleStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::ModuleState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state_res.state.total_active, 0);

        // Submit improved performance
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::SubmitPerformanceReport {
                validator: val1.to_string(),
                uptime_bps: 9500,
                governance_participation_bps: 8000,
                ecosystem_contribution_bps: 8000,
            },
        )
        .unwrap();

        // Restore from probation (score should now be above threshold)
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::RestoreFromProbation {
                validator: val1.to_string(),
            },
        )
        .unwrap();

        let vr: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vr.validator.status, ValidatorStatus::Active);

        // Active count restored
        let state_res: ModuleStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::ModuleState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state_res.state.total_active, 1);
    }

    // ── Test 5: Compensation distribution ──────────────────────────────

    #[test]
    fn test_compensation_distribution() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let val1 = addr("validator1");
        let val2 = addr("validator2");

        // Set up two active validators
        apply_validator(
            deps.as_mut(),
            &val1,
            ValidatorCategory::InfrastructureBuilders,
        );
        approve_validator(deps.as_mut(), &admin, &val1);
        activate_validator(deps.as_mut(), &admin, &val1);

        apply_validator(
            deps.as_mut(),
            &val2,
            ValidatorCategory::TrustedRefiPartners,
        );
        approve_validator(deps.as_mut(), &admin, &val2);
        activate_validator(deps.as_mut(), &admin, &val2);

        // Submit performance: val1 gets 8000, val2 gets 6000
        let admin_info = message_info(&admin, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            ExecuteMsg::SubmitPerformanceReport {
                validator: val1.to_string(),
                uptime_bps: 8000,
                governance_participation_bps: 8000,
                ecosystem_contribution_bps: 8000,
            },
        )
        .unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            ExecuteMsg::SubmitPerformanceReport {
                validator: val2.to_string(),
                uptime_bps: 6000,
                governance_participation_bps: 6000,
                ecosystem_contribution_bps: 6000,
            },
        )
        .unwrap();

        // Fund the validator pool with 10000 uregen
        let fund_info = message_info(&admin, &[Coin::new(10000u128, DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            fund_info,
            ExecuteMsg::UpdateValidatorFund {},
        )
        .unwrap();

        // Verify fund balance
        let state_res: ModuleStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::ModuleState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(
            state_res.state.validator_fund_balance,
            Uint128::new(10000)
        );

        // Distribute compensation
        execute(
            deps.as_mut(),
            mock_env(),
            admin_info,
            ExecuteMsg::DistributeCompensation {},
        )
        .unwrap();

        // Check compensation_due for each validator
        // Base pool = 10000 * 9000 / 10000 = 9000 → 4500 each
        // Bonus pool = 10000 * 1000 / 10000 = 1000
        // val1 score = 8000, val2 score = 6000, total = 14000
        // val1 bonus = 1000 * 8000 / 14000 = 571
        // val2 bonus = 1000 * 6000 / 14000 = 428
        // val1 total = 4500 + 571 = 5071
        // val2 total = 4500 + 428 = 4928

        let vr1: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        let vr2: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val2.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        // Due to integer division: base_per = 9000/2 = 4500
        // val1 bonus = 1000 * 8000 / 14000 = 571 (integer)
        // val2 bonus = 1000 * 6000 / 14000 = 428 (integer)
        assert_eq!(vr1.validator.compensation_due, Uint128::new(5071));
        assert_eq!(vr2.validator.compensation_due, Uint128::new(4928));

        // Total distributed = 5071 + 4928 = 9999 (1 lost to rounding)
        let state_res: ModuleStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::ModuleState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state_res.state.validator_fund_balance, Uint128::new(1)); // rounding dust

        // val1 claims
        let val1_info = message_info(&val1, &[]);
        let claim_res = execute(
            deps.as_mut(),
            mock_env(),
            val1_info,
            ExecuteMsg::ClaimCompensation {},
        )
        .unwrap();

        // Should have a BankMsg::Send
        assert_eq!(claim_res.messages.len(), 1);

        // Compensation should be zero after claim
        let vr1: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vr1.validator.compensation_due, Uint128::zero());
    }

    // ── Test 6: Duplicate application rejected ─────────────────────────

    #[test]
    fn test_duplicate_application_rejected() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let val1 = addr("validator1");
        apply_validator(
            deps.as_mut(),
            &val1,
            ValidatorCategory::InfrastructureBuilders,
        );

        // Second application should fail
        let info = message_info(&val1, &[]);
        let msg = ExecuteMsg::ApplyForValidator {
            category: ValidatorCategory::TrustedRefiPartners,
            application_data: "second attempt".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::ValidatorAlreadyExists { .. }));
    }

    // ── Test 7: Non-admin cannot approve ───────────────────────────────

    #[test]
    fn test_non_admin_cannot_approve() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let val1 = addr("validator1");
        let impostor = addr("impostor");

        apply_validator(
            deps.as_mut(),
            &val1,
            ValidatorCategory::InfrastructureBuilders,
        );

        let info = message_info(&impostor, &[]);
        let msg = ExecuteMsg::ApproveValidator {
            applicant: val1.to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));
    }

    // ── Test 8: End term after expiry ──────────────────────────────────

    #[test]
    fn test_end_term() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let val1 = addr("validator1");

        apply_validator(
            deps.as_mut(),
            &val1,
            ValidatorCategory::EcologicalDataStewards,
        );
        approve_validator(deps.as_mut(), &admin, &val1);
        activate_validator(deps.as_mut(), &admin, &val1);

        // Try ending term before expiry — should fail
        let info = message_info(&admin, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::EndValidatorTerm {
                validator: val1.to_string(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::TermNotEnded));

        // Advance time past term end (12 months + 1 second)
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(31_536_001);

        execute(
            deps.as_mut(),
            env.clone(),
            info,
            ExecuteMsg::EndValidatorTerm {
                validator: val1.to_string(),
            },
        )
        .unwrap();

        let vr: ValidatorResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                env,
                QueryMsg::Validator {
                    address: val1.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(vr.validator.status, ValidatorStatus::TermExpired);
    }

    // ── Test 9: Composition breakdown query ────────────────────────────

    #[test]
    fn test_composition_breakdown() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let v1 = addr("v1");
        let v2 = addr("v2");
        let v3 = addr("v3");

        apply_validator(
            deps.as_mut(),
            &v1,
            ValidatorCategory::InfrastructureBuilders,
        );
        approve_validator(deps.as_mut(), &admin, &v1);
        activate_validator(deps.as_mut(), &admin, &v1);

        apply_validator(
            deps.as_mut(),
            &v2,
            ValidatorCategory::TrustedRefiPartners,
        );
        approve_validator(deps.as_mut(), &admin, &v2);
        activate_validator(deps.as_mut(), &admin, &v2);

        apply_validator(
            deps.as_mut(),
            &v3,
            ValidatorCategory::EcologicalDataStewards,
        );
        approve_validator(deps.as_mut(), &admin, &v3);
        activate_validator(deps.as_mut(), &admin, &v3);

        let breakdown: CompositionBreakdownResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::CompositionBreakdown {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(breakdown.infrastructure_builders, 1);
        assert_eq!(breakdown.trusted_refi_partners, 1);
        assert_eq!(breakdown.ecological_data_stewards, 1);
        assert_eq!(breakdown.total_active, 3);
    }

    // ── Test 10: Weight validation on instantiate ──────────────────────

    #[test]
    fn test_invalid_weight_sum() {
        let mut deps = mock_dependencies();
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            min_validators: None,
            max_validators: None,
            term_length_seconds: None,
            probation_period_seconds: None,
            min_uptime_bps: None,
            performance_threshold_bps: None,
            uptime_weight_bps: Some(5000),
            governance_weight_bps: Some(3000),
            ecosystem_weight_bps: Some(3000), // sum = 11000, invalid
            base_compensation_share_bps: None,
            performance_bonus_share_bps: None,
            min_per_category: None,
            denom: DENOM.to_string(),
        };
        let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::InvalidWeightSum { total: 11000 }));
    }
}
