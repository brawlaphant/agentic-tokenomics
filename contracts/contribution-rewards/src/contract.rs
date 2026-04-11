use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Timestamp, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ActivityScoreResponse, ClaimableRewardsResponse, ConfigResponse, DistributionRecordResponse,
    ExecuteMsg, InstantiateMsg, MechanismStateResponse, ParticipantRewardsResponse, QueryMsg,
    StabilityCommitmentResponse,
};
use crate::state::{
    ActivityScore, CommitmentStatus, Config, DistributionRecord, MechanismState, MechanismStatus,
    StabilityCommitment, ACTIVITY_SCORES, CLAIMABLE_REWARDS, COMMITMENTS, CONFIG, DISTRIBUTIONS,
    MECHANISM_STATE, NEXT_COMMITMENT_ID,
};

const CONTRACT_NAME: &str = "crates.io:contribution-rewards";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Seconds per month approximation (30.44 days)
const SECONDS_PER_MONTH: u64 = 2_629_744;

// ── Instantiate ────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        admin: info.sender.clone(),
        community_pool_addr: deps.api.addr_validate(&msg.community_pool_addr)?,
        denom: msg.denom,
        credit_purchase_weight: 3000,
        credit_retirement_weight: 3000,
        platform_facilitation_weight: 2000,
        governance_voting_weight: 1000,
        proposal_submission_weight: 1000,
        stability_annual_return_bps: 600,
        max_stability_share_bps: 3000,
        min_commitment_amount: Uint128::new(100_000_000), // 100 REGEN
        min_lock_months: 6,
        max_lock_months: 24,
        early_exit_penalty_bps: 5000,
        period_seconds: 604_800, // 7 days
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;

    let state = MechanismState {
        status: MechanismStatus::Inactive,
        tracking_start: None,
        current_period: 0,
        last_distribution_period: None,
        total_committed_principal: Uint128::zero(),
    };
    MECHANISM_STATE.save(deps.storage, &state)?;
    NEXT_COMMITMENT_ID.save(deps.storage, &1u64)?;

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
        ExecuteMsg::InitializeMechanism {} => execute_initialize(deps, env, info),
        ExecuteMsg::ActivateDistribution {} => execute_activate(deps, info),
        ExecuteMsg::CommitStability { lock_months } => {
            execute_commit_stability(deps, env, info, lock_months)
        }
        ExecuteMsg::ExitStabilityEarly { commitment_id } => {
            execute_exit_early(deps, env, info, commitment_id)
        }
        ExecuteMsg::ClaimMaturedStability { commitment_id } => {
            execute_claim_matured(deps, env, info, commitment_id)
        }
        ExecuteMsg::RecordActivity {
            participant,
            credit_purchase_value,
            credit_retirement_value,
            platform_facilitation_value,
            governance_votes,
            proposal_credits,
        } => execute_record_activity(
            deps,
            info,
            participant,
            credit_purchase_value,
            credit_retirement_value,
            platform_facilitation_value,
            governance_votes,
            proposal_credits,
        ),
        ExecuteMsg::TriggerDistribution {
            community_pool_inflow,
        } => execute_trigger_distribution(deps, env, info, community_pool_inflow),
        ExecuteMsg::ClaimRewards {} => execute_claim_rewards(deps, info),
        ExecuteMsg::UpdateConfig {
            community_pool_addr,
            credit_purchase_weight,
            credit_retirement_weight,
            platform_facilitation_weight,
            governance_voting_weight,
            proposal_submission_weight,
            stability_annual_return_bps,
            max_stability_share_bps,
            min_commitment_amount,
            min_lock_months,
            max_lock_months,
            early_exit_penalty_bps,
            period_seconds,
        } => execute_update_config(
            deps,
            info,
            community_pool_addr,
            credit_purchase_weight,
            credit_retirement_weight,
            platform_facilitation_weight,
            governance_voting_weight,
            proposal_submission_weight,
            stability_annual_return_bps,
            max_stability_share_bps,
            min_commitment_amount,
            min_lock_months,
            max_lock_months,
            early_exit_penalty_bps,
            period_seconds,
        ),
    }
}

// ── Execute handlers ───────────────────────────────────────────────────

fn execute_initialize(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let mut state = MECHANISM_STATE.load(deps.storage)?;
    if state.status != MechanismStatus::Inactive {
        return Err(ContractError::AlreadyInitialized);
    }

    state.status = MechanismStatus::Tracking;
    state.tracking_start = Some(env.block.time);
    state.current_period = 1;
    MECHANISM_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "initialize_mechanism")
        .add_attribute("status", "Tracking")
        .add_attribute("period", "1"))
}

fn execute_activate(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let mut state = MECHANISM_STATE.load(deps.storage)?;
    if state.status != MechanismStatus::Tracking {
        return Err(ContractError::InvalidMechanismStatus {
            expected: "Tracking".to_string(),
            actual: state.status.to_string(),
        });
    }

    state.status = MechanismStatus::Distributing;
    MECHANISM_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "activate_distribution")
        .add_attribute("status", "Distributing"))
}

fn execute_commit_stability(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lock_months: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let state = MECHANISM_STATE.load(deps.storage)?;

    // Must be at least Tracking
    if state.status == MechanismStatus::Inactive {
        return Err(ContractError::InvalidMechanismStatus {
            expected: "Tracking or Distributing".to_string(),
            actual: state.status.to_string(),
        });
    }

    // Validate lock_months
    if lock_months < config.min_lock_months || lock_months > config.max_lock_months {
        return Err(ContractError::InvalidLockMonths {
            months: lock_months,
            min: config.min_lock_months,
            max: config.max_lock_months,
        });
    }

    // Validate attached funds
    let amount = extract_single_coin(&info, &config.denom)?;
    if amount < config.min_commitment_amount {
        return Err(ContractError::BelowMinCommitment {
            amount: amount.to_string(),
            min: config.min_commitment_amount.to_string(),
        });
    }

    let id = NEXT_COMMITMENT_ID.load(deps.storage)?;
    let committed_at = env.block.time;
    let matures_at =
        Timestamp::from_seconds(committed_at.seconds() + lock_months * SECONDS_PER_MONTH);

    let commitment = StabilityCommitment {
        id,
        holder: info.sender.clone(),
        amount,
        lock_months,
        committed_at,
        matures_at,
        accrued_rewards: Uint128::zero(),
        status: CommitmentStatus::Committed,
    };

    COMMITMENTS.save(deps.storage, id, &commitment)?;
    NEXT_COMMITMENT_ID.save(deps.storage, &(id + 1))?;

    // Maintain running total of committed principal
    let mut state = MECHANISM_STATE.load(deps.storage)?;
    state.total_committed_principal += amount;
    MECHANISM_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "commit_stability")
        .add_attribute("commitment_id", id.to_string())
        .add_attribute("holder", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("lock_months", lock_months.to_string())
        .add_attribute("matures_at", matures_at.seconds().to_string()))
}

