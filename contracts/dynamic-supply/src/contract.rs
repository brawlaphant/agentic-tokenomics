use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, MintBurnRecordResponse, QueryMsg, SimulatePeriodResponse,
    SupplyParamsResponse, SupplyStateResponse,
};
use crate::state::{
    M014Phase, MintBurnRecord, SupplyParams, SupplyPhase, SupplyState, MINT_BURN_HISTORY,
    SUPPLY_PARAMS, SUPPLY_STATE,
};

/// Maximum allowed base regrowth rate: 10% (0.10).
/// Decimal stores 18 decimal places, so 0.1 = 100_000_000_000_000_000.
const MAX_REGROWTH_RATE: Decimal = Decimal::raw(100_000_000_000_000_000);

/// Maximum effective multiplier (staking or stability): 2.0
const MAX_MULTIPLIER: Decimal = Decimal::raw(2_000_000_000_000_000_000);

/// Minimum multiplier floor: 1.0
const MIN_MULTIPLIER: Decimal = Decimal::raw(1_000_000_000_000_000_000);

// ---------------------------------------------------------------------------
// Instantiate
// ---------------------------------------------------------------------------

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Validate hard cap > 0
    if msg.hard_cap.is_zero() {
        return Err(ContractError::ZeroCap {});
    }

    // Validate initial supply <= hard cap
    if msg.initial_supply > msg.hard_cap {
        return Err(ContractError::SupplyExceedsCap {
            current: msg.initial_supply.to_string(),
            cap: msg.hard_cap.to_string(),
        });
    }

    // Validate regrowth rate in [0, 0.10]
    validate_regrowth_rate(msg.base_regrowth_rate)?;

    // Validate ecological reference value > 0 if multiplier is enabled
    if msg.ecological_multiplier_enabled && msg.ecological_reference_value.is_zero() {
        return Err(ContractError::ZeroReferenceValue {});
    }

    let params = SupplyParams {
        admin: info.sender.clone(),
        hard_cap: msg.hard_cap,
        base_regrowth_rate: msg.base_regrowth_rate,
        ecological_multiplier_enabled: msg.ecological_multiplier_enabled,
        ecological_reference_value: msg.ecological_reference_value,
        m014_phase: msg.m014_phase,
        equilibrium_threshold: msg.equilibrium_threshold,
        equilibrium_periods_required: msg.equilibrium_periods_required,
    };
    SUPPLY_PARAMS.save(deps.storage, &params)?;

    let state = SupplyState {
        current_supply: msg.initial_supply,
        total_minted: Uint128::zero(),
        total_burned: Uint128::zero(),
        period_count: 0,
        phase: SupplyPhase::Transition,
        consecutive_equilibrium_periods: 0,
    };
    SUPPLY_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender)
        .add_attribute("hard_cap", msg.hard_cap)
        .add_attribute("initial_supply", msg.initial_supply)
        .add_attribute("base_regrowth_rate", msg.base_regrowth_rate.to_string()))
}

// ---------------------------------------------------------------------------
// Execute
// ---------------------------------------------------------------------------

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ExecutePeriod {
            burn_amount,
            staked_amount,
            stability_committed,
            delta_co2,
        } => execute_period(
            deps,
            env,
            info,
            burn_amount,
            staked_amount,
            stability_committed,
            delta_co2,
        ),
        ExecuteMsg::UpdateRegrowthRate { rate } => execute_update_regrowth_rate(deps, info, rate),
        ExecuteMsg::UpdateM014Phase { phase } => execute_update_m014_phase(deps, info, phase),
        ExecuteMsg::SetEcologicalMultiplier {
            enabled,
            reference_value,
        } => execute_set_ecological_multiplier(deps, info, enabled, reference_value),
        ExecuteMsg::UpdateEquilibriumParams {
            threshold,
            periods_required,
        } => execute_update_equilibrium_params(deps, info, threshold, periods_required),
    }
}

