use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    BondStatusResponse, ChallengeResponse, CollectionResponse, CollectionsResponse,
    ConfigResponse, CuratorStatsResponse, ExecuteMsg, InstantiateMsg, QualityScoreResponse,
    QueryMsg,
};
use crate::state::{
    Challenge, ChallengeResolution, Collection, CollectionStatus, Config, QualityScore,
    BATCH_COLLECTIONS, CHALLENGES, COLLECTIONS, COLLECTION_CHALLENGES, CONFIG,
    CURATOR_COLLECTION_COUNT, NEXT_CHALLENGE_ID, NEXT_COLLECTION_ID, QUALITY_SCORES,
};

const CONTRACT_NAME: &str = "crates.io:marketplace-curation";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

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
        community_pool: deps.api.addr_validate(&msg.community_pool)?,
        min_curation_bond: msg.min_curation_bond.unwrap_or(Uint128::new(1_000_000_000)),
        curation_fee_rate_bps: msg.curation_fee_rate_bps.unwrap_or(50),
        challenge_deposit: msg.challenge_deposit.unwrap_or(Uint128::new(100_000_000)),
        slash_percentage_bps: msg.slash_percentage_bps.unwrap_or(2000),
        activation_delay_seconds: msg.activation_delay_seconds.unwrap_or(172_800), // 48h
        unbonding_period_seconds: msg.unbonding_period_seconds.unwrap_or(1_209_600), // 14 days
        bond_top_up_window_seconds: msg.bond_top_up_window_seconds.unwrap_or(604_800), // 7 days
        min_quality_score: msg.min_quality_score.unwrap_or(300),
        max_collections_per_curator: msg.max_collections_per_curator.unwrap_or(5),
        denom: msg.denom,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    NEXT_COLLECTION_ID.save(deps.storage, &1u64)?;
    NEXT_CHALLENGE_ID.save(deps.storage, &1u64)?;

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
        ExecuteMsg::CreateCollection { name, criteria } => {
            execute_create_collection(deps, env, info, name, criteria)
        }
        ExecuteMsg::ActivateCollection { collection_id } => {
            execute_activate_collection(deps, env, info, collection_id)
        }
        ExecuteMsg::AddToCollection {
            collection_id,
            batch_denom,
        } => execute_add_to_collection(deps, env, info, collection_id, batch_denom),
        ExecuteMsg::RemoveFromCollection {
            collection_id,
            batch_denom,
        } => execute_remove_from_collection(deps, info, collection_id, batch_denom),
        ExecuteMsg::CloseCollection { collection_id } => {
            execute_close_collection(deps, env, info, collection_id)
        }
        ExecuteMsg::TopUpBond { collection_id } => {
            execute_top_up_bond(deps, info, collection_id)
        }
        ExecuteMsg::ChallengeBatchInclusion {
            collection_id,
            batch_denom,
            evidence,
        } => execute_challenge_batch(deps, env, info, collection_id, batch_denom, evidence),
        ExecuteMsg::ResolveChallenge {
            challenge_id,
            resolution,
        } => execute_resolve_challenge(deps, env, info, challenge_id, resolution),
        ExecuteMsg::SubmitQualityScore {
            batch_denom,
            score,
            confidence,
        } => execute_submit_quality_score(deps, env, info, batch_denom, score, confidence),
        ExecuteMsg::WithdrawBond { collection_id } => {
            execute_withdraw_bond(deps, env, info, collection_id)
        }
        ExecuteMsg::UpdateConfig {
            community_pool,
            min_curation_bond,
            curation_fee_rate_bps,
            challenge_deposit,
            slash_percentage_bps,
            activation_delay_seconds,
            unbonding_period_seconds,
            bond_top_up_window_seconds,
            min_quality_score,
            max_collections_per_curator,
        } => execute_update_config(
            deps,
            info,
            community_pool,
            min_curation_bond,
            curation_fee_rate_bps,
            challenge_deposit,
            slash_percentage_bps,
            activation_delay_seconds,
            unbonding_period_seconds,
            bond_top_up_window_seconds,
            min_quality_score,
            max_collections_per_curator,
        ),
    }
}

// ── Execute handlers ───────────────────────────────────────────────────

fn execute_create_collection(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    criteria: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Verify bond sent
    let sent = must_pay(&info, &config.denom)?;
    if sent < config.min_curation_bond {
        return Err(ContractError::InsufficientBond {
            required: config.min_curation_bond.to_string(),
            sent: sent.to_string(),
        });
    }

    // Check curator hasn't exceeded max collections
    let count = CURATOR_COLLECTION_COUNT
        .may_load(deps.storage, &info.sender)?
        .unwrap_or(0);
    if count >= config.max_collections_per_curator {
        return Err(ContractError::MaxCollectionsExceeded {
            max: config.max_collections_per_curator,
        });
    }

    let id = NEXT_COLLECTION_ID.load(deps.storage)?;
    let activates_at = env
        .block
        .time
        .plus_seconds(config.activation_delay_seconds);

    let collection = Collection {
        id,
        curator: info.sender.clone(),
        name,
        criteria,
        status: CollectionStatus::Proposed,
        bond_amount: sent,
        batches: vec![],
        created_at: env.block.time,
        activates_at,
        suspension_expires_at: None,
        closed_at: None,
    };

    COLLECTIONS.save(deps.storage, id, &collection)?;
    NEXT_COLLECTION_ID.save(deps.storage, &(id + 1))?;
    CURATOR_COLLECTION_COUNT.save(deps.storage, &info.sender, &(count + 1))?;
    COLLECTION_CHALLENGES.save(deps.storage, id, &vec![])?;

    Ok(Response::new()
        .add_attribute("action", "create_collection")
        .add_attribute("collection_id", id.to_string())
        .add_attribute("curator", info.sender)
        .add_attribute("bond_amount", sent))
}