fn execute_exit_early(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    commitment_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut commitment = COMMITMENTS
        .load(deps.storage, commitment_id)
        .map_err(|_| ContractError::CommitmentNotFound { id: commitment_id })?;

    // Must be the holder
    if commitment.holder != info.sender {
        return Err(ContractError::Unauthorized {
            reason: "only the commitment holder can exit".to_string(),
        });
    }

    // Must be in Committed status
    if commitment.status != CommitmentStatus::Committed {
        return Err(ContractError::InvalidCommitmentStatus {
            id: commitment_id,
            expected: "Committed".to_string(),
        });
    }

    // Calculate penalty: forfeit early_exit_penalty_bps of accrued rewards
    let penalty = commitment
        .accrued_rewards
        .multiply_ratio(config.early_exit_penalty_bps as u128, 10_000u128);
    let remaining_rewards = commitment.accrued_rewards - penalty;
    let total_return = commitment.amount + remaining_rewards;

    commitment.status = CommitmentStatus::EarlyExit;
    COMMITMENTS.save(deps.storage, commitment_id, &commitment)?;

    // Decrement running total of committed principal
    let mut state = MECHANISM_STATE.load(deps.storage)?;
    state.total_committed_principal = state
        .total_committed_principal
        .checked_sub(commitment.amount)
        .unwrap_or(Uint128::zero());
    MECHANISM_STATE.save(deps.storage, &state)?;

    let mut msgs = vec![];
    if !total_return.is_zero() {
        msgs.push(BankMsg::Send {
            to_address: commitment.holder.to_string(),
            amount: vec![Coin {
                denom: config.denom.clone(),
                amount: total_return,
            }],
        });
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "exit_stability_early")
        .add_attribute("commitment_id", commitment_id.to_string())
        .add_attribute("returned", total_return)
        .add_attribute("penalty", penalty)
        .add_attribute("forfeited_rewards", penalty))
}

fn execute_claim_matured(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    commitment_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut commitment = COMMITMENTS
        .load(deps.storage, commitment_id)
        .map_err(|_| ContractError::CommitmentNotFound { id: commitment_id })?;

    // Must be the holder
    if commitment.holder != info.sender {
        return Err(ContractError::Unauthorized {
            reason: "only the commitment holder can claim".to_string(),
        });
    }

    // Must be Committed (not already exited or claimed)
    if commitment.status != CommitmentStatus::Committed {
        return Err(ContractError::InvalidCommitmentStatus {
            id: commitment_id,
            expected: "Committed".to_string(),
        });
    }

    // Must have matured
    if env.block.time < commitment.matures_at {
        return Err(ContractError::CommitmentNotMatured {
            id: commitment_id,
            matures_at: commitment.matures_at.seconds().to_string(),
        });
    }

    let total_return = commitment.amount + commitment.accrued_rewards;
    commitment.status = CommitmentStatus::Matured;
    COMMITMENTS.save(deps.storage, commitment_id, &commitment)?;

    // Decrement running total of committed principal
    let mut state = MECHANISM_STATE.load(deps.storage)?;
    state.total_committed_principal = state
        .total_committed_principal
        .checked_sub(commitment.amount)
        .unwrap_or(Uint128::zero());
    MECHANISM_STATE.save(deps.storage, &state)?;

    let mut msgs = vec![];
    if !total_return.is_zero() {
        msgs.push(BankMsg::Send {
            to_address: commitment.holder.to_string(),
            amount: vec![Coin {
                denom: config.denom.clone(),
                amount: total_return,
            }],
        });
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "claim_matured_stability")
        .add_attribute("commitment_id", commitment_id.to_string())
        .add_attribute("returned_principal", commitment.amount)
        .add_attribute("returned_rewards", commitment.accrued_rewards)
        .add_attribute("total", total_return))
}

fn execute_record_activity(
    deps: DepsMut,
    info: MessageInfo,
    participant: String,
    credit_purchase_value: Uint128,
    credit_retirement_value: Uint128,
    platform_facilitation_value: Uint128,
    governance_votes: u32,
    proposal_credits: u32,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    let state = MECHANISM_STATE.load(deps.storage)?;
    if state.status == MechanismStatus::Inactive {
        return Err(ContractError::InvalidMechanismStatus {
            expected: "Tracking or Distributing".to_string(),
            actual: state.status.to_string(),
        });
    }

    let participant_addr = deps.api.addr_validate(&participant)?;
    let period = state.current_period;

    // Load existing or create new
    let mut score = ACTIVITY_SCORES
        .may_load(deps.storage, (period, &participant_addr))?
        .unwrap_or(ActivityScore {
            address: participant.clone(),
            period,
            ..Default::default()
        });

    // Accumulate (additive within a period)
    score.credit_purchase_value += credit_purchase_value;
    score.credit_retirement_value += credit_retirement_value;
    score.platform_facilitation_value += platform_facilitation_value;
    score.governance_votes += governance_votes;
    score.proposal_credits += proposal_credits;

    ACTIVITY_SCORES.save(deps.storage, (period, &participant_addr), &score)?;

    Ok(Response::new()
        .add_attribute("action", "record_activity")
        .add_attribute("participant", participant)
        .add_attribute("period", period.to_string()))
}

