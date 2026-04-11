use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Timestamp, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)?;
    let config = Config {
        admin,
        activation_delay_seconds: msg.activation_delay_seconds.unwrap_or(86_400),
        challenge_window_seconds: msg.challenge_window_seconds.unwrap_or(15_552_000),
        resolution_deadline_seconds: msg.resolution_deadline_seconds.unwrap_or(1_209_600),
        challenge_bond_denom: msg.challenge_bond_denom.unwrap_or_else(|| "uregen".to_string()),
        challenge_bond_amount: msg.challenge_bond_amount.unwrap_or(Uint128::zero()),
        decay_half_life_seconds: msg.decay_half_life_seconds.unwrap_or(1_209_600),
        default_min_stake: msg.default_min_stake.unwrap_or(Uint128::zero()),
        arbiters: vec![],
    };

    CONFIG.save(deps.storage, &config)?;
    NEXT_SIGNAL_ID.save(deps.storage, &1u64)?;
    NEXT_CHALLENGE_ID.save(deps.storage, &1u64)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", config.admin.as_str()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SubmitSignal {
            subject_type,
            subject_id,
            category,
            endorsement_level,
            evidence,
        } => exec_submit_signal(
            deps,
            env,
            info,
            subject_type,
            subject_id,
            category,
            endorsement_level,
            evidence,
        ),
        ExecuteMsg::ActivateSignal { signal_id } => exec_activate_signal(deps, env, signal_id),
        ExecuteMsg::WithdrawSignal { signal_id } => {
            exec_withdraw_signal(deps, env, info, signal_id)
        }
        ExecuteMsg::SubmitChallenge {
            signal_id,
            rationale,
            evidence,
        } => exec_submit_challenge(deps, env, info, signal_id, rationale, evidence),
        ExecuteMsg::ResolveChallenge {
            challenge_id,
            outcome_valid,
            rationale,
        } => exec_resolve_challenge(deps, env, info, challenge_id, outcome_valid, rationale),
        ExecuteMsg::EscalateChallenge { challenge_id } => {
            exec_escalate_challenge(deps, env, challenge_id)
        }
        ExecuteMsg::InvalidateSignal {
            signal_id,
            rationale,
        } => exec_invalidate_signal(deps, info, signal_id, rationale),
        ExecuteMsg::UpdateConfig {
            activation_delay_seconds,
            challenge_window_seconds,
            resolution_deadline_seconds,
            challenge_bond_denom,
            challenge_bond_amount,
            decay_half_life_seconds,
            default_min_stake,
        } => exec_update_config(
            deps,
            info,
            activation_delay_seconds,
            challenge_window_seconds,
            resolution_deadline_seconds,
            challenge_bond_denom,
            challenge_bond_amount,
            decay_half_life_seconds,
            default_min_stake,
        ),
        ExecuteMsg::SetCategoryMinStake {
            category,
            min_stake,
        } => exec_set_category_min_stake(deps, info, category, min_stake),
        ExecuteMsg::AddArbiter { address } => exec_add_arbiter(deps, info, address),
        ExecuteMsg::RemoveArbiter { address } => exec_remove_arbiter(deps, info, address),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Signal { signal_id } => to_json_binary(&query_signal(deps, signal_id)?),
        QueryMsg::SignalsBySubject {
            subject_type,
            subject_id,
            category,
        } => to_json_binary(&query_signals_by_subject(
            deps,
            subject_type,
            subject_id,
            category,
        )?),
        QueryMsg::ReputationScore {
            subject_type,
            subject_id,
            category,
        } => to_json_binary(&query_reputation_score(
            deps,
            env,
            subject_type,
            subject_id,
            category,
        )?),
        QueryMsg::Challenge { challenge_id } => {
            to_json_binary(&query_challenge(deps, challenge_id)?)
        }
        QueryMsg::ActiveChallenges { start_after, limit } => {
            to_json_binary(&query_active_challenges(deps, start_after, limit)?)
        }
        QueryMsg::CategoryMinStake { category } => {
            to_json_binary(&query_category_min_stake(deps, category)?)
        }
    }
}

// ---------------------------------------------------------------------------
// Execute handlers
// ---------------------------------------------------------------------------