fn execute_activate_collection(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection_id: u64,
) -> Result<Response, ContractError> {
    let mut collection = COLLECTIONS
        .may_load(deps.storage, collection_id)?
        .ok_or(ContractError::CollectionNotFound { id: collection_id })?;

    // Only curator can activate
    if info.sender != collection.curator {
        return Err(ContractError::Unauthorized {
            reason: "only the curator can activate".to_string(),
        });
    }

    if collection.status != CollectionStatus::Proposed {
        return Err(ContractError::InvalidCollectionStatus {
            expected: "Proposed".to_string(),
            actual: collection.status.to_string(),
        });
    }

    // Check activation delay
    if env.block.time < collection.activates_at {
        return Err(ContractError::ActivationDelayNotElapsed);
    }

    // Check no pending challenges
    let challenge_ids = COLLECTION_CHALLENGES
        .may_load(deps.storage, collection_id)?
        .unwrap_or_default();
    for cid in &challenge_ids {
        let challenge = CHALLENGES.load(deps.storage, *cid)?;
        if challenge.resolution.is_none() {
            return Err(ContractError::PendingChallenges);
        }
    }

    collection.status = CollectionStatus::Active;
    COLLECTIONS.save(deps.storage, collection_id, &collection)?;

    Ok(Response::new()
        .add_attribute("action", "activate_collection")
        .add_attribute("collection_id", collection_id.to_string()))
}

fn execute_add_to_collection(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection_id: u64,
    batch_denom: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut collection = COLLECTIONS
        .may_load(deps.storage, collection_id)?
        .ok_or(ContractError::CollectionNotFound { id: collection_id })?;

    // Only curator
    if info.sender != collection.curator {
        return Err(ContractError::Unauthorized {
            reason: "only the curator can add batches".to_string(),
        });
    }

    // Must be Active
    if collection.status != CollectionStatus::Active {
        return Err(ContractError::InvalidCollectionStatus {
            expected: "Active".to_string(),
            actual: collection.status.to_string(),
        });
    }

    // Check not already in collection
    if collection.batches.contains(&batch_denom) {
        return Err(ContractError::BatchAlreadyInCollection {
            batch_denom,
            collection_id,
        });
    }

    // Check quality score exists and meets minimum
    let qs = QUALITY_SCORES
        .may_load(deps.storage, &batch_denom)?
        .ok_or(ContractError::NoQualityScore {
            batch_denom: batch_denom.clone(),
        })?;
    if qs.score < config.min_quality_score {
        return Err(ContractError::QualityScoreTooLow {
            batch_denom,
            score: qs.score,
            min: config.min_quality_score,
        });
    }

    collection.batches.push(batch_denom.clone());
    COLLECTIONS.save(deps.storage, collection_id, &collection)?;

    // Maintain reverse index: batch_denom -> collection IDs
    let mut batch_colls = BATCH_COLLECTIONS
        .may_load(deps.storage, &batch_denom)?
        .unwrap_or_default();
    batch_colls.push(collection_id);
    BATCH_COLLECTIONS.save(deps.storage, &batch_denom, &batch_colls)?;

    Ok(Response::new()
        .add_attribute("action", "add_to_collection")
        .add_attribute("collection_id", collection_id.to_string())
        .add_attribute("batch_denom", batch_denom))
}

fn execute_remove_from_collection(
    deps: DepsMut,
    info: MessageInfo,
    collection_id: u64,
    batch_denom: String,
) -> Result<Response, ContractError> {
    let mut collection = COLLECTIONS
        .may_load(deps.storage, collection_id)?
        .ok_or(ContractError::CollectionNotFound { id: collection_id })?;

    // Only curator
    if info.sender != collection.curator {
        return Err(ContractError::Unauthorized {
            reason: "only the curator can remove batches".to_string(),
        });
    }

    let pos = collection
        .batches
        .iter()
        .position(|b| b == &batch_denom)
        .ok_or(ContractError::BatchNotInCollection {
            batch_denom: batch_denom.clone(),
            collection_id,
        })?;

    collection.batches.remove(pos);
    COLLECTIONS.save(deps.storage, collection_id, &collection)?;

    // Maintain reverse index: remove this collection from the batch's list
    let mut batch_colls = BATCH_COLLECTIONS
        .may_load(deps.storage, &batch_denom)?
        .unwrap_or_default();
    batch_colls.retain(|&cid| cid != collection_id);
    if batch_colls.is_empty() {
        BATCH_COLLECTIONS.remove(deps.storage, &batch_denom);
    } else {
        BATCH_COLLECTIONS.save(deps.storage, &batch_denom, &batch_colls)?;
    }

    Ok(Response::new()
        .add_attribute("action", "remove_from_collection")
        .add_attribute("collection_id", collection_id.to_string())
        .add_attribute("batch_denom", batch_denom))
}