fn execute_trigger_distribution(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    community_pool_inflow: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    if community_pool_inflow.is_zero() {
        return Err(ContractError::ZeroInflow);
    }

    let mut state = MECHANISM_STATE.load(deps.storage)?;
    if state.status != MechanismStatus::Distributing {
        return Err(ContractError::InvalidMechanismStatus {
            expected: "Distributing".to_string(),
            actual: state.status.to_string(),
        });
    }

    let period = state.current_period;

    // Check not already distributed for this period
    if DISTRIBUTIONS.may_load(deps.storage, period)?.is_some() {
        return Err(ContractError::AlreadyDistributed { period });
    }

    // ── Step 1: Calculate stability allocation ──
    let stability_allocation = calculate_stability_allocation(deps.storage, &config, &env, community_pool_inflow)?;
    let activity_pool = community_pool_inflow - stability_allocation;

    // ── Step 2: Accrue stability rewards to commitments ──
    accrue_stability_rewards(deps.storage, stability_allocation)?;

    // ── Step 3: Calculate total weighted score for all participants ──
    let (total_score, participant_scores) =
        calculate_period_scores(deps.as_ref(), &config, period)?;

    // ── Step 4: Accumulate activity rewards into claimable balances (pull model) ──
    if !activity_pool.is_zero() && !total_score.is_zero() {
        for (addr_str, score) in &participant_scores {
            let reward = activity_pool.multiply_ratio(score.u128(), total_score.u128());
            if !reward.is_zero() {
                let addr = deps.api.addr_validate(addr_str)?;
                let existing = CLAIMABLE_REWARDS
                    .may_load(deps.storage, &addr)?
                    .unwrap_or(Uint128::zero());
                CLAIMABLE_REWARDS.save(deps.storage, &addr, &(existing + reward))?;
            }
        }
    }

    // ── Step 5: Record distribution ──
    let record = DistributionRecord {
        period,
        community_pool_inflow,
        stability_allocation,
        activity_pool,
        total_score,
        executed_at: env.block.time,
    };
    DISTRIBUTIONS.save(deps.storage, period, &record)?;

    // Advance period
    state.last_distribution_period = Some(period);
    state.current_period = period + 1;
    MECHANISM_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "trigger_distribution")
        .add_attribute("period", period.to_string())
        .add_attribute("community_pool_inflow", community_pool_inflow)
        .add_attribute("stability_allocation", stability_allocation)
        .add_attribute("activity_pool", activity_pool)
        .add_attribute("total_score", total_score)
        .add_attribute("participants", participant_scores.len().to_string()))
}

fn execute_claim_rewards(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let claimable = CLAIMABLE_REWARDS
        .may_load(deps.storage, &info.sender)?
        .unwrap_or(Uint128::zero());

    if claimable.is_zero() {
        return Err(ContractError::NoClaimableRewards {
            address: info.sender.to_string(),
        });
    }

    // Zero out the balance before sending
    CLAIMABLE_REWARDS.save(deps.storage, &info.sender, &Uint128::zero())?;

    let msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: config.denom.clone(),
            amount: claimable,
        }],
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "claim_rewards")
        .add_attribute("address", info.sender.to_string())
        .add_attribute("amount", claimable))
}

#[allow(clippy::too_many_arguments)]
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    community_pool_addr: Option<String>,
    credit_purchase_weight: Option<u64>,
    credit_retirement_weight: Option<u64>,
    platform_facilitation_weight: Option<u64>,
    governance_voting_weight: Option<u64>,
    proposal_submission_weight: Option<u64>,
    stability_annual_return_bps: Option<u64>,
    max_stability_share_bps: Option<u64>,
    min_commitment_amount: Option<Uint128>,
    min_lock_months: Option<u64>,
    max_lock_months: Option<u64>,
    early_exit_penalty_bps: Option<u64>,
    period_seconds: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    require_admin(&config, &info)?;

    if let Some(addr) = community_pool_addr {
        config.community_pool_addr = deps.api.addr_validate(&addr)?;
    }
    if let Some(v) = credit_purchase_weight {
        config.credit_purchase_weight = v;
    }
    if let Some(v) = credit_retirement_weight {
        config.credit_retirement_weight = v;
    }
    if let Some(v) = platform_facilitation_weight {
        config.platform_facilitation_weight = v;
    }
    if let Some(v) = governance_voting_weight {
        config.governance_voting_weight = v;
    }
    if let Some(v) = proposal_submission_weight {
        config.proposal_submission_weight = v;
    }
    if let Some(v) = stability_annual_return_bps {
        config.stability_annual_return_bps = v;
    }
    if let Some(v) = max_stability_share_bps {
        config.max_stability_share_bps = v;
    }
    if let Some(v) = min_commitment_amount {
        config.min_commitment_amount = v;
    }
    if let Some(v) = min_lock_months {
        config.min_lock_months = v;
    }
    if let Some(v) = max_lock_months {
        config.max_lock_months = v;
    }
    if let Some(v) = early_exit_penalty_bps {
        config.early_exit_penalty_bps = v;
    }
    if let Some(v) = period_seconds {
        config.period_seconds = v;
    }

    // Validate weights sum to 10_000
    let weight_sum = config.credit_purchase_weight
        + config.credit_retirement_weight
        + config.platform_facilitation_weight
        + config.governance_voting_weight
        + config.proposal_submission_weight;
    if weight_sum != 10_000 {
        return Err(ContractError::InvalidWeightSum { sum: weight_sum });
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// ── Internal helpers ───────────────────────────────────────────────────

fn require_admin(config: &Config, info: &MessageInfo) -> Result<(), ContractError> {
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "only admin can perform this action".to_string(),
        });
    }
    Ok(())
}

/// Extract a single coin of the expected denom from message funds
fn extract_single_coin(info: &MessageInfo, expected_denom: &str) -> Result<Uint128, ContractError> {
    if info.funds.is_empty() {
        return Err(ContractError::NoFundsAttached);
    }
    let coin = info
        .funds
        .iter()
        .find(|c| c.denom == expected_denom)
        .ok_or_else(|| ContractError::WrongDenom {
            expected: expected_denom.to_string(),
            got: info.funds[0].denom.clone(),
        })?;
    Ok(coin.amount)
}

/// Calculate how much of the inflow goes to stability tier.
/// Pro-rate 6% annual across all active commitments, capped at max_stability_share_bps of inflow.
/// Uses the `total_committed_principal` counter instead of scanning all commitments.
fn calculate_stability_allocation(
    storage: &dyn cosmwasm_std::Storage,
    config: &Config,
    _env: &Env,
    inflow: Uint128,
) -> Result<Uint128, ContractError> {
    let state = MECHANISM_STATE.load(storage)?;
    let total_committed = state.total_committed_principal;

    if total_committed.is_zero() {
        return Ok(Uint128::zero());
    }

    // Annual return pro-rated to one period: (annual_bps / 10_000) * (period_seconds / seconds_per_year)
    // stability_owed = total_committed * annual_bps * period_seconds / (10_000 * 31_556_926)
    let seconds_per_year: u128 = 31_556_926;
    let stability_owed = total_committed
        .multiply_ratio(
            config.stability_annual_return_bps as u128 * config.period_seconds as u128,
            10_000u128 * seconds_per_year,
        );

    // Cap at max_stability_share_bps of inflow
    let max_stability = inflow.multiply_ratio(config.max_stability_share_bps as u128, 10_000u128);
    let allocation = std::cmp::min(stability_owed, max_stability);

    Ok(allocation)
}