fn exec_submit_signal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    subject_type: SubjectType,
    subject_id: String,
    category: String,
    endorsement_level: u8,
    evidence: Evidence,
) -> Result<Response, ContractError> {
    // Validate endorsement level 1-5
    if endorsement_level < 1 || endorsement_level > 5 {
        return Err(ContractError::InvalidEndorsementLevel {
            level: endorsement_level,
        });
    }

    // Validate evidence
    if !evidence.has_required_refs() {
        return Err(ContractError::InsufficientEvidence {});
    }

    let config = CONFIG.load(deps.storage)?;
    let now = env.block.time;

    // Allocate signal ID
    let signal_id = NEXT_SIGNAL_ID.load(deps.storage)?;
    NEXT_SIGNAL_ID.save(deps.storage, &(signal_id + 1))?;

    let activates_at = Timestamp::from_seconds(now.seconds() + config.activation_delay_seconds);

    let signal = Signal {
        id: signal_id,
        signaler: info.sender.clone(),
        subject_type: subject_type.clone(),
        subject_id: subject_id.clone(),
        category: category.clone(),
        endorsement_level,
        evidence,
        status: SignalStatus::Submitted,
        submitted_at: now,
        activates_at,
    };

    SIGNALS.save(deps.storage, signal_id, &signal)?;

    // Update subject index
    let key = subject_key(&subject_type, &subject_id, &category);
    let mut ids = SUBJECT_SIGNALS
        .may_load(deps.storage, &key)?
        .unwrap_or_default();
    ids.push(signal_id);
    SUBJECT_SIGNALS.save(deps.storage, &key, &ids)?;

    Ok(Response::new()
        .add_attribute("action", "submit_signal")
        .add_attribute("signal_id", signal_id.to_string())
        .add_attribute("signaler", info.sender.as_str())
        .add_attribute("subject_type", subject_type.to_string())
        .add_attribute("subject_id", &subject_id)
        .add_attribute("category", &category)
        .add_attribute("endorsement_level", endorsement_level.to_string()))
}

fn exec_activate_signal(
    deps: DepsMut,
    env: Env,
    signal_id: u64,
) -> Result<Response, ContractError> {
    let mut signal = SIGNALS
        .may_load(deps.storage, signal_id)?
        .ok_or(ContractError::SignalNotFound { id: signal_id })?;

    // Must be in Submitted state
    if !matches!(signal.status, SignalStatus::Submitted) {
        return Err(ContractError::SignalNotYetActive { id: signal_id });
    }

    // Activation delay must have passed
    if env.block.time < signal.activates_at {
        return Err(ContractError::SignalNotYetActive { id: signal_id });
    }

    signal.status = SignalStatus::Active;
    SIGNALS.save(deps.storage, signal_id, &signal)?;

    Ok(Response::new()
        .add_attribute("action", "activate_signal")
        .add_attribute("signal_id", signal_id.to_string()))
}

fn exec_withdraw_signal(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    signal_id: u64,
) -> Result<Response, ContractError> {
    let mut signal = SIGNALS
        .may_load(deps.storage, signal_id)?
        .ok_or(ContractError::SignalNotFound { id: signal_id })?;

    // Only the signaler can withdraw
    if info.sender != signal.signaler {
        return Err(ContractError::NotSignalOwner { id: signal_id });
    }

    // Cannot withdraw if terminal
    if signal.status.is_terminal() {
        return Err(ContractError::SignalTerminal { id: signal_id });
    }

    // Cannot withdraw if currently challenged (spec section 6.1)
    if matches!(
        signal.status,
        SignalStatus::Challenged | SignalStatus::Escalated
    ) {
        return Err(ContractError::WithdrawWhileChallenged { id: signal_id });
    }

    signal.status = SignalStatus::Withdrawn;
    SIGNALS.save(deps.storage, signal_id, &signal)?;

    Ok(Response::new()
        .add_attribute("action", "withdraw_signal")
        .add_attribute("signal_id", signal_id.to_string()))
}