/// Execute a mint/burn period.
///
/// Implements the core supply algorithm from M012 SPEC section 5:
///   S[t+1] = S[t] + M[t] - B[t]
///   M[t] = r * (C - S[t])
///   r = r_base * effective_multiplier * ecological_multiplier
fn execute_period(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    burn_amount: Uint128,
    staked_amount: Uint128,
    stability_committed: Uint128,
    delta_co2: Option<Decimal>,
) -> Result<Response, ContractError> {
    let params = SUPPLY_PARAMS.load(deps.storage)?;

    // Admin-only
    if info.sender != params.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut state = SUPPLY_STATE.load(deps.storage)?;

    // Compute the period
    let (mint_amount, effective_mult, eco_mult, regrowth_rate) =
        compute_mint(&params, &state, staked_amount, stability_committed, delta_co2);

    // Apply supply adjustment: S[t+1] = S[t] + M[t] - B[t]
    let supply_before = state.current_supply;
    let supply_after_mint = state.current_supply + mint_amount;

    // Cap enforcement: S[t+1] <= hard_cap
    let capped_supply = if supply_after_mint > params.hard_cap {
        params.hard_cap
    } else {
        supply_after_mint
    };

    // Non-negative supply: S[t+1] >= 0
    let supply_after = if burn_amount >= capped_supply {
        Uint128::zero()
    } else {
        capped_supply - burn_amount
    };

    // Effective mint (may be reduced by cap)
    let effective_mint = if capped_supply > state.current_supply {
        capped_supply - state.current_supply
    } else {
        Uint128::zero()
    };

    // Effective burn (may be reduced by zero-floor)
    let effective_burn = if burn_amount >= capped_supply {
        capped_supply
    } else {
        burn_amount
    };

    // Update state
    state.current_supply = supply_after;
    state.total_minted += effective_mint;
    state.total_burned += effective_burn;
    state.period_count += 1;

    // Phase transitions
    let mint_burn_diff = if effective_mint > effective_burn {
        effective_mint - effective_burn
    } else {
        effective_burn - effective_mint
    };

    // TRANSITION -> DYNAMIC: first successful burn period
    if state.phase == SupplyPhase::Transition && effective_burn > Uint128::zero() {
        state.phase = SupplyPhase::Dynamic;
        state.consecutive_equilibrium_periods = 0;
    }

    // DYNAMIC <-> EQUILIBRIUM detection
    if state.phase == SupplyPhase::Dynamic || state.phase == SupplyPhase::Equilibrium {
        if mint_burn_diff < params.equilibrium_threshold {
            state.consecutive_equilibrium_periods += 1;
            if state.consecutive_equilibrium_periods >= params.equilibrium_periods_required {
                state.phase = SupplyPhase::Equilibrium;
            }
        } else {
            state.consecutive_equilibrium_periods = 0;
            if state.phase == SupplyPhase::Equilibrium {
                state.phase = SupplyPhase::Dynamic;
            }
        }
    }

    // Record period history
    let record = MintBurnRecord {
        period_id: state.period_count,
        block_height: env.block.height,
        minted: effective_mint,
        burned: effective_burn,
        supply_before,
        supply_after,
        regrowth_rate,
        effective_multiplier: effective_mult,
        ecological_multiplier: eco_mult,
    };
    MINT_BURN_HISTORY.save(deps.storage, state.period_count, &record)?;

    SUPPLY_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "execute_period")
        .add_attribute("period_id", state.period_count.to_string())
        .add_attribute("minted", effective_mint)
        .add_attribute("burned", effective_burn)
        .add_attribute("supply_before", supply_before)
        .add_attribute("supply_after", supply_after)
        .add_attribute("phase", format!("{:?}", state.phase))
        .add_attribute("regrowth_rate", regrowth_rate.to_string())
        .add_attribute("effective_multiplier", effective_mult.to_string())
        .add_attribute("ecological_multiplier", eco_mult.to_string()))
}

fn execute_update_regrowth_rate(
    deps: DepsMut,
    info: MessageInfo,
    rate: Decimal,
) -> Result<Response, ContractError> {
    let mut params = SUPPLY_PARAMS.load(deps.storage)?;

    if info.sender != params.admin {
        return Err(ContractError::Unauthorized {});
    }

    validate_regrowth_rate(rate)?;
    params.base_regrowth_rate = rate;
    SUPPLY_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new()
        .add_attribute("action", "update_regrowth_rate")
        .add_attribute("rate", rate.to_string()))
}

fn execute_update_m014_phase(
    deps: DepsMut,
    info: MessageInfo,
    phase: M014Phase,
) -> Result<Response, ContractError> {
    let mut params = SUPPLY_PARAMS.load(deps.storage)?;

    if info.sender != params.admin {
        return Err(ContractError::Unauthorized {});
    }

    params.m014_phase = phase.clone();
    SUPPLY_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new()
        .add_attribute("action", "update_m014_phase")
        .add_attribute("phase", format!("{:?}", phase)))
}

fn execute_set_ecological_multiplier(
    deps: DepsMut,
    info: MessageInfo,
    enabled: bool,
    reference_value: Option<Decimal>,
) -> Result<Response, ContractError> {
    let mut params = SUPPLY_PARAMS.load(deps.storage)?;

    if info.sender != params.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(ref_val) = reference_value {
        if ref_val.is_zero() {
            return Err(ContractError::ZeroReferenceValue {});
        }
        params.ecological_reference_value = ref_val;
    }

    if enabled && params.ecological_reference_value.is_zero() {
        return Err(ContractError::ZeroReferenceValue {});
    }

    params.ecological_multiplier_enabled = enabled;
    SUPPLY_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new()
        .add_attribute("action", "set_ecological_multiplier")
        .add_attribute("enabled", enabled.to_string())
        .add_attribute(
            "reference_value",
            params.ecological_reference_value.to_string(),
        ))
}