/// Distribute stability_allocation pro-rata across all active commitments
fn accrue_stability_rewards(
    storage: &mut dyn cosmwasm_std::Storage,
    stability_allocation: Uint128,
) -> Result<(), ContractError> {
    if stability_allocation.is_zero() {
        return Ok(());
    }

    // Collect all active commitments and their amounts
    let active: Vec<(u64, Uint128)> = COMMITMENTS
        .range(storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .filter(|(_, c)| c.status == CommitmentStatus::Committed)
        .map(|(id, c)| (id, c.amount))
        .collect();

    let total: Uint128 = active.iter().map(|(_, a)| *a).fold(Uint128::zero(), |acc, a| acc + a);
    if total.is_zero() {
        return Ok(());
    }

    for (id, amount) in &active {
        let share = stability_allocation.multiply_ratio(amount.u128(), total.u128());
        let mut commitment = COMMITMENTS.load(storage, *id)?;
        commitment.accrued_rewards += share;
        COMMITMENTS.save(storage, *id, &commitment)?;
    }

    Ok(())
}

/// Calculate weighted scores for all participants in a period.
/// Returns (total_score, vec of (addr, individual_score)).
fn calculate_period_scores(
    deps: Deps,
    config: &Config,
    period: u32,
) -> Result<(Uint128, Vec<(String, Uint128)>), ContractError> {
    let mut total = Uint128::zero();
    let mut participants: Vec<(String, Uint128)> = vec![];

    // Find the max values for normalization of governance fields
    let mut max_votes: u32 = 0;
    let mut max_proposals: u32 = 0;
    let scores: Vec<ActivityScore> = ACTIVITY_SCORES
        .prefix(period)
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(_, s)| {
            if s.governance_votes > max_votes {
                max_votes = s.governance_votes;
            }
            if s.proposal_credits > max_proposals {
                max_proposals = s.proposal_credits;
            }
            s
        })
        .collect();

    for score in &scores {
        let weighted = calculate_weighted_score(config, score, max_votes, max_proposals);
        total += weighted;
        participants.push((score.address.clone(), weighted));
    }

    Ok((total, participants))
}

/// Calculate a single participant's weighted score.
/// Monetary fields are used directly (micro-denom units).
/// Count fields (votes, proposals) are normalized against the period max, then scaled to a reference value.
fn calculate_weighted_score(
    config: &Config,
    score: &ActivityScore,
    max_votes: u32,
    max_proposals: u32,
) -> Uint128 {
    // Monetary components: value * weight / 10_000
    let purchase = score
        .credit_purchase_value
        .multiply_ratio(config.credit_purchase_weight as u128, 10_000u128);
    let retirement = score
        .credit_retirement_value
        .multiply_ratio(config.credit_retirement_weight as u128, 10_000u128);
    let facilitation = score
        .platform_facilitation_value
        .multiply_ratio(config.platform_facilitation_weight as u128, 10_000u128);

    // Count components: normalize to 0-1_000_000 range, then apply weight
    let reference_scale: u128 = 1_000_000;

    let vote_normalized = if max_votes > 0 {
        Uint128::new(score.governance_votes as u128)
            .multiply_ratio(reference_scale, max_votes as u128)
    } else {
        Uint128::zero()
    };
    let vote_weighted =
        vote_normalized.multiply_ratio(config.governance_voting_weight as u128, 10_000u128);

    // proposal_credits is scaled by 100 (100 = 1.0)
    let proposal_normalized = if max_proposals > 0 {
        Uint128::new(score.proposal_credits as u128)
            .multiply_ratio(reference_scale, max_proposals as u128)
    } else {
        Uint128::zero()
    };
    let proposal_weighted =
        proposal_normalized.multiply_ratio(config.proposal_submission_weight as u128, 10_000u128);

    purchase + retirement + facilitation + vote_weighted + proposal_weighted
}

// ── Query ──────────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::MechanismState {} => to_json_binary(&query_mechanism_state(deps)?),
        QueryMsg::ActivityScore { address, period } => {
            to_json_binary(&query_activity_score(deps, address, period)?)
        }
        QueryMsg::StabilityCommitment { commitment_id } => {
            to_json_binary(&query_stability_commitment(deps, commitment_id)?)
        }
        QueryMsg::DistributionRecord { period } => {
            to_json_binary(&query_distribution_record(deps, period)?)
        }
        QueryMsg::ParticipantRewards {
            address,
            period_from,
            period_to,
        } => to_json_binary(&query_participant_rewards(
            deps,
            address,
            period_from,
            period_to,
        )?),
        QueryMsg::ClaimableRewards { address } => {
            to_json_binary(&query_claimable_rewards(deps, address)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.to_string(),
        community_pool_addr: config.community_pool_addr.to_string(),
        denom: config.denom,
        credit_purchase_weight: config.credit_purchase_weight,
        credit_retirement_weight: config.credit_retirement_weight,
        platform_facilitation_weight: config.platform_facilitation_weight,
        governance_voting_weight: config.governance_voting_weight,
        proposal_submission_weight: config.proposal_submission_weight,
        stability_annual_return_bps: config.stability_annual_return_bps,
        max_stability_share_bps: config.max_stability_share_bps,
        min_commitment_amount: config.min_commitment_amount,
        min_lock_months: config.min_lock_months,
        max_lock_months: config.max_lock_months,
        early_exit_penalty_bps: config.early_exit_penalty_bps,
        period_seconds: config.period_seconds,
    })
}

fn query_mechanism_state(deps: Deps) -> StdResult<MechanismStateResponse> {
    let state = MECHANISM_STATE.load(deps.storage)?;
    Ok(MechanismStateResponse {
        status: state.status.to_string(),
        tracking_start: state.tracking_start,
        current_period: state.current_period,
        last_distribution_period: state.last_distribution_period,
    })
}

fn query_activity_score(deps: Deps, address: String, period: u32) -> StdResult<ActivityScoreResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let score = ACTIVITY_SCORES
        .may_load(deps.storage, (period, &addr))?
        .unwrap_or(ActivityScore {
            address: address.clone(),
            period,
            ..Default::default()
        });
    Ok(ActivityScoreResponse { score })
}

fn query_stability_commitment(deps: Deps, commitment_id: u64) -> StdResult<StabilityCommitmentResponse> {
    let commitment = COMMITMENTS.load(deps.storage, commitment_id)?;
    Ok(StabilityCommitmentResponse { commitment })
}

fn query_distribution_record(deps: Deps, period: u32) -> StdResult<DistributionRecordResponse> {
    let record = DISTRIBUTIONS.load(deps.storage, period)?;
    Ok(DistributionRecordResponse { record })
}