fn execute_close_collection(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection_id: u64,
) -> Result<Response, ContractError> {
    let mut collection = COLLECTIONS
        .may_load(deps.storage, collection_id)?
        .ok_or(ContractError::CollectionNotFound { id: collection_id })?;

    // Only curator
    if info.sender != collection.curator {
        return Err(ContractError::Unauthorized {
            reason: "only the curator can close".to_string(),
        });
    }

    // Check no pending challenges
    let challenge_ids = COLLECTION_CHALLENGES
        .may_load(deps.storage, collection_id)?
        .unwrap_or_default();
    for cid in &challenge_ids {
        let challenge = CHALLENGES.load(deps.storage, *cid)?;
        if challenge.resolution.is_none() {
            return Err(ContractError::PendingChallenges);
        }
    }

    collection.status = CollectionStatus::Closed;
    collection.closed_at = Some(env.block.time);
    COLLECTIONS.save(deps.storage, collection_id, &collection)?;

    Ok(Response::new()
        .add_attribute("action", "close_collection")
        .add_attribute("collection_id", collection_id.to_string()))
}

fn execute_top_up_bond(
    deps: DepsMut,
    info: MessageInfo,
    collection_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut collection = COLLECTIONS
        .may_load(deps.storage, collection_id)?
        .ok_or(ContractError::CollectionNotFound { id: collection_id })?;

    // Only curator
    if info.sender != collection.curator {
        return Err(ContractError::Unauthorized {
            reason: "only the curator can top up bond".to_string(),
        });
    }

    // Must be Suspended
    if collection.status != CollectionStatus::Suspended {
        return Err(ContractError::InvalidCollectionStatus {
            expected: "Suspended".to_string(),
            actual: collection.status.to_string(),
        });
    }

    let sent = must_pay(&info, &config.denom)?;
    collection.bond_amount += sent;

    // Restore to Active if bond is now sufficient
    if collection.bond_amount >= config.min_curation_bond {
        collection.status = CollectionStatus::Active;
        collection.suspension_expires_at = None;
    }

    COLLECTIONS.save(deps.storage, collection_id, &collection)?;

    Ok(Response::new()
        .add_attribute("action", "top_up_bond")
        .add_attribute("collection_id", collection_id.to_string())
        .add_attribute("new_bond", collection.bond_amount)
        .add_attribute("status", collection.status.to_string()))
}

fn execute_challenge_batch(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection_id: u64,
    batch_denom: String,
    evidence: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut collection = COLLECTIONS
        .may_load(deps.storage, collection_id)?
        .ok_or(ContractError::CollectionNotFound { id: collection_id })?;

    // Verify deposit
    let sent = must_pay(&info, &config.denom)?;
    if sent < config.challenge_deposit {
        return Err(ContractError::InsufficientDeposit {
            required: config.challenge_deposit.to_string(),
            sent: sent.to_string(),
        });
    }

    // Collection must be Active or Proposed (not already Closed/Suspended)
    if collection.status != CollectionStatus::Active
        && collection.status != CollectionStatus::Proposed
    {
        return Err(ContractError::InvalidCollectionStatus {
            expected: "Active or Proposed".to_string(),
            actual: collection.status.to_string(),
        });
    }

    let challenge_id = NEXT_CHALLENGE_ID.load(deps.storage)?;
    let challenge = Challenge {
        id: challenge_id,
        collection_id,
        challenger: info.sender.clone(),
        batch_denom: batch_denom.clone(),
        deposit: sent,
        evidence,
        filed_at: env.block.time,
        resolution: None,
    };

    CHALLENGES.save(deps.storage, challenge_id, &challenge)?;
    NEXT_CHALLENGE_ID.save(deps.storage, &(challenge_id + 1))?;

    // Add to collection's challenge list
    let mut challenge_ids = COLLECTION_CHALLENGES
        .may_load(deps.storage, collection_id)?
        .unwrap_or_default();
    challenge_ids.push(challenge_id);
    COLLECTION_CHALLENGES.save(deps.storage, collection_id, &challenge_ids)?;

    // Move collection to UnderReview
    collection.status = CollectionStatus::UnderReview;
    COLLECTIONS.save(deps.storage, collection_id, &collection)?;

    Ok(Response::new()
        .add_attribute("action", "challenge_batch")
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("collection_id", collection_id.to_string())
        .add_attribute("batch_denom", batch_denom)
        .add_attribute("challenger", info.sender))
}