fn execute_update_equilibrium_params(
    deps: DepsMut,
    info: MessageInfo,
    threshold: Option<Uint128>,
    periods_required: Option<u64>,
) -> Result<Response, ContractError> {
    let mut params = SUPPLY_PARAMS.load(deps.storage)?;

    if info.sender != params.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(t) = threshold {
        params.equilibrium_threshold = t;
    }
    if let Some(p) = periods_required {
        params.equilibrium_periods_required = p;
    }

    SUPPLY_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new()
        .add_attribute("action", "update_equilibrium_params")
        .add_attribute("threshold", params.equilibrium_threshold)
        .add_attribute(
            "periods_required",
            params.equilibrium_periods_required.to_string(),
        ))
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::SupplyState {} => to_json_binary(&query_supply_state(deps)?),
        QueryMsg::SupplyParams {} => to_json_binary(&query_supply_params(deps)?),
        QueryMsg::PeriodHistory { period_id } => {
            to_json_binary(&query_period_history(deps, period_id)?)
        }
        QueryMsg::SimulatePeriod {
            burn_amount,
            staked_amount,
            stability_committed,
            delta_co2,
        } => to_json_binary(&query_simulate_period(
            deps,
            burn_amount,
            staked_amount,
            stability_committed,
            delta_co2,
        )?),
    }
}

fn query_supply_state(deps: Deps) -> StdResult<SupplyStateResponse> {
    let state = SUPPLY_STATE.load(deps.storage)?;
    let params = SUPPLY_PARAMS.load(deps.storage)?;

    let cap_headroom = if params.hard_cap > state.current_supply {
        params.hard_cap - state.current_supply
    } else {
        Uint128::zero()
    };

    Ok(SupplyStateResponse {
        current_supply: state.current_supply,
        hard_cap: params.hard_cap,
        total_minted: state.total_minted,
        total_burned: state.total_burned,
        period_count: state.period_count,
        phase: state.phase,
        cap_headroom,
        consecutive_equilibrium_periods: state.consecutive_equilibrium_periods,
    })
}

fn query_supply_params(deps: Deps) -> StdResult<SupplyParamsResponse> {
    let params = SUPPLY_PARAMS.load(deps.storage)?;

    Ok(SupplyParamsResponse {
        admin: params.admin.to_string(),
        hard_cap: params.hard_cap,
        base_regrowth_rate: params.base_regrowth_rate,
        ecological_multiplier_enabled: params.ecological_multiplier_enabled,
        ecological_reference_value: params.ecological_reference_value,
        m014_phase: params.m014_phase,
        equilibrium_threshold: params.equilibrium_threshold,
        equilibrium_periods_required: params.equilibrium_periods_required,
    })
}

fn query_period_history(deps: Deps, period_id: u64) -> StdResult<MintBurnRecordResponse> {
    let record = MINT_BURN_HISTORY
        .load(deps.storage, period_id)
        .map_err(|_| StdError::not_found(format!("MintBurnRecord for period {}", period_id)))?;

    Ok(MintBurnRecordResponse { record })
}

