use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};

use crate::error::ContractError;
use crate::msg::{
    CalculateFeeResponse, ExecuteMsg, FeeConfigResponse, InstantiateMsg, PoolBalancesResponse,
    QueryMsg, TxType,
};
use crate::state::{FeeConfig, PoolBalances, FEE_CONFIG, POOL_BALANCES};

/// Maximum allowed fee rate: 10%% (0.1).
/// Decimal stores 18 decimal places, so 0.1 = 100_000_000_000_000_000.
const MAX_FEE_RATE: Decimal = Decimal::raw(100_000_000_000_000_000);

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
    // Validate all rates are within [0, MAX_FEE_RATE]
    validate_rate(msg.issuance_rate, MAX_FEE_RATE)?;
    validate_rate(msg.transfer_rate, MAX_FEE_RATE)?;
    validate_rate(msg.retirement_rate, MAX_FEE_RATE)?;
    validate_rate(msg.trade_rate, MAX_FEE_RATE)?;

    // Validate distribution shares sum to 1.0
    validate_shares(
        msg.burn_share,
        msg.validator_share,
        msg.community_share,
        msg.agent_share,
    )?;

    let config = FeeConfig {
        admin: info.sender.clone(),
        issuance_rate: msg.issuance_rate,
        transfer_rate: msg.transfer_rate,
        retirement_rate: msg.retirement_rate,
        trade_rate: msg.trade_rate,
        burn_share: msg.burn_share,
        validator_share: msg.validator_share,
        community_share: msg.community_share,
        agent_share: msg.agent_share,
        min_fee: msg.min_fee,
    };
    FEE_CONFIG.save(deps.storage, &config)?;

    let pools = PoolBalances {
        burn_pool: Uint128::zero(),
        validator_fund: Uint128::zero(),
        community_pool: Uint128::zero(),
        agent_infra: Uint128::zero(),
    };
    POOL_BALANCES.save(deps.storage, &pools)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender))
}

// ---------------------------------------------------------------------------
// Execute
// ---------------------------------------------------------------------------

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CollectFee { tx_type, value } => execute_collect_fee(deps, tx_type, value),
        ExecuteMsg::UpdateFeeRate { tx_type, rate } => {
            execute_update_fee_rate(deps, info, tx_type, rate)
        }
        ExecuteMsg::UpdateDistribution {
            burn_share,
            validator_share,
            community_share,
            agent_share,
        } => execute_update_distribution(
            deps,
            info,
            burn_share,
            validator_share,
            community_share,
            agent_share,
        ),
    }
}

fn execute_collect_fee(
    deps: DepsMut,
    tx_type: TxType,
    value: Uint128,
) -> Result<Response, ContractError> {
    if value.is_zero() {
        return Err(ContractError::ZeroValue {});
    }

    let config = FEE_CONFIG.load(deps.storage)?;

    let (fee_amount, min_fee_applied) = calculate_fee_amount(&config, &tx_type, value);
    let (burn, validator, community, agent) = distribute_fee(&config, fee_amount);

    // Update pool balances
    POOL_BALANCES.update(deps.storage, |mut pools| -> StdResult<PoolBalances> {
        pools.burn_pool += burn;
        pools.validator_fund += validator;
        pools.community_pool += community;
        pools.agent_infra += agent;
        Ok(pools)
    })?;

    Ok(Response::new()
        .add_attribute("action", "collect_fee")
        .add_attribute("tx_type", format!("{:?}", tx_type))
        .add_attribute("value", value)
        .add_attribute("fee_amount", fee_amount)
        .add_attribute("min_fee_applied", min_fee_applied.to_string())
        .add_attribute("burn", burn)
        .add_attribute("validator", validator)
        .add_attribute("community", community)
        .add_attribute("agent", agent))
}

fn execute_update_fee_rate(
    deps: DepsMut,
    info: MessageInfo,
    tx_type: TxType,
    rate: Decimal,
) -> Result<Response, ContractError> {
    let mut config = FEE_CONFIG.load(deps.storage)?;

    // Admin-only
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Validate rate within [0, MAX_FEE_RATE]
    validate_rate(rate, MAX_FEE_RATE)?;

    match tx_type {
        TxType::CreditIssuance => config.issuance_rate = rate,
        TxType::CreditTransfer => config.transfer_rate = rate,
        TxType::CreditRetirement => config.retirement_rate = rate,
        TxType::MarketplaceTrade => config.trade_rate = rate,
    }

    FEE_CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_fee_rate")
        .add_attribute("tx_type", format!("{:?}", tx_type))
        .add_attribute("rate", rate.to_string()))
}