fn execute_resolve_challenge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    challenge_id: u64,
    resolution: ChallengeResolution,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Admin only
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "only admin can resolve challenges".to_string(),
        });
    }

    let mut challenge = CHALLENGES
        .may_load(deps.storage, challenge_id)?
        .ok_or(ContractError::ChallengeNotFound { id: challenge_id })?;

    if challenge.resolution.is_some() {
        return Err(ContractError::ChallengeAlreadyResolved);
    }

    challenge.resolution = Some(resolution.clone());
    CHALLENGES.save(deps.storage, challenge_id, &challenge)?;

    let mut collection = COLLECTIONS.load(deps.storage, challenge.collection_id)?;
    let mut msgs: Vec<BankMsg> = vec![];

    match resolution {
        ChallengeResolution::CuratorWins => {
            // Challenger loses deposit — send to community pool
            msgs.push(BankMsg::Send {
                to_address: config.community_pool.to_string(),
                amount: vec![Coin {
                    denom: config.denom.clone(),
                    amount: challenge.deposit,
                }],
            });

            // Restore collection to Active if no other pending challenges
            if all_challenges_resolved(deps.storage, challenge.collection_id)? {
                collection.status = CollectionStatus::Active;
            }
        }
        ChallengeResolution::ChallengerWins => {
            // Slash curator bond by slash_percentage_bps
            let slash_amount = collection
                .bond_amount
                .multiply_ratio(config.slash_percentage_bps, 10_000u64);

            collection.bond_amount = collection.bond_amount.saturating_sub(slash_amount);

            // Split: 50% to challenger, 50% to community pool
            let challenger_share = slash_amount.multiply_ratio(1u128, 2u128);
            let community_share = slash_amount - challenger_share;

            // Return challenger deposit + their share of slash
            let challenger_total = challenge.deposit + challenger_share;
            msgs.push(BankMsg::Send {
                to_address: challenge.challenger.to_string(),
                amount: vec![Coin {
                    denom: config.denom.clone(),
                    amount: challenger_total,
                }],
            });

            if !community_share.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: config.community_pool.to_string(),
                    amount: vec![Coin {
                        denom: config.denom.clone(),
                        amount: community_share,
                    }],
                });
            }

            // Suspend if bond is below minimum
            if collection.bond_amount < config.min_curation_bond {
                collection.status = CollectionStatus::Suspended;
                collection.suspension_expires_at = Some(
                    env.block
                        .time
                        .plus_seconds(config.bond_top_up_window_seconds),
                );
            } else if all_challenges_resolved(deps.storage, challenge.collection_id)? {
                collection.status = CollectionStatus::Active;
            }
        }
    }

    COLLECTIONS.save(deps.storage, challenge.collection_id, &collection)?;

    let mut resp = Response::new()
        .add_attribute("action", "resolve_challenge")
        .add_attribute("challenge_id", challenge_id.to_string())
        .add_attribute("collection_id", challenge.collection_id.to_string())
        .add_attribute("status", collection.status.to_string());

    for msg in msgs {
        resp = resp.add_message(msg);
    }

    Ok(resp)
}

fn execute_submit_quality_score(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    batch_denom: String,
    score: u32,
    confidence: u32,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Admin/agent only
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "only admin/agent can submit quality scores".to_string(),
        });
    }

    if score > 1000 {
        return Err(ContractError::InvalidScore { value: score });
    }
    if confidence > 1000 {
        return Err(ContractError::InvalidConfidence { value: confidence });
    }

    let qs = QualityScore {
        batch_denom: batch_denom.clone(),
        score,
        confidence,
        computed_at: env.block.time,
    };
    QUALITY_SCORES.save(deps.storage, &batch_denom, &qs)?;

    // Auto-remove from any collections if score dropped below minimum
    let mut removed_from: Vec<u64> = vec![];
    if score < config.min_quality_score {
        // Use reverse index to find only the collections containing this batch
        let batch_colls = BATCH_COLLECTIONS
            .may_load(deps.storage, &batch_denom)?
            .unwrap_or_default();

        for cid in batch_colls {
            let mut coll = COLLECTIONS.load(deps.storage, cid)?;
            if let Some(pos) = coll.batches.iter().position(|b| b == &batch_denom) {
                coll.batches.remove(pos);
                COLLECTIONS.save(deps.storage, cid, &coll)?;
                removed_from.push(cid);
            }
        }

        // Clean up the reverse index entry since batch is removed from all collections
        if !removed_from.is_empty() {
            BATCH_COLLECTIONS.remove(deps.storage, &batch_denom);
        }
    }

    Ok(Response::new()
        .add_attribute("action", "submit_quality_score")
        .add_attribute("batch_denom", batch_denom)
        .add_attribute("score", score.to_string())
        .add_attribute("confidence", confidence.to_string())
        .add_attribute("auto_removed_from", format!("{:?}", removed_from)))
}

fn execute_withdraw_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut collection = COLLECTIONS
        .may_load(deps.storage, collection_id)?
        .ok_or(ContractError::CollectionNotFound { id: collection_id })?;

    // Only curator
    if info.sender != collection.curator {
        return Err(ContractError::Unauthorized {
            reason: "only the curator can withdraw bond".to_string(),
        });
    }

    // Must be Closed
    if collection.status != CollectionStatus::Closed {
        return Err(ContractError::InvalidCollectionStatus {
            expected: "Closed".to_string(),
            actual: collection.status.to_string(),
        });
    }

    // Check unbonding period
    let closed_at = collection.closed_at.unwrap_or(env.block.time);
    let unbonds_at = closed_at.plus_seconds(config.unbonding_period_seconds);
    if env.block.time < unbonds_at {
        return Err(ContractError::UnbondingNotComplete);
    }

    let amount = collection.bond_amount;
    collection.bond_amount = Uint128::zero();
    COLLECTIONS.save(deps.storage, collection_id, &collection)?;

    // Decrement curator count
    let count = CURATOR_COLLECTION_COUNT
        .may_load(deps.storage, &info.sender)?
        .unwrap_or(1);
    CURATOR_COLLECTION_COUNT.save(deps.storage, &info.sender, &count.saturating_sub(1))?;

    let msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: config.denom,
            amount,
        }],
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "withdraw_bond")
        .add_attribute("collection_id", collection_id.to_string())
        .add_attribute("amount", amount))
}