fn query_participant_rewards(
    deps: Deps,
    address: String,
    period_from: u32,
    period_to: u32,
) -> StdResult<ParticipantRewardsResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let mut total_activity_rewards = Uint128::zero();
    let mut active_periods = 0u32;

    for period in period_from..=period_to {
        let score_opt = ACTIVITY_SCORES.may_load(deps.storage, (period, &addr))?;
        let dist_opt = DISTRIBUTIONS.may_load(deps.storage, period)?;

        if let (Some(score), Some(dist)) = (score_opt, dist_opt) {
            if !dist.total_score.is_zero() && !dist.activity_pool.is_zero() {
                let config = CONFIG.load(deps.storage)?;

                // Recalculate this participant's weighted score for the period
                // We need the max values from all participants
                let mut max_votes: u32 = 0;
                let mut max_proposals: u32 = 0;
                let all_scores: Vec<ActivityScore> = ACTIVITY_SCORES
                    .prefix(period)
                    .range(deps.storage, None, None, Order::Ascending)
                    .filter_map(|r| r.ok())
                    .map(|(_, s)| {
                        if s.governance_votes > max_votes {
                            max_votes = s.governance_votes;
                        }
                        if s.proposal_credits > max_proposals {
                            max_proposals = s.proposal_credits;
                        }
                        s
                    })
                    .collect();

                // Find this participant's score in all_scores
                let _participant_score_entry = all_scores.iter().find(|s| s.address == address);

                let weighted = calculate_weighted_score(&config, &score, max_votes, max_proposals);
                let reward = dist.activity_pool.multiply_ratio(weighted.u128(), dist.total_score.u128());
                total_activity_rewards += reward;
                active_periods += 1;
            }
        }
    }

    Ok(ParticipantRewardsResponse {
        address,
        period_from,
        period_to,
        total_activity_rewards,
        active_periods,
    })
}