fn exec_submit_challenge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    signal_id: u64,
    rationale: String,
    evidence: Evidence,
) -> Result<Response, ContractError> {
    let signal = SIGNALS
        .may_load(deps.storage, signal_id)?
        .ok_or(ContractError::SignalNotFound { id: signal_id })?;

    let config = CONFIG.load(deps.storage)?;

    // Only one active challenge per signal (check before state check for specificity)
    if SIGNAL_CHALLENGE.may_load(deps.storage, signal_id)?.is_some() {
        return Err(ContractError::SignalAlreadyChallenged { id: signal_id });
    }

    // Signal must be in SUBMITTED or ACTIVE state (spec section 6.1)
    if !matches!(
        signal.status,
        SignalStatus::Submitted | SignalStatus::Active
    ) {
        return Err(ContractError::SignalNotChallengeable { id: signal_id });
    }

    // Cannot self-challenge
    if info.sender == signal.signaler {
        return Err(ContractError::SelfChallenge {});
    }

    // Must be within challenge window
    let challenge_deadline = Timestamp::from_seconds(
        signal.submitted_at.seconds() + config.challenge_window_seconds,
    );
    if env.block.time > challenge_deadline {
        return Err(ContractError::ChallengeWindowExpired { id: signal_id });
    }

    // Evidence required
    if !evidence.has_required_refs() {
        return Err(ContractError::InsufficientEvidence {});
    }

    // Rationale minimum 50 chars
    if rationale.len() < 50 {
        return Err(ContractError::RationaleTooShort {});
    }

    // Validate bond if required
    if !config.challenge_bond_amount.is_zero() {
        let sent = info
            .funds
            .iter()
            .find(|c| c.denom == config.challenge_bond_denom)
            .map(|c| c.amount)
            .unwrap_or(Uint128::zero());

        if sent < config.challenge_bond_amount {
            return Err(ContractError::InsufficientBond {
                required: config.challenge_bond_amount.to_string(),
                sent: sent.to_string(),
            });
        }
    }

    // Create challenge
    let challenge_id = NEXT_CHALLENGE_ID.load(deps.storage)?;
    NEXT_CHALLENGE_ID.save(deps.storage, &(challenge_id + 1))?;

    let resolution_deadline = Timestamp::from_seconds(
        env.block.time.seconds() + config.resolution_deadline_seconds,
    );

    let challenge = Challenge {
        id: challenge_id,
        signal_id,
        challenger: info.sender.clone(),
        rationale,
        evidence,
        bond_amount: config.challenge_bond_amount,
        outcome: ChallengeOutcome::Pending,
        challenged_at: env.block.time,
        resolution_deadline,
        resolution_rationale: None,
    };

    CHALLENGES.save(deps.storage, challenge_id, &challenge)?;
    SIGNAL_CHALLENGE.save(deps.storage, signal_id, &challenge_id)?;

    // Transition signal to Challenged
    let mut signal = signal;
    signal.status = SignalStatus::Challenged;
    SIGNALS.save(deps.storage, signal_id, &signal)?;

    Ok(Response::new()
        .add_attribute("action", "submit_challenge")
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("signal_id", signal_id.to_string())
        .add_attribute("challenger", info.sender.as_str()))
}

fn exec_resolve_challenge(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    challenge_id: u64,
    outcome_valid: bool,
    rationale: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Must be admin or arbiter
    let is_admin = info.sender == config.admin;
    let is_arbiter = config.arbiters.contains(&info.sender);
    if !is_admin && !is_arbiter {
        return Err(ContractError::NotResolver {});
    }

    let mut challenge = CHALLENGES
        .may_load(deps.storage, challenge_id)?
        .ok_or(ContractError::ChallengeNotFound { id: challenge_id })?;

    // Must be pending
    if !matches!(challenge.outcome, ChallengeOutcome::Pending) {
        return Err(ContractError::ChallengeNotPending { id: challenge_id });
    }

    let signal_id = challenge.signal_id;
    let mut signal = SIGNALS.load(deps.storage, signal_id)?;

    let mut msgs: Vec<BankMsg> = vec![];

    if outcome_valid {
        // Signal found valid — restore
        challenge.outcome = ChallengeOutcome::Valid;
        signal.status = SignalStatus::ResolvedValid;

        // v1: challenger forfeits bond. In v0 with zero bond this is a no-op.
        // Bond stays in contract (could be distributed to community pool).
    } else {
        // Signal found invalid — permanently remove
        challenge.outcome = ChallengeOutcome::Invalid;
        signal.status = SignalStatus::ResolvedInvalid;

        // v1: return bond to challenger (or reward them). In v0 this is a no-op.
        if !challenge.bond_amount.is_zero() {
            msgs.push(BankMsg::Send {
                to_address: challenge.challenger.to_string(),
                amount: vec![Coin {
                    denom: config.challenge_bond_denom.clone(),
                    amount: challenge.bond_amount,
                }],
            });
        }
    }

    challenge.resolution_rationale = Some(rationale);
    CHALLENGES.save(deps.storage, challenge_id, &challenge)?;
    SIGNALS.save(deps.storage, signal_id, &signal)?;

    // Remove active challenge index
    SIGNAL_CHALLENGE.remove(deps.storage, signal_id);

    let mut resp = Response::new()
        .add_attribute("action", "resolve_challenge")
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("signal_id", signal_id.to_string())
        .add_attribute(
            "outcome",
            if outcome_valid { "valid" } else { "invalid" },
        );

    for msg in msgs {
        resp = resp.add_message(msg);
    }

    Ok(resp)
}