#[allow(clippy::too_many_arguments)]
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    community_pool: Option<String>,
    min_curation_bond: Option<Uint128>,
    curation_fee_rate_bps: Option<u64>,
    challenge_deposit: Option<Uint128>,
    slash_percentage_bps: Option<u64>,
    activation_delay_seconds: Option<u64>,
    unbonding_period_seconds: Option<u64>,
    bond_top_up_window_seconds: Option<u64>,
    min_quality_score: Option<u32>,
    max_collections_per_curator: Option<u32>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "only admin can update config".to_string(),
        });
    }

    if let Some(cp) = community_pool {
        config.community_pool = deps.api.addr_validate(&cp)?;
    }
    if let Some(v) = min_curation_bond {
        config.min_curation_bond = v;
    }
    if let Some(v) = curation_fee_rate_bps {
        config.curation_fee_rate_bps = v;
    }
    if let Some(v) = challenge_deposit {
        config.challenge_deposit = v;
    }
    if let Some(v) = slash_percentage_bps {
        config.slash_percentage_bps = v;
    }
    if let Some(v) = activation_delay_seconds {
        config.activation_delay_seconds = v;
    }
    if let Some(v) = unbonding_period_seconds {
        config.unbonding_period_seconds = v;
    }
    if let Some(v) = bond_top_up_window_seconds {
        config.bond_top_up_window_seconds = v;
    }
    if let Some(v) = min_quality_score {
        config.min_quality_score = v;
    }
    if let Some(v) = max_collections_per_curator {
        config.max_collections_per_curator = v;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// ── Query ──────────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Collection { collection_id } => {
            to_json_binary(&query_collection(deps, collection_id)?)
        }
        QueryMsg::Collections {
            status,
            curator,
            start_after,
            limit,
        } => to_json_binary(&query_collections(deps, status, curator, start_after, limit)?),
        QueryMsg::QualityScore { batch_denom } => {
            to_json_binary(&query_quality_score(deps, batch_denom)?)
        }
        QueryMsg::Challenge { challenge_id } => {
            to_json_binary(&query_challenge(deps, challenge_id)?)
        }
        QueryMsg::CuratorStats { curator } => to_json_binary(&query_curator_stats(deps, curator)?),
        QueryMsg::BondStatus { collection_id } => {
            to_json_binary(&query_bond_status(deps, collection_id)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.to_string(),
        community_pool: config.community_pool.to_string(),
        min_curation_bond: config.min_curation_bond,
        curation_fee_rate_bps: config.curation_fee_rate_bps,
        challenge_deposit: config.challenge_deposit,
        slash_percentage_bps: config.slash_percentage_bps,
        activation_delay_seconds: config.activation_delay_seconds,
        unbonding_period_seconds: config.unbonding_period_seconds,
        bond_top_up_window_seconds: config.bond_top_up_window_seconds,
        min_quality_score: config.min_quality_score,
        max_collections_per_curator: config.max_collections_per_curator,
        denom: config.denom,
    })
}

fn query_collection(deps: Deps, collection_id: u64) -> StdResult<CollectionResponse> {
    let collection = COLLECTIONS.load(deps.storage, collection_id)?;
    Ok(CollectionResponse { collection })
}

fn query_collections(
    deps: Deps,
    status: Option<CollectionStatus>,
    curator: Option<String>,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<CollectionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| cw_storage_plus::Bound::exclusive(s));

    let curator_addr = curator
        .map(|c| deps.api.addr_validate(&c))
        .transpose()?;

    let collections: Vec<Collection> = COLLECTIONS
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|item| {
            let (_, coll) = item.ok()?;
            if let Some(ref s) = status {
                if &coll.status != s {
                    return None;
                }
            }
            if let Some(ref c) = curator_addr {
                if &coll.curator != c {
                    return None;
                }
            }
            Some(coll)
        })
        .take(limit)
        .collect();

    Ok(CollectionsResponse { collections })
}

fn query_quality_score(deps: Deps, batch_denom: String) -> StdResult<QualityScoreResponse> {
    let qs = QUALITY_SCORES.may_load(deps.storage, &batch_denom)?;
    Ok(QualityScoreResponse { quality_score: qs })
}

fn query_challenge(deps: Deps, challenge_id: u64) -> StdResult<ChallengeResponse> {
    let challenge = CHALLENGES.load(deps.storage, challenge_id)?;
    Ok(ChallengeResponse { challenge })
}

fn query_curator_stats(deps: Deps, curator: String) -> StdResult<CuratorStatsResponse> {
    let curator_addr = deps.api.addr_validate(&curator)?;
    let config = CONFIG.load(deps.storage)?;
    let count = CURATOR_COLLECTION_COUNT
        .may_load(deps.storage, &curator_addr)?
        .unwrap_or(0);
    Ok(CuratorStatsResponse {
        curator,
        collection_count: count,
        max_collections: config.max_collections_per_curator,
    })
}