fn execute_update_distribution(
    deps: DepsMut,
    info: MessageInfo,
    burn_share: Decimal,
    validator_share: Decimal,
    community_share: Decimal,
    agent_share: Decimal,
) -> Result<Response, ContractError> {
    let mut config = FEE_CONFIG.load(deps.storage)?;

    // Admin-only
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Validate shares sum to 1.0
    validate_shares(burn_share, validator_share, community_share, agent_share)?;

    config.burn_share = burn_share;
    config.validator_share = validator_share;
    config.community_share = community_share;
    config.agent_share = agent_share;

    FEE_CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_distribution")
        .add_attribute("burn_share", burn_share.to_string())
        .add_attribute("validator_share", validator_share.to_string())
        .add_attribute("community_share", community_share.to_string())
        .add_attribute("agent_share", agent_share.to_string()))
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::FeeConfig {} => to_json_binary(&query_fee_config(deps)?),
        QueryMsg::PoolBalances {} => to_json_binary(&query_pool_balances(deps)?),
        QueryMsg::CalculateFee { tx_type, value } => {
            to_json_binary(&query_calculate_fee(deps, tx_type, value)?)
        }
    }
}

fn query_fee_config(deps: Deps) -> StdResult<FeeConfigResponse> {
    let config = FEE_CONFIG.load(deps.storage)?;
    Ok(FeeConfigResponse {
        admin: config.admin.to_string(),
        issuance_rate: config.issuance_rate,
        transfer_rate: config.transfer_rate,
        retirement_rate: config.retirement_rate,
        trade_rate: config.trade_rate,
        burn_share: config.burn_share,
        validator_share: config.validator_share,
        community_share: config.community_share,
        agent_share: config.agent_share,
        min_fee: config.min_fee,
    })
}

fn query_pool_balances(deps: Deps) -> StdResult<PoolBalancesResponse> {
    let pools = POOL_BALANCES.load(deps.storage)?;
    Ok(PoolBalancesResponse {
        burn_pool: pools.burn_pool,
        validator_fund: pools.validator_fund,
        community_pool: pools.community_pool,
        agent_infra: pools.agent_infra,
    })
}

fn query_calculate_fee(
    deps: Deps,
    tx_type: TxType,
    value: Uint128,
) -> StdResult<CalculateFeeResponse> {
    let config = FEE_CONFIG.load(deps.storage)?;
    let (fee_amount, min_fee_applied) = calculate_fee_amount(&config, &tx_type, value);
    let (burn, validator, community, agent) = distribute_fee(&config, fee_amount);

    Ok(CalculateFeeResponse {
        fee_amount,
        min_fee_applied,
        burn_amount: burn,
        validator_amount: validator,
        community_amount: community,
        agent_amount: agent,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the fee rate for a given transaction type.
fn get_rate(config: &FeeConfig, tx_type: &TxType) -> Decimal {
    match tx_type {
        TxType::CreditIssuance => config.issuance_rate,
        TxType::CreditTransfer => config.transfer_rate,
        TxType::CreditRetirement => config.retirement_rate,
        TxType::MarketplaceTrade => config.trade_rate,
    }
}

/// Calculate the fee amount for a transaction.
///
/// fee_amount = max(value * rate, min_fee)
///
/// Uses Decimal multiplication which truncates (floor) by default,
/// matching the JS reference implementation's Math.floor behavior.
fn calculate_fee_amount(
    config: &FeeConfig,
    tx_type: &TxType,
    value: Uint128,
) -> (Uint128, bool) {
    let rate = get_rate(config, tx_type);
    let raw_fee = value.mul_floor(rate); // floor(value * rate)
    let min_fee_applied = raw_fee < config.min_fee;
    let fee_amount = if min_fee_applied {
        config.min_fee
    } else {
        raw_fee
    };
    (fee_amount, min_fee_applied)
}

/// Distribute a fee amount across the four pools.
///
/// Three pools (burn, community, agent) use floor division;
/// the validator fund receives the remainder to preserve the
/// Fee Conservation invariant (fee_amount == sum of all distributions).
fn distribute_fee(config: &FeeConfig, fee_amount: Uint128) -> (Uint128, Uint128, Uint128, Uint128) {
    let burn = fee_amount.mul_floor(config.burn_share);
    let community = fee_amount.mul_floor(config.community_share);
    let agent = fee_amount.mul_floor(config.agent_share);
    let validator = fee_amount - burn - community - agent;

    (burn, validator, community, agent)
}

/// Validate that a fee rate is within [0, max_rate].
fn validate_rate(rate: Decimal, max_rate: Decimal) -> Result<(), ContractError> {
    if rate > max_rate {
        return Err(ContractError::RateExceedsCap {
            rate: rate.to_string(),
        });
    }
    Ok(())
}

/// Validate that distribution shares sum to exactly 1.0.
fn validate_shares(
    burn_share: Decimal,
    validator_share: Decimal,
    community_share: Decimal,
    agent_share: Decimal,
) -> Result<(), ContractError> {
    let sum = burn_share + validator_share + community_share + agent_share;
    if sum != Decimal::one() {
        return Err(ContractError::ShareSumNotUnity {
            sum: sum.to_string(),
        });
    }
    Ok(())
}