fn exec_escalate_challenge(
    deps: DepsMut,
    env: Env,
    challenge_id: u64,
) -> Result<Response, ContractError> {
    let challenge = CHALLENGES
        .may_load(deps.storage, challenge_id)?
        .ok_or(ContractError::ChallengeNotFound { id: challenge_id })?;

    // Must be pending
    if !matches!(challenge.outcome, ChallengeOutcome::Pending) {
        return Err(ContractError::ChallengeNotPending { id: challenge_id });
    }

    // Deadline must have passed
    if env.block.time <= challenge.resolution_deadline {
        return Err(ContractError::DeadlineNotExceeded { id: challenge_id });
    }

    // Transition signal to Escalated
    let signal_id = challenge.signal_id;
    let mut signal = SIGNALS.load(deps.storage, signal_id)?;
    signal.status = SignalStatus::Escalated;
    SIGNALS.save(deps.storage, signal_id, &signal)?;

    Ok(Response::new()
        .add_attribute("action", "escalate_challenge")
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("signal_id", signal_id.to_string()))
}

fn exec_invalidate_signal(
    deps: DepsMut,
    info: MessageInfo,
    signal_id: u64,
    rationale: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Admin only
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Rationale required
    if rationale.is_empty() {
        return Err(ContractError::InvalidationRationaleRequired {});
    }

    let mut signal = SIGNALS
        .may_load(deps.storage, signal_id)?
        .ok_or(ContractError::SignalNotFound { id: signal_id })?;

    // Cannot invalidate terminal signals
    if signal.status.is_terminal() {
        return Err(ContractError::SignalTerminal { id: signal_id });
    }

    signal.status = SignalStatus::Invalidated;
    SIGNALS.save(deps.storage, signal_id, &signal)?;

    Ok(Response::new()
        .add_attribute("action", "invalidate_signal")
        .add_attribute("signal_id", signal_id.to_string())
        .add_attribute("rationale", &rationale))
}

fn exec_update_config(
    deps: DepsMut,
    info: MessageInfo,
    activation_delay_seconds: Option<u64>,
    challenge_window_seconds: Option<u64>,
    resolution_deadline_seconds: Option<u64>,
    challenge_bond_denom: Option<String>,
    challenge_bond_amount: Option<Uint128>,
    decay_half_life_seconds: Option<u64>,
    default_min_stake: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(v) = activation_delay_seconds {
        config.activation_delay_seconds = v;
    }
    if let Some(v) = challenge_window_seconds {
        config.challenge_window_seconds = v;
    }
    if let Some(v) = resolution_deadline_seconds {
        config.resolution_deadline_seconds = v;
    }
    if let Some(v) = challenge_bond_denom {
        config.challenge_bond_denom = v;
    }
    if let Some(v) = challenge_bond_amount {
        config.challenge_bond_amount = v;
    }
    if let Some(v) = decay_half_life_seconds {
        config.decay_half_life_seconds = v;
    }
    if let Some(v) = default_min_stake {
        config.default_min_stake = v;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn exec_set_category_min_stake(
    deps: DepsMut,
    info: MessageInfo,
    category: String,
    min_stake: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    CATEGORY_MIN_STAKE.save(deps.storage, &category, &min_stake)?;

    Ok(Response::new()
        .add_attribute("action", "set_category_min_stake")
        .add_attribute("category", &category)
        .add_attribute("min_stake", min_stake.to_string()))
}

fn exec_add_arbiter(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let addr = deps.api.addr_validate(&address)?;
    if !config.arbiters.contains(&addr) {
        config.arbiters.push(addr.clone());
        CONFIG.save(deps.storage, &config)?;
    }

    Ok(Response::new()
        .add_attribute("action", "add_arbiter")
        .add_attribute("arbiter", addr.as_str()))
}

fn exec_remove_arbiter(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let addr = deps.api.addr_validate(&address)?;
    config.arbiters.retain(|a| a != &addr);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "remove_arbiter")
        .add_attribute("arbiter", addr.as_str()))
}