fn query_bond_status(deps: Deps, collection_id: u64) -> StdResult<BondStatusResponse> {
    let config = CONFIG.load(deps.storage)?;
    let collection = COLLECTIONS.load(deps.storage, collection_id)?;
    Ok(BondStatusResponse {
        collection_id,
        bond_amount: collection.bond_amount,
        min_required: config.min_curation_bond,
        is_sufficient: collection.bond_amount >= config.min_curation_bond,
        denom: config.denom,
    })
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Extract the single coin payment matching expected denom
fn must_pay(info: &MessageInfo, denom: &str) -> Result<Uint128, ContractError> {
    if info.funds.len() != 1 {
        return Err(ContractError::WrongDenom {
            expected: denom.to_string(),
            got: format!("{} coins sent", info.funds.len()),
        });
    }
    let coin = &info.funds[0];
    if coin.denom != denom {
        return Err(ContractError::WrongDenom {
            expected: denom.to_string(),
            got: coin.denom.clone(),
        });
    }
    Ok(coin.amount)
}

/// Check if all challenges for a collection are resolved
fn all_challenges_resolved(
    storage: &dyn cosmwasm_std::Storage,
    collection_id: u64,
) -> StdResult<bool> {
    let challenge_ids = COLLECTION_CHALLENGES
        .may_load(storage, collection_id)?
        .unwrap_or_default();
    for cid in challenge_ids {
        let challenge = CHALLENGES.load(storage, cid)?;
        if challenge.resolution.is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
    use cosmwasm_std::{Addr, Coin, Uint128};

    const DENOM: &str = "uregen";
    const MIN_BOND: u128 = 1_000_000_000;
    const CHALLENGE_DEP: u128 = 100_000_000;

    fn addr(input: &str) -> Addr {
        MockApi::default().addr_make(input)
    }

    fn setup_contract(deps: DepsMut) -> MessageInfo {
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            community_pool: addr("community_pool").to_string(),
            min_curation_bond: None,
            curation_fee_rate_bps: None,
            challenge_deposit: None,
            slash_percentage_bps: None,
            activation_delay_seconds: Some(0), // no delay for easier testing
            unbonding_period_seconds: Some(0), // no unbonding for easier testing
            bond_top_up_window_seconds: None,
            min_quality_score: None,
            max_collections_per_curator: None,
            denom: DENOM.to_string(),
        };
        instantiate(deps, mock_env(), info.clone(), msg).unwrap();
        info
    }

    fn setup_contract_with_delays(deps: DepsMut) -> MessageInfo {
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            community_pool: addr("community_pool").to_string(),
            min_curation_bond: None,
            curation_fee_rate_bps: None,
            challenge_deposit: None,
            slash_percentage_bps: None,
            activation_delay_seconds: Some(172_800), // 48h
            unbonding_period_seconds: Some(1_209_600), // 14 days
            bond_top_up_window_seconds: None,
            min_quality_score: None,
            max_collections_per_curator: None,
            denom: DENOM.to_string(),
        };
        instantiate(deps, mock_env(), info.clone(), msg).unwrap();
        info
    }

    fn create_collection(deps: DepsMut, curator: &Addr) -> u64 {
        let info = message_info(curator, &[Coin::new(MIN_BOND, DENOM)]);
        let msg = ExecuteMsg::CreateCollection {
            name: "Top Carbon Batches".to_string(),
            criteria: "Verified carbon removal credits with >90% permanence".to_string(),
        };
        let res = execute(deps, mock_env(), info, msg).unwrap();
        res.attributes
            .iter()
            .find(|a| a.key == "collection_id")
            .unwrap()
            .value
            .parse()
            .unwrap()
    }

    fn activate_collection(deps: DepsMut, curator: &Addr, collection_id: u64) {
        let info = message_info(curator, &[]);
        execute(
            deps,
            mock_env(),
            info,
            ExecuteMsg::ActivateCollection { collection_id },
        )
        .unwrap();
    }

    fn submit_quality_score(deps: DepsMut, admin: &Addr, batch_denom: &str, score: u32) {
        let info = message_info(admin, &[]);
        execute(
            deps,
            mock_env(),
            info,
            ExecuteMsg::SubmitQualityScore {
                batch_denom: batch_denom.to_string(),
                score,
                confidence: 800,
            },
        )
        .unwrap();
    }

    // ── Test 1: Instantiate ───────────────────────────────────────────

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let info = setup_contract(deps.as_mut());

        let config: ConfigResponse =
            cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap())
                .unwrap();

        assert_eq!(config.admin, info.sender.to_string());
        assert_eq!(config.min_curation_bond, Uint128::new(MIN_BOND));
        assert_eq!(config.curation_fee_rate_bps, 50);
        assert_eq!(config.challenge_deposit, Uint128::new(CHALLENGE_DEP));
        assert_eq!(config.slash_percentage_bps, 2000);
        assert_eq!(config.activation_delay_seconds, 0); // overridden for tests
        assert_eq!(config.unbonding_period_seconds, 0);
        assert_eq!(config.bond_top_up_window_seconds, 604_800);
        assert_eq!(config.min_quality_score, 300);
        assert_eq!(config.max_collections_per_curator, 5);
        assert_eq!(config.denom, DENOM);
    }

    // ── Test 2: Create + Activate Collection ──────────────────────────

    #[test]
    fn test_create_and_activate_collection() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let curator = addr("curator1");
        let id = create_collection(deps.as_mut(), &curator);
        assert_eq!(id, 1);

        // Query collection — should be Proposed
        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection { collection_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.status, CollectionStatus::Proposed);
        assert_eq!(resp.collection.curator, curator);
        assert_eq!(resp.collection.bond_amount, Uint128::new(MIN_BOND));
        assert!(resp.collection.batches.is_empty());

        // Activate (delay=0 in test config)
        activate_collection(deps.as_mut(), &curator, 1);

        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection { collection_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.status, CollectionStatus::Active);

        // Curator stats
        let stats: CuratorStatsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::CuratorStats {
                    curator: curator.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(stats.collection_count, 1);
        assert_eq!(stats.max_collections, 5);
    }

    // ── Test 3: Add Batch to Collection ───────────────────────────────

    #[test]
    fn test_add_batch_to_collection() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let curator = addr("curator1");
        let batch = "C01-001-20250101-20251231-001";

        // Create + activate collection
        let id = create_collection(deps.as_mut(), &curator);
        activate_collection(deps.as_mut(), &curator, id);

        // Submit quality score for the batch
        submit_quality_score(deps.as_mut(), &admin, batch, 500);

        // Add batch
        let info = message_info(&curator, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::AddToCollection {
                collection_id: id,
                batch_denom: batch.to_string(),
            },
        )
        .unwrap();

        // Verify batch is in collection
        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection { collection_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.batches.len(), 1);
        assert_eq!(resp.collection.batches[0], batch);

        // Adding batch with score below minimum should fail
        let low_batch = "C01-001-20250101-20251231-002";
        submit_quality_score(deps.as_mut(), &admin, low_batch, 200);

        let info = message_info(&curator, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::AddToCollection {
                collection_id: id,
                batch_denom: low_batch.to_string(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::QualityScoreTooLow { .. }));

        // Adding batch with no quality score should fail
        let unknown_batch = "C01-001-20250101-20251231-003";
        let info = message_info(&curator, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::AddToCollection {
                collection_id: id,
                batch_denom: unknown_batch.to_string(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::NoQualityScore { .. }));
    }

    // ── Test 4: Challenge + Resolve ───────────────────────────────────

    #[test]
    fn test_challenge_and_resolve() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let curator = addr("curator1");
        let challenger = addr("challenger1");
        let batch = "C01-001-20250101-20251231-001";

        // Create + activate + add batch
        let coll_id = create_collection(deps.as_mut(), &curator);
        activate_collection(deps.as_mut(), &curator, coll_id);
        submit_quality_score(deps.as_mut(), &admin, batch, 500);

        let info = message_info(&curator, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::AddToCollection {
                collection_id: coll_id,
                batch_denom: batch.to_string(),
            },
        )
        .unwrap();

        // File challenge
        let info = message_info(&challenger, &[Coin::new(CHALLENGE_DEP, DENOM)]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ChallengeBatchInclusion {
                collection_id: coll_id,
                batch_denom: batch.to_string(),
                evidence: "Batch credits are from a revoked project".to_string(),
            },
        )
        .unwrap();

        let challenge_id: u64 = res
            .attributes
            .iter()
            .find(|a| a.key == "challenge_id")
            .unwrap()
            .value
            .parse()
            .unwrap();
        assert_eq!(challenge_id, 1);

        // Collection should be UnderReview
        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.status, CollectionStatus::UnderReview);

        // Resolve: CuratorWins — challenger loses deposit
        let info = message_info(&admin, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ResolveChallenge {
                challenge_id,
                resolution: ChallengeResolution::CuratorWins,
            },
        )
        .unwrap();

        // Should have one bank message (deposit to community)
        assert_eq!(res.messages.len(), 1);

        // Collection back to Active
        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.status, CollectionStatus::Active);
        // Bond unchanged
        assert_eq!(resp.collection.bond_amount, Uint128::new(MIN_BOND));
    }

    // ── Test 5: Challenge ChallengerWins — slash + suspend ────────────

    #[test]
    fn test_challenge_challenger_wins_slash() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let curator = addr("curator1");
        let challenger = addr("challenger1");
        let batch = "C01-001-20250101-20251231-001";

        let coll_id = create_collection(deps.as_mut(), &curator);
        activate_collection(deps.as_mut(), &curator, coll_id);
        submit_quality_score(deps.as_mut(), &admin, batch, 500);

        let info = message_info(&curator, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::AddToCollection {
                collection_id: coll_id,
                batch_denom: batch.to_string(),
            },
        )
        .unwrap();

        // File challenge
        let info = message_info(&challenger, &[Coin::new(CHALLENGE_DEP, DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ChallengeBatchInclusion {
                collection_id: coll_id,
                batch_denom: batch.to_string(),
                evidence: "Invalid methodology".to_string(),
            },
        )
        .unwrap();

        // Resolve: ChallengerWins
        let info = message_info(&admin, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ResolveChallenge {
                challenge_id: 1,
                resolution: ChallengeResolution::ChallengerWins,
            },
        )
        .unwrap();

        // Should have two bank messages: challenger gets deposit + slash share, community gets slash share
        assert_eq!(res.messages.len(), 2);

        // Bond should be slashed by 20%
        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // 1_000_000_000 * 20% = 200_000_000 slashed, remaining = 800_000_000
        assert_eq!(
            resp.collection.bond_amount,
            Uint128::new(800_000_000)
        );
        // Bond below min (1B) so should be Suspended
        assert_eq!(resp.collection.status, CollectionStatus::Suspended);
        assert!(resp.collection.suspension_expires_at.is_some());

        // Bond status query
        let bond: BondStatusResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BondStatus {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!bond.is_sufficient);
        assert_eq!(bond.bond_amount, Uint128::new(800_000_000));
    }

    // ── Test 6: Quality Score Auto-Removal ────────────────────────────

    #[test]
    fn test_quality_score_auto_removal() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let curator = addr("curator1");
        let batch = "C01-001-20250101-20251231-001";

        let coll_id = create_collection(deps.as_mut(), &curator);
        activate_collection(deps.as_mut(), &curator, coll_id);

        // Submit good score, add batch
        submit_quality_score(deps.as_mut(), &admin, batch, 500);
        let info = message_info(&curator, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::AddToCollection {
                collection_id: coll_id,
                batch_denom: batch.to_string(),
            },
        )
        .unwrap();

        // Verify batch is in collection
        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.batches.len(), 1);

        // Now submit a score below minimum (300) — should auto-remove
        submit_quality_score(deps.as_mut(), &admin, batch, 100);

        // Batch should be removed
        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(resp.collection.batches.is_empty());

        // Quality score query should return the updated score
        let qs: QualityScoreResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QualityScore {
                    batch_denom: batch.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(qs.quality_score.unwrap().score, 100);
    }

    // ── Test 7: Max collections per curator ───────────────────────────

    #[test]
    fn test_max_collections_per_curator() {
        let mut deps = mock_dependencies();

        // Setup with max 2 collections
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            community_pool: addr("community_pool").to_string(),
            min_curation_bond: None,
            curation_fee_rate_bps: None,
            challenge_deposit: None,
            slash_percentage_bps: None,
            activation_delay_seconds: Some(0),
            unbonding_period_seconds: Some(0),
            bond_top_up_window_seconds: None,
            min_quality_score: None,
            max_collections_per_curator: Some(2),
            denom: DENOM.to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let curator = addr("curator1");
        create_collection(deps.as_mut(), &curator);
        create_collection(deps.as_mut(), &curator);

        // Third should fail
        let info = message_info(&curator, &[Coin::new(MIN_BOND, DENOM)]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CreateCollection {
                name: "Third".to_string(),
                criteria: "Too many".to_string(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::MaxCollectionsExceeded { max: 2 }));
    }

    // ── Test 8: Activation delay enforcement ──────────────────────────

    #[test]
    fn test_activation_delay_enforcement() {
        let mut deps = mock_dependencies();
        setup_contract_with_delays(deps.as_mut());

        let curator = addr("curator1");
        let coll_id = create_collection(deps.as_mut(), &curator);

        // Try activating immediately — should fail
        let info = message_info(&curator, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ActivateCollection {
                collection_id: coll_id,
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::ActivationDelayNotElapsed));

        // Advance time past 48h
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(172_801);

        let info = message_info(&curator, &[]);
        execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::ActivateCollection {
                collection_id: coll_id,
            },
        )
        .unwrap();

        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.status, CollectionStatus::Active);
    }

    // ── Test 9: Close + withdraw bond ─────────────────────────────────

    #[test]
    fn test_close_and_withdraw_bond() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut()); // unbonding = 0 for easy testing

        let curator = addr("curator1");
        let coll_id = create_collection(deps.as_mut(), &curator);
        activate_collection(deps.as_mut(), &curator, coll_id);

        // Close collection
        let info = message_info(&curator, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CloseCollection {
                collection_id: coll_id,
            },
        )
        .unwrap();

        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.status, CollectionStatus::Closed);

        // Withdraw bond
        let info = message_info(&curator, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::WithdrawBond {
                collection_id: coll_id,
            },
        )
        .unwrap();

        // Should send bond back
        assert_eq!(res.messages.len(), 1);

        // Curator count should decrement
        let stats: CuratorStatsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::CuratorStats {
                    curator: curator.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(stats.collection_count, 0);
    }

    // ── Test 10: Top up bond restores active ──────────────────────────

    #[test]
    fn test_top_up_bond_restores_active() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let admin = addr("admin");
        let curator = addr("curator1");
        let challenger = addr("challenger1");
        let batch = "C01-001-20250101-20251231-001";

        let coll_id = create_collection(deps.as_mut(), &curator);
        activate_collection(deps.as_mut(), &curator, coll_id);
        submit_quality_score(deps.as_mut(), &admin, batch, 500);

        let info = message_info(&curator, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::AddToCollection {
                collection_id: coll_id,
                batch_denom: batch.to_string(),
            },
        )
        .unwrap();

        // Challenge + ChallengerWins to get it suspended
        let info = message_info(&challenger, &[Coin::new(CHALLENGE_DEP, DENOM)]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ChallengeBatchInclusion {
                collection_id: coll_id,
                batch_denom: batch.to_string(),
                evidence: "Bad batch".to_string(),
            },
        )
        .unwrap();

        let info = message_info(&admin, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ResolveChallenge {
                challenge_id: 1,
                resolution: ChallengeResolution::ChallengerWins,
            },
        )
        .unwrap();

        // Confirm suspended
        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.status, CollectionStatus::Suspended);

        // Top up with enough to restore (need 200M more to reach 1B)
        let info = message_info(&curator, &[Coin::new(200_000_000u128, DENOM)]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::TopUpBond {
                collection_id: coll_id,
            },
        )
        .unwrap();

        // Should be Active again
        let status_attr = res
            .attributes
            .iter()
            .find(|a| a.key == "status")
            .unwrap();
        assert_eq!(status_attr.value, "Active");

        let resp: CollectionResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Collection {
                    collection_id: coll_id,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.collection.status, CollectionStatus::Active);
        assert_eq!(resp.collection.bond_amount, Uint128::new(1_000_000_000));
    }
}