fn query_claimable_rewards(deps: Deps, address: String) -> StdResult<ClaimableRewardsResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let amount = CLAIMABLE_REWARDS
        .may_load(deps.storage, &addr)?
        .unwrap_or(Uint128::zero());
    Ok(ClaimableRewardsResponse { address, amount })
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
            community_pool_addr: addr("community_pool").to_string(),
            denom: DENOM.to_string(),
        };
        instantiate(deps, mock_env(), info.clone(), msg).unwrap();
        info
    }

    fn initialize_mechanism(deps: DepsMut, admin: &Addr) {
        let info = message_info(admin, &[]);
        execute(deps, mock_env(), info, ExecuteMsg::InitializeMechanism {}).unwrap();
    }

    fn activate_distribution(deps: DepsMut, admin: &Addr) {
        let info = message_info(admin, &[]);
        execute(deps, mock_env(), info, ExecuteMsg::ActivateDistribution {}).unwrap();
    }

    // ── Test 1: Instantiate + Initialize ──

    #[test]
    fn test_instantiate_and_initialize() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();

        // Query config — verify defaults
        let config: ConfigResponse =
            cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap())
                .unwrap();
        assert_eq!(config.admin, admin.to_string());
        assert_eq!(config.denom, DENOM);
        assert_eq!(config.credit_purchase_weight, 3000);
        assert_eq!(config.credit_retirement_weight, 3000);
        assert_eq!(config.platform_facilitation_weight, 2000);
        assert_eq!(config.governance_voting_weight, 1000);
        assert_eq!(config.proposal_submission_weight, 1000);
        assert_eq!(config.stability_annual_return_bps, 600);
        assert_eq!(config.max_stability_share_bps, 3000);
        assert_eq!(config.min_commitment_amount, Uint128::new(100_000_000));
        assert_eq!(config.min_lock_months, 6);
        assert_eq!(config.max_lock_months, 24);
        assert_eq!(config.early_exit_penalty_bps, 5000);
        assert_eq!(config.period_seconds, 604_800);

        // Query state — should be Inactive
        let state: MechanismStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::MechanismState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state.status, "Inactive");
        assert_eq!(state.current_period, 0);

        // Initialize
        initialize_mechanism(deps.as_mut(), &admin);

        // Query state — should be Tracking, period 1
        let state: MechanismStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::MechanismState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state.status, "Tracking");
        assert_eq!(state.current_period, 1);
        assert!(state.tracking_start.is_some());

        // Double init should fail
        let info = message_info(&admin, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::InitializeMechanism {},
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::AlreadyInitialized));
    }

    // ── Test 2: Commit stability ──

    #[test]
    fn test_commit_stability() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();
        initialize_mechanism(deps.as_mut(), &admin);

        let holder = addr("holder1");
        let commit_amount = Uint128::new(200_000_000); // 200 REGEN

        // Commit for 12 months
        let info = message_info(&holder, &[Coin::new(commit_amount.u128(), DENOM)]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CommitStability { lock_months: 12 },
        )
        .unwrap();
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "commitment_id")
                .unwrap()
                .value,
            "1"
        );
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "lock_months")
                .unwrap()
                .value,
            "12"
        );

        // Query commitment
        let commitment: StabilityCommitmentResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StabilityCommitment { commitment_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(commitment.commitment.holder, holder);
        assert_eq!(commitment.commitment.amount, commit_amount);
        assert_eq!(commitment.commitment.lock_months, 12);
        assert_eq!(commitment.commitment.accrued_rewards, Uint128::zero());
        assert_eq!(commitment.commitment.status, CommitmentStatus::Committed);

        // Maturity should be ~12 months later
        let expected_maturity = mock_env().block.time.seconds() + 12 * SECONDS_PER_MONTH;
        assert_eq!(commitment.commitment.matures_at.seconds(), expected_maturity);

        // Below minimum fails
        let small_holder = addr("small_holder");
        let info = message_info(&small_holder, &[Coin::new(50_000_000u128, DENOM)]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CommitStability { lock_months: 6 },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::BelowMinCommitment { .. }));

        // Invalid lock months (too short)
        let info = message_info(&holder, &[Coin::new(200_000_000u128, DENOM)]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CommitStability { lock_months: 3 },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::InvalidLockMonths { .. }));

        // Invalid lock months (too long)
        let info = message_info(&holder, &[Coin::new(200_000_000u128, DENOM)]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CommitStability { lock_months: 36 },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::InvalidLockMonths { .. }));

        // No funds fails
        let info = message_info(&holder, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CommitStability { lock_months: 12 },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::NoFundsAttached));
    }

    // ── Test 3: Record activity + trigger distribution ──

    #[test]
    fn test_record_activity_and_distribute() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();
        initialize_mechanism(deps.as_mut(), &admin);
        activate_distribution(deps.as_mut(), &admin);

        let alice = addr("alice");
        let bob = addr("bob");

        // Record activity for alice
        let info = message_info(&admin, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::RecordActivity {
                participant: alice.to_string(),
                credit_purchase_value: Uint128::new(1_000_000),
                credit_retirement_value: Uint128::new(500_000),
                platform_facilitation_value: Uint128::new(200_000),
                governance_votes: 5,
                proposal_credits: 100, // 1.0
            },
        )
        .unwrap();

        // Record activity for bob
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::RecordActivity {
                participant: bob.to_string(),
                credit_purchase_value: Uint128::new(500_000),
                credit_retirement_value: Uint128::new(250_000),
                platform_facilitation_value: Uint128::new(100_000),
                governance_votes: 3,
                proposal_credits: 50, // 0.5
            },
        )
        .unwrap();

        // Query alice's activity
        let score: ActivityScoreResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ActivityScore {
                    address: alice.to_string(),
                    period: 1,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(score.score.credit_purchase_value, Uint128::new(1_000_000));
        assert_eq!(score.score.governance_votes, 5);

        // Trigger distribution with 10_000_000 inflow (no stability commitments, so all goes to activity)
        let inflow = Uint128::new(10_000_000);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::TriggerDistribution {
                community_pool_inflow: inflow,
            },
        )
        .unwrap();

        // Stability allocation should be 0 (no commitments)
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "stability_allocation")
                .unwrap()
                .value,
            "0"
        );
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "activity_pool")
                .unwrap()
                .value,
            "10000000"
        );

        // Pull model: distribution should NOT send bank messages
        assert_eq!(res.messages.len(), 0);

        // Both participants should have claimable rewards
        let alice_claimable: ClaimableRewardsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ClaimableRewards {
                    address: alice.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(
            alice_claimable.amount > Uint128::zero(),
            "alice should have claimable rewards"
        );

        let bob_claimable: ClaimableRewardsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ClaimableRewards {
                    address: bob.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(
            bob_claimable.amount > Uint128::zero(),
            "bob should have claimable rewards"
        );

        // Total claimable should equal the full inflow (no stability commitments)
        // Allow 1 unit of rounding error from pro-rata integer division
        let total_claimable = alice_claimable.amount + bob_claimable.amount;
        assert!(
            total_claimable.u128().abs_diff(inflow.u128()) <= 1,
            "total claimable {} should approximately equal inflow {}",
            total_claimable,
            inflow
        );

        // Alice should get more than Bob (higher activity scores)
        assert!(
            alice_claimable.amount > bob_claimable.amount,
            "alice should get more than bob"
        );

        // Period should advance to 2
        let state: MechanismStateResponse = cosmwasm_std::from_json(
            query(deps.as_ref(), mock_env(), QueryMsg::MechanismState {}).unwrap(),
        )
        .unwrap();
        assert_eq!(state.current_period, 2);
        assert_eq!(state.last_distribution_period, Some(1));

        // Distribution record should exist
        let dist: DistributionRecordResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::DistributionRecord { period: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(dist.record.community_pool_inflow, inflow);
        assert_eq!(dist.record.stability_allocation, Uint128::zero());
        assert_eq!(dist.record.activity_pool, inflow);

        // Alice claims her rewards
        let alice_info = message_info(&alice, &[]);
        let claim_res = execute(
            deps.as_mut(),
            mock_env(),
            alice_info,
            ExecuteMsg::ClaimRewards {},
        )
        .unwrap();
        assert_eq!(claim_res.messages.len(), 1); // one BankMsg::Send
        assert_eq!(
            claim_res
                .attributes
                .iter()
                .find(|a| a.key == "amount")
                .unwrap()
                .value,
            alice_claimable.amount.to_string()
        );

        // After claiming, alice's claimable should be zero
        let alice_after: ClaimableRewardsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ClaimableRewards {
                    address: alice.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(alice_after.amount, Uint128::zero());

        // Claiming again should fail
        let alice_info2 = message_info(&alice, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            alice_info2,
            ExecuteMsg::ClaimRewards {},
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::NoClaimableRewards { .. }));
    }

    // ── Test 4: Early exit penalty ──

    #[test]
    fn test_early_exit_penalty() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();
        initialize_mechanism(deps.as_mut(), &admin);
        activate_distribution(deps.as_mut(), &admin);

        let holder = addr("holder1");
        let commit_amount = Uint128::new(500_000_000); // 500 REGEN

        // Commit
        let info = message_info(&holder, &[Coin::new(commit_amount.u128(), DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CommitStability { lock_months: 12 },
        )
        .unwrap();

        // Record some activity so distribution has participants
        let admin_info_no_funds = message_info(&admin, &[]);
        let participant = addr("participant1");
        execute(
            deps.as_mut(),
            mock_env(),
            admin_info_no_funds.clone(),
            ExecuteMsg::RecordActivity {
                participant: participant.to_string(),
                credit_purchase_value: Uint128::new(1_000_000),
                credit_retirement_value: Uint128::zero(),
                platform_facilitation_value: Uint128::zero(),
                governance_votes: 0,
                proposal_credits: 0,
            },
        )
        .unwrap();

        // Trigger distribution to accrue some stability rewards
        execute(
            deps.as_mut(),
            mock_env(),
            admin_info_no_funds.clone(),
            ExecuteMsg::TriggerDistribution {
                community_pool_inflow: Uint128::new(50_000_000),
            },
        )
        .unwrap();

        // Check accrued rewards
        let commitment: StabilityCommitmentResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StabilityCommitment { commitment_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        let accrued = commitment.commitment.accrued_rewards;
        assert!(accrued > Uint128::zero(), "should have accrued some rewards");

        // Early exit
        let holder_info = message_info(&holder, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            holder_info,
            ExecuteMsg::ExitStabilityEarly { commitment_id: 1 },
        )
        .unwrap();

        // Penalty = 50% of accrued
        let expected_penalty = accrued.multiply_ratio(5000u128, 10_000u128);
        let expected_return = commit_amount + accrued - expected_penalty;

        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "penalty")
                .unwrap()
                .value,
            expected_penalty.to_string()
        );
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "returned")
                .unwrap()
                .value,
            expected_return.to_string()
        );

        // Should have bank message returning funds
        assert_eq!(res.messages.len(), 1);

        // Commitment should be EarlyExit
        let commitment: StabilityCommitmentResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StabilityCommitment { commitment_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(commitment.commitment.status, CommitmentStatus::EarlyExit);

        // Can't exit again
        let holder_info2 = message_info(&holder, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            holder_info2,
            ExecuteMsg::ExitStabilityEarly { commitment_id: 1 },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::InvalidCommitmentStatus { .. }));
    }

    // ── Test 5: Matured claim ──

    #[test]
    fn test_matured_claim() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();
        initialize_mechanism(deps.as_mut(), &admin);
        activate_distribution(deps.as_mut(), &admin);

        let holder = addr("holder1");
        let commit_amount = Uint128::new(300_000_000); // 300 REGEN

        // Commit for 6 months (minimum)
        let info = message_info(&holder, &[Coin::new(commit_amount.u128(), DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CommitStability { lock_months: 6 },
        )
        .unwrap();

        // Record activity + distribute to accrue rewards
        let admin_info_nf = message_info(&admin, &[]);
        let participant = addr("p1");
        execute(
            deps.as_mut(),
            mock_env(),
            admin_info_nf.clone(),
            ExecuteMsg::RecordActivity {
                participant: participant.to_string(),
                credit_purchase_value: Uint128::new(1_000_000),
                credit_retirement_value: Uint128::zero(),
                platform_facilitation_value: Uint128::zero(),
                governance_votes: 0,
                proposal_credits: 0,
            },
        )
        .unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            admin_info_nf.clone(),
            ExecuteMsg::TriggerDistribution {
                community_pool_inflow: Uint128::new(20_000_000),
            },
        )
        .unwrap();

        let commitment: StabilityCommitmentResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StabilityCommitment { commitment_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        let accrued = commitment.commitment.accrued_rewards;

        // Try to claim before maturity — should fail
        let holder_info = message_info(&holder, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            holder_info.clone(),
            ExecuteMsg::ClaimMaturedStability { commitment_id: 1 },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::CommitmentNotMatured { .. }));

        // Fast-forward time past maturity (6 months + 1 day)
        let mut future_env = mock_env();
        future_env.block.time = Timestamp::from_seconds(
            mock_env().block.time.seconds() + 6 * SECONDS_PER_MONTH + 86_400,
        );

        // Claim matured
        let res = execute(
            deps.as_mut(),
            future_env.clone(),
            holder_info,
            ExecuteMsg::ClaimMaturedStability { commitment_id: 1 },
        )
        .unwrap();

        let expected_total = commit_amount + accrued;
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "total")
                .unwrap()
                .value,
            expected_total.to_string()
        );
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "returned_principal")
                .unwrap()
                .value,
            commit_amount.to_string()
        );
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "returned_rewards")
                .unwrap()
                .value,
            accrued.to_string()
        );

        // Should be Matured
        let commitment: StabilityCommitmentResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                future_env,
                QueryMsg::StabilityCommitment { commitment_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(commitment.commitment.status, CommitmentStatus::Matured);
    }

    // ── Test 6: Unauthorized checks ──

    #[test]
    fn test_unauthorized_operations() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let stranger = addr("stranger");

        // Non-admin cannot initialize
        let info = message_info(&stranger, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::InitializeMechanism {},
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));

        // Non-admin cannot record activity
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::RecordActivity {
                participant: stranger.to_string(),
                credit_purchase_value: Uint128::new(100),
                credit_retirement_value: Uint128::zero(),
                platform_facilitation_value: Uint128::zero(),
                governance_votes: 0,
                proposal_credits: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));

        // Non-admin cannot trigger distribution
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::TriggerDistribution {
                community_pool_inflow: Uint128::new(1_000_000),
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));
    }

    // ── Test 7: Distribution with stability + activity ──

    #[test]
    fn test_distribution_with_stability_and_activity() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();
        initialize_mechanism(deps.as_mut(), &admin);
        activate_distribution(deps.as_mut(), &admin);

        // Two stability commitments
        let holder_a = addr("holder_a");
        let holder_b = addr("holder_b");

        let info_a = message_info(&holder_a, &[Coin::new(400_000_000u128, DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            info_a,
            ExecuteMsg::CommitStability { lock_months: 12 },
        )
        .unwrap();

        let info_b = message_info(&holder_b, &[Coin::new(600_000_000u128, DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            info_b,
            ExecuteMsg::CommitStability { lock_months: 24 },
        )
        .unwrap();

        // Activity participant
        let alice = addr("alice");
        let admin_nf = message_info(&admin, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            admin_nf.clone(),
            ExecuteMsg::RecordActivity {
                participant: alice.to_string(),
                credit_purchase_value: Uint128::new(2_000_000),
                credit_retirement_value: Uint128::new(1_000_000),
                platform_facilitation_value: Uint128::zero(),
                governance_votes: 0,
                proposal_credits: 0,
            },
        )
        .unwrap();

        // Trigger distribution with 100M inflow
        let inflow = Uint128::new(100_000_000);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            admin_nf,
            ExecuteMsg::TriggerDistribution {
                community_pool_inflow: inflow,
            },
        )
        .unwrap();

        // Stability allocation should be > 0
        let stability_alloc: u128 = res
            .attributes
            .iter()
            .find(|a| a.key == "stability_allocation")
            .unwrap()
            .value
            .parse()
            .unwrap();
        assert!(stability_alloc > 0, "stability allocation should be positive");

        // Activity pool = inflow - stability
        let activity_pool: u128 = res
            .attributes
            .iter()
            .find(|a| a.key == "activity_pool")
            .unwrap()
            .value
            .parse()
            .unwrap();
        assert_eq!(activity_pool, inflow.u128() - stability_alloc);

        // Check that holder_a got 40% and holder_b got 60% of stability rewards
        let c_a: StabilityCommitmentResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StabilityCommitment { commitment_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        let c_b: StabilityCommitmentResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::StabilityCommitment { commitment_id: 2 },
            )
            .unwrap(),
        )
        .unwrap();

        // holder_b committed 1.5x holder_a, so should get proportionally more
        assert!(
            c_b.commitment.accrued_rewards > c_a.commitment.accrued_rewards,
            "holder_b (600M) should accrue more than holder_a (400M)"
        );
        // Total accrued should approximately equal stability_alloc
        let total_accrued = c_a.commitment.accrued_rewards + c_b.commitment.accrued_rewards;
        // Allow 1 unit of rounding error
        assert!(
            total_accrued.u128().abs_diff(stability_alloc) <= 1,
            "total accrued {} should match stability allocation {}",
            total_accrued,
            stability_alloc
        );
    }

    // ── Test 8: Update config weight validation ──

    #[test]
    fn test_update_config_weight_validation() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();

        // Valid update (weights still sum to 10_000)
        let info = message_info(&admin, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::UpdateConfig {
                community_pool_addr: None,
                credit_purchase_weight: Some(4000),
                credit_retirement_weight: Some(2000),
                platform_facilitation_weight: Some(2000),
                governance_voting_weight: Some(1000),
                proposal_submission_weight: Some(1000),
                stability_annual_return_bps: None,
                max_stability_share_bps: None,
                min_commitment_amount: None,
                min_lock_months: None,
                max_lock_months: None,
                early_exit_penalty_bps: None,
                period_seconds: None,
            },
        );
        assert!(res.is_ok());

        // Invalid update (weights don't sum to 10_000)
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateConfig {
                community_pool_addr: None,
                credit_purchase_weight: Some(5000),
                credit_retirement_weight: None, // stays 2000 from previous update
                platform_facilitation_weight: None,
                governance_voting_weight: None,
                proposal_submission_weight: None,
                stability_annual_return_bps: None,
                max_stability_share_bps: None,
                min_commitment_amount: None,
                min_lock_months: None,
                max_lock_months: None,
                early_exit_penalty_bps: None,
                period_seconds: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::InvalidWeightSum { .. }));
    }

    // ── Test 9: Total committed principal counter ──

    #[test]
    fn test_total_committed_principal_counter() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();
        initialize_mechanism(deps.as_mut(), &admin);
        activate_distribution(deps.as_mut(), &admin);

        // Initially zero — read internal state directly since the query response
        // doesn't expose total_committed_principal
        let internal = MECHANISM_STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(internal.total_committed_principal, Uint128::zero());

        // Commit 400 REGEN
        let holder_a = addr("holder_a");
        let info_a = message_info(&holder_a, &[Coin::new(400_000_000u128, DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            info_a,
            ExecuteMsg::CommitStability { lock_months: 12 },
        )
        .unwrap();

        let internal = MECHANISM_STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            internal.total_committed_principal,
            Uint128::new(400_000_000)
        );

        // Commit 600 REGEN
        let holder_b = addr("holder_b");
        let info_b = message_info(&holder_b, &[Coin::new(600_000_000u128, DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            info_b,
            ExecuteMsg::CommitStability { lock_months: 6 },
        )
        .unwrap();

        let internal = MECHANISM_STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            internal.total_committed_principal,
            Uint128::new(1_000_000_000)
        );

        // Early exit commitment 1 (400M) — counter decrements
        let exit_info = message_info(&holder_a, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            exit_info,
            ExecuteMsg::ExitStabilityEarly { commitment_id: 1 },
        )
        .unwrap();

        let internal = MECHANISM_STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            internal.total_committed_principal,
            Uint128::new(600_000_000)
        );

        // Need activity participant for distribution to work
        let p = addr("participant");
        let admin_nf = message_info(&admin, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            admin_nf.clone(),
            ExecuteMsg::RecordActivity {
                participant: p.to_string(),
                credit_purchase_value: Uint128::new(1_000_000),
                credit_retirement_value: Uint128::zero(),
                platform_facilitation_value: Uint128::zero(),
                governance_votes: 0,
                proposal_credits: 0,
            },
        )
        .unwrap();

        // Distribute so commitment 2 accrues some rewards
        execute(
            deps.as_mut(),
            mock_env(),
            admin_nf,
            ExecuteMsg::TriggerDistribution {
                community_pool_inflow: Uint128::new(10_000_000),
            },
        )
        .unwrap();

        // Claim matured commitment 2 (fast-forward time)
        let mut future_env = mock_env();
        future_env.block.time = Timestamp::from_seconds(
            mock_env().block.time.seconds() + 6 * SECONDS_PER_MONTH + 86_400,
        );
        let claim_info = message_info(&holder_b, &[]);
        execute(
            deps.as_mut(),
            future_env,
            claim_info,
            ExecuteMsg::ClaimMaturedStability { commitment_id: 2 },
        )
        .unwrap();

        let internal = MECHANISM_STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            internal.total_committed_principal,
            Uint128::zero(),
            "all commitments exited/claimed, counter should be zero"
        );
    }

    // ── Test 10: Pull model claim rewards accumulation ──

    #[test]
    fn test_claim_rewards_accumulates_across_periods() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let admin = admin_info.sender.clone();
        initialize_mechanism(deps.as_mut(), &admin);
        activate_distribution(deps.as_mut(), &admin);

        let alice = addr("alice");
        let admin_nf = message_info(&admin, &[]);

        // Period 1: record activity + distribute
        execute(
            deps.as_mut(),
            mock_env(),
            admin_nf.clone(),
            ExecuteMsg::RecordActivity {
                participant: alice.to_string(),
                credit_purchase_value: Uint128::new(1_000_000),
                credit_retirement_value: Uint128::zero(),
                platform_facilitation_value: Uint128::zero(),
                governance_votes: 0,
                proposal_credits: 0,
            },
        )
        .unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            admin_nf.clone(),
            ExecuteMsg::TriggerDistribution {
                community_pool_inflow: Uint128::new(5_000_000),
            },
        )
        .unwrap();

        let after_p1: ClaimableRewardsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ClaimableRewards {
                    address: alice.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        let p1_amount = after_p1.amount;
        assert!(p1_amount > Uint128::zero());

        // Period 2: record activity + distribute (rewards should accumulate)
        execute(
            deps.as_mut(),
            mock_env(),
            admin_nf.clone(),
            ExecuteMsg::RecordActivity {
                participant: alice.to_string(),
                credit_purchase_value: Uint128::new(2_000_000),
                credit_retirement_value: Uint128::zero(),
                platform_facilitation_value: Uint128::zero(),
                governance_votes: 0,
                proposal_credits: 0,
            },
        )
        .unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            admin_nf,
            ExecuteMsg::TriggerDistribution {
                community_pool_inflow: Uint128::new(8_000_000),
            },
        )
        .unwrap();

        let after_p2: ClaimableRewardsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ClaimableRewards {
                    address: alice.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        // Should have accumulated rewards from both periods
        assert!(
            after_p2.amount > p1_amount,
            "claimable should accumulate: {} > {}",
            after_p2.amount,
            p1_amount
        );

        // Claim all accumulated rewards
        let alice_info = message_info(&alice, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            alice_info,
            ExecuteMsg::ClaimRewards {},
        )
        .unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "amount")
                .unwrap()
                .value,
            after_p2.amount.to_string()
        );

        // Balance should be zero after claim
        let after_claim: ClaimableRewardsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ClaimableRewards {
                    address: alice.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(after_claim.amount, Uint128::zero());

        // Claiming with nothing available should fail
        let alice_info2 = message_info(&alice, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            alice_info2,
            ExecuteMsg::ClaimRewards {},
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::NoClaimableRewards { .. }));
    }
}