// ---------------------------------------------------------------------------
// Query handlers
// ---------------------------------------------------------------------------

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_signal(deps: Deps, signal_id: u64) -> StdResult<SignalResponse> {
    let signal = SIGNALS.load(deps.storage, signal_id)?;
    Ok(SignalResponse { signal })
}

fn query_signals_by_subject(
    deps: Deps,
    subject_type: SubjectType,
    subject_id: String,
    category: String,
) -> StdResult<SignalsBySubjectResponse> {
    let key = subject_key(&subject_type, &subject_id, &category);
    let ids = SUBJECT_SIGNALS
        .may_load(deps.storage, &key)?
        .unwrap_or_default();

    let mut signals = Vec::with_capacity(ids.len());
    for id in ids {
        if let Ok(s) = SIGNALS.load(deps.storage, id) {
            signals.push(s);
        }
    }

    Ok(SignalsBySubjectResponse { signals })
}

fn query_reputation_score(
    deps: Deps,
    env: Env,
    subject_type: SubjectType,
    subject_id: String,
    category: String,
) -> StdResult<ReputationScoreResponse> {
    let config = CONFIG.load(deps.storage)?;
    let key = subject_key(&subject_type, &subject_id, &category);
    let ids = SUBJECT_SIGNALS
        .may_load(deps.storage, &key)?
        .unwrap_or_default();

    let total_signals = ids.len() as u32;
    let now = env.block.time;

    // v0 scoring: decay-weighted average of endorsement_level/5 (no stake weighting)
    // score = sum(decay * endorsement_level / 5) / sum(decay)
    // decay = exp(-lambda * age_seconds) where lambda = ln(2) / half_life_seconds
    let half_life = config.decay_half_life_seconds as f64;
    let lambda = (2.0_f64).ln() / half_life;

    let mut w_sum: f64 = 0.0;
    let mut d_sum: f64 = 0.0;
    let mut contributing: u32 = 0;

    for id in &ids {
        if let Ok(signal) = SIGNALS.load(deps.storage, *id) {
            if !signal.status.contributes_to_score() {
                continue;
            }
            contributing += 1;

            let age_secs = now.seconds().saturating_sub(signal.submitted_at.seconds()) as f64;
            let decay = (-lambda * age_secs).exp();
            let w = signal.endorsement_level as f64 / 5.0;

            w_sum += w * decay;
            d_sum += decay;
        }
    }

    let score_0_1 = if d_sum > 0.0 { w_sum / d_sum } else { 0.0 };
    // Scale to 0-1000
    let score = (score_0_1 * 1000.0).round().min(1000.0) as u64;

    Ok(ReputationScoreResponse {
        score,
        contributing_signals: contributing,
        total_signals,
    })
}

fn query_challenge(deps: Deps, challenge_id: u64) -> StdResult<ChallengeResponse> {
    let challenge = CHALLENGES.load(deps.storage, challenge_id)?;
    Ok(ChallengeResponse { challenge })
}

fn query_active_challenges(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ActiveChallengesResponse> {
    let limit = limit.unwrap_or(30).min(100) as usize;
    let start = start_after.map(|s| s + 1).unwrap_or(0);

    let mut challenges = Vec::new();
    // Iterate challenges from start
    for result in CHALLENGES
        .range(deps.storage, Some(Bound::inclusive(start)), None, Order::Ascending)
    {
        let (_, challenge) = result?;
        if matches!(challenge.outcome, ChallengeOutcome::Pending) {
            challenges.push(challenge);
            if challenges.len() >= limit {
                break;
            }
        }
    }

    Ok(ActiveChallengesResponse { challenges })
}

fn query_category_min_stake(deps: Deps, category: String) -> StdResult<CategoryMinStakeResponse> {
    let config = CONFIG.load(deps.storage)?;
    let min_stake = CATEGORY_MIN_STAKE
        .may_load(deps.storage, &category)?
        .unwrap_or(config.default_min_stake);

    Ok(CategoryMinStakeResponse {
        category,
        min_stake,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the composite key for subject signal indexing.
fn subject_key(subject_type: &SubjectType, subject_id: &str, category: &str) -> String {
    format!("{}:{}:{}", subject_type, subject_id, category)
}