fn query_simulate_period(
    deps: Deps,
    burn_amount: Uint128,
    staked_amount: Uint128,
    stability_committed: Uint128,
    delta_co2: Option<Decimal>,
) -> StdResult<SimulatePeriodResponse> {
    let params = SUPPLY_PARAMS.load(deps.storage)?;
    let state = SUPPLY_STATE.load(deps.storage)?;

    let (mint_amount, effective_mult, eco_mult, regrowth_rate) =
        compute_mint(&params, &state, staked_amount, stability_committed, delta_co2);

    let supply_after_mint = state.current_supply + mint_amount;
    let capped_supply = if supply_after_mint > params.hard_cap {
        params.hard_cap
    } else {
        supply_after_mint
    };

    let supply_after = if burn_amount >= capped_supply {
        Uint128::zero()
    } else {
        capped_supply - burn_amount
    };

    // Check if this period would move toward equilibrium
    let effective_mint = if capped_supply > state.current_supply {
        capped_supply - state.current_supply
    } else {
        Uint128::zero()
    };
    let effective_burn = if burn_amount >= capped_supply {
        capped_supply
    } else {
        burn_amount
    };
    let diff = if effective_mint > effective_burn {
        effective_mint - effective_burn
    } else {
        effective_burn - effective_mint
    };
    let would_reach_equilibrium = diff < params.equilibrium_threshold
        && state.consecutive_equilibrium_periods + 1 >= params.equilibrium_periods_required;

    Ok(SimulatePeriodResponse {
        mint_amount,
        burn_amount,
        supply_before: state.current_supply,
        supply_after,
        regrowth_rate,
        effective_multiplier: effective_mult,
        ecological_multiplier: eco_mult,
        would_reach_equilibrium,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute the mint amount for a period.
///
/// M[t] = r * (C - S[t])
/// r = r_base * effective_multiplier * ecological_multiplier
///
/// Returns: (mint_amount, effective_multiplier, ecological_multiplier, regrowth_rate)
fn compute_mint(
    params: &SupplyParams,
    state: &SupplyState,
    staked_amount: Uint128,
    stability_committed: Uint128,
    delta_co2: Option<Decimal>,
) -> (Uint128, Decimal, Decimal, Decimal) {
    // Headroom: C - S[t]
    let headroom = if params.hard_cap > state.current_supply {
        params.hard_cap - state.current_supply
    } else {
        return (Uint128::zero(), Decimal::one(), Decimal::one(), Decimal::zero());
    };

    // Compute staking multiplier: clamp(1 + S_staked / S_total, 1.0, 2.0)
    let staking_multiplier = compute_staking_multiplier(staked_amount, state.current_supply);

    // Compute stability multiplier: clamp(1 + S_stability / S_total, 1.0, 2.0)
    let stability_multiplier =
        compute_stability_multiplier(stability_committed, state.current_supply);

    // Phase-gated effective multiplier selection (SPEC 5.3)
    let effective_multiplier = match params.m014_phase {
        M014Phase::Inactive => staking_multiplier,
        M014Phase::Transition => {
            if staking_multiplier > stability_multiplier {
                staking_multiplier
            } else {
                stability_multiplier
            }
        }
        M014Phase::Active | M014Phase::Equilibrium => stability_multiplier,
    };

    // Ecological multiplier (SPEC 5.4)
    let ecological_multiplier = if params.ecological_multiplier_enabled {
        compute_ecological_multiplier(delta_co2, params.ecological_reference_value)
    } else {
        Decimal::one()
    };

    // r = r_base * effective_multiplier * ecological_multiplier
    let regrowth_rate = params.base_regrowth_rate * effective_multiplier * ecological_multiplier;

    // M[t] = r * headroom (floor division via mul_floor)
    let mint_amount = headroom.mul_floor(regrowth_rate);

    (
        mint_amount,
        effective_multiplier,
        ecological_multiplier,
        regrowth_rate,
    )
}

/// Compute staking multiplier: clamp(1 + S_staked / S_total, 1.0, 2.0)
///
/// If current_supply is zero, returns 1.0 (minimum).
fn compute_staking_multiplier(staked: Uint128, current_supply: Uint128) -> Decimal {
    if current_supply.is_zero() {
        return MIN_MULTIPLIER;
    }

    let ratio = Decimal::from_ratio(staked, current_supply);
    let raw = Decimal::one() + ratio;

    clamp_decimal(raw, MIN_MULTIPLIER, MAX_MULTIPLIER)
}

/// Compute stability multiplier: clamp(1 + S_stability / S_total, 1.0, 2.0)
///
/// If current_supply is zero, returns 1.0 (minimum).
fn compute_stability_multiplier(stability_committed: Uint128, current_supply: Uint128) -> Decimal {
    if current_supply.is_zero() {
        return MIN_MULTIPLIER;
    }

    let ratio = Decimal::from_ratio(stability_committed, current_supply);
    let raw = Decimal::one() + ratio;

    clamp_decimal(raw, MIN_MULTIPLIER, MAX_MULTIPLIER)
}

/// Compute ecological multiplier: max(0, 1 - delta_co2 / reference_value)
///
/// Returns 1.0 if delta_co2 is None (disabled / no data).
fn compute_ecological_multiplier(
    delta_co2: Option<Decimal>,
    reference_value: Decimal,
) -> Decimal {
    match delta_co2 {
        None => Decimal::one(),
        Some(delta) => {
            if reference_value.is_zero() {
                return Decimal::one();
            }
            let ratio = delta / reference_value;
            if ratio >= Decimal::one() {
                Decimal::zero()
            } else {
                Decimal::one() - ratio
            }
        }
    }
}

/// Clamp a Decimal to [min, max].
fn clamp_decimal(val: Decimal, min: Decimal, max: Decimal) -> Decimal {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

/// Validate that the regrowth rate is within [0, MAX_REGROWTH_RATE].
fn validate_regrowth_rate(rate: Decimal) -> Result<(), ContractError> {
    if rate > MAX_REGROWTH_RATE {
        return Err(ContractError::RegrowthRateExceedsBound {
            rate: rate.to_string(),
        });
    }
    Ok(())
}
