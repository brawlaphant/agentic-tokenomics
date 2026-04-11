use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    AgreementResponse, AgreementsResponse, ConfigResponse, DisputeResponse, EscrowBalanceResponse,
    ExecuteMsg, InstantiateMsg, MilestoneInput, MilestonesResponse, QueryMsg,
};
use crate::state::{
    AgreementStatus, Config, Dispute, DisputeResolution, Milestone, MilestoneStatus,
    ServiceAgreement, AGREEMENTS, CONFIG, DISPUTES, NEXT_AGREEMENT_ID,
};

const CONTRACT_NAME: &str = "crates.io:service-escrow";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

// Governance parameter bounds (basis points)
const MIN_BOND_RATIO: u64 = 500; // 5%
const MAX_BOND_RATIO: u64 = 2500; // 25%
const MIN_PLATFORM_FEE: u64 = 0;
const MAX_PLATFORM_FEE: u64 = 500; // 5%
const MIN_CANCEL_FEE: u64 = 0;
const MAX_CANCEL_FEE: u64 = 1000; // 10%
const MIN_ARBITER_FEE: u64 = 100; // 1%
const MAX_ARBITER_FEE: u64 = 1500; // 15%

// ── Instantiate ────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let bond_ratio = msg.provider_bond_ratio_bps.unwrap_or(1000);
    let platform_fee = msg.platform_fee_rate_bps.unwrap_or(100);
    let cancel_fee = msg.cancellation_fee_rate_bps.unwrap_or(200);
    let arbiter_fee = msg.arbiter_fee_rate_bps.unwrap_or(500);

    validate_bond_ratio(bond_ratio)?;
    validate_fee_rate(platform_fee, MIN_PLATFORM_FEE, MAX_PLATFORM_FEE, "platform")?;
    validate_fee_rate(cancel_fee, MIN_CANCEL_FEE, MAX_CANCEL_FEE, "cancellation")?;
    validate_fee_rate(arbiter_fee, MIN_ARBITER_FEE, MAX_ARBITER_FEE, "arbiter")?;

    let config = Config {
        admin: info.sender.clone(),
        arbiter_dao: deps.api.addr_validate(&msg.arbiter_dao)?,
        community_pool: deps.api.addr_validate(&msg.community_pool)?,
        provider_bond_ratio_bps: bond_ratio,
        platform_fee_rate_bps: platform_fee,
        cancellation_fee_rate_bps: cancel_fee,
        arbiter_fee_rate_bps: arbiter_fee,
        review_period_seconds: msg.review_period_seconds.unwrap_or(1_209_600), // 14 days
        max_milestones: msg.max_milestones.unwrap_or(20),
        max_revisions: msg.max_revisions.unwrap_or(3),
        denom: msg.denom,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    NEXT_AGREEMENT_ID.save(deps.storage, &1u64)?;

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
        ExecuteMsg::ProposeAgreement {
            provider,
            service_type,
            description,
            milestones,
        } => execute_propose(deps, env, info, provider, service_type, description, milestones),
        ExecuteMsg::AcceptAgreement { agreement_id } => {
            execute_accept(deps, env, info, agreement_id)
        }
        ExecuteMsg::FundAgreement { agreement_id } => {
            execute_fund(deps, env, info, agreement_id)
        }
        ExecuteMsg::StartAgreement { agreement_id } => {
            execute_start(deps, env, info, agreement_id)
        }
        ExecuteMsg::SubmitMilestone {
            agreement_id,
            milestone_index,
            deliverable_iri,
        } => execute_submit_milestone(deps, env, info, agreement_id, milestone_index, deliverable_iri),
        ExecuteMsg::ApproveMilestone {
            agreement_id,
            milestone_index,
        } => execute_approve_milestone(deps, env, info, agreement_id, milestone_index),
        ExecuteMsg::ReviseMilestone {
            agreement_id,
            milestone_index,
            deliverable_iri,
        } => execute_revise_milestone(deps, env, info, agreement_id, milestone_index, deliverable_iri),
        ExecuteMsg::DisputeMilestone {
            agreement_id,
            milestone_index,
            reason,
        } => execute_dispute_milestone(deps, env, info, agreement_id, milestone_index, reason),
        ExecuteMsg::ResolveDispute {
            agreement_id,
            resolution,
        } => execute_resolve_dispute(deps, env, info, agreement_id, resolution),
        ExecuteMsg::CancelAgreement { agreement_id } => {
            execute_cancel(deps, env, info, agreement_id)
        }
        ExecuteMsg::UpdateConfig {
            arbiter_dao,
            community_pool,
            provider_bond_ratio_bps,
            platform_fee_rate_bps,
            cancellation_fee_rate_bps,
            arbiter_fee_rate_bps,
            review_period_seconds,
            max_milestones,
            max_revisions,
        } => execute_update_config(
            deps, info, arbiter_dao, community_pool, provider_bond_ratio_bps,
            platform_fee_rate_bps, cancellation_fee_rate_bps, arbiter_fee_rate_bps,
            review_period_seconds, max_milestones, max_revisions,
        ),
    }
}

fn execute_propose(
    deps: DepsMut, env: Env, info: MessageInfo,
    provider: String, service_type: String, description: String,
    milestones: Vec<MilestoneInput>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let provider_addr = deps.api.addr_validate(&provider)?;

    if info.sender == provider_addr {
        return Err(ContractError::SelfAgreement);
    }

    let ms_count = milestones.len() as u32;
    if ms_count == 0 || ms_count > config.max_milestones {
        return Err(ContractError::InvalidMilestoneCount {
            max: config.max_milestones, got: ms_count,
        });
    }

    let total_escrow: Uint128 = milestones.iter().map(|m| m.payment_amount).sum();
    if total_escrow.is_zero() {
        return Err(ContractError::InsufficientFunds {
            required: "non-zero".to_string(), sent: "0".to_string(),
        });
    }

    let provider_bond = total_escrow.multiply_ratio(config.provider_bond_ratio_bps, 10_000u128);
    let id = NEXT_AGREEMENT_ID.load(deps.storage)?;

    let ms: Vec<Milestone> = milestones.iter().enumerate().map(|(i, m)| Milestone {
        index: i as u32, description: m.description.clone(), payment: m.payment_amount,
        status: MilestoneStatus::Pending, deliverable_iri: None,
        submitted_at: None, approved_at: None, revision_count: 0,
    }).collect();

    let agreement = ServiceAgreement {
        id, client: info.sender.clone(), provider: provider_addr,
        service_type, description, escrow_amount: total_escrow, provider_bond,
        milestones: ms, current_milestone: 0, status: AgreementStatus::Proposed,
        created_at: env.block.time, funded_at: None, started_at: None,
        completed_at: None, provider_accepted: false, client_funded: false,
        total_released: Uint128::zero(), total_fees: Uint128::zero(),
    };

    AGREEMENTS.save(deps.storage, id, &agreement)?;
    NEXT_AGREEMENT_ID.save(deps.storage, &(id + 1))?;

    Ok(Response::new()
        .add_attribute("action", "propose_agreement")
        .add_attribute("agreement_id", id.to_string())
        .add_attribute("client", info.sender)
        .add_attribute("escrow_amount", total_escrow)
        .add_attribute("provider_bond", provider_bond))
}

fn execute_accept(
    deps: DepsMut, env: Env, info: MessageInfo, agreement_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    if info.sender != agreement.provider {
        return Err(ContractError::Unauthorized {
            reason: "Only the designated provider can accept".to_string(),
        });
    }
    if agreement.status != AgreementStatus::Proposed {
        return Err(ContractError::InvalidStatus {
            expected: "Proposed".to_string(), actual: agreement.status.to_string(),
        });
    }
    if agreement.provider_accepted {
        return Err(ContractError::InvalidStatus {
            expected: "not yet accepted".to_string(), actual: "already accepted".to_string(),
        });
    }

    let bond_coin = must_pay(&info, &config.denom)?;
    if bond_coin < agreement.provider_bond {
        return Err(ContractError::InsufficientFunds {
            required: agreement.provider_bond.to_string(), sent: bond_coin.to_string(),
        });
    }

    agreement.provider_accepted = true;
    if agreement.client_funded {
        agreement.status = AgreementStatus::Funded;
        agreement.funded_at = Some(env.block.time);
    }

    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;

    Ok(Response::new()
        .add_attribute("action", "accept_agreement")
        .add_attribute("agreement_id", agreement_id.to_string())
        .add_attribute("bond_posted", bond_coin))
}

fn execute_fund(
    deps: DepsMut, env: Env, info: MessageInfo, agreement_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    if info.sender != agreement.client {
        return Err(ContractError::Unauthorized {
            reason: "Only the client can fund the escrow".to_string(),
        });
    }
    if agreement.status != AgreementStatus::Proposed {
        return Err(ContractError::InvalidStatus {
            expected: "Proposed".to_string(), actual: agreement.status.to_string(),
        });
    }
    if agreement.client_funded {
        return Err(ContractError::InvalidStatus {
            expected: "not yet funded".to_string(), actual: "already funded".to_string(),
        });
    }

    let paid = must_pay(&info, &config.denom)?;
    if paid < agreement.escrow_amount {
        return Err(ContractError::InsufficientFunds {
            required: agreement.escrow_amount.to_string(), sent: paid.to_string(),
        });
    }

    agreement.client_funded = true;
    if agreement.provider_accepted {
        agreement.status = AgreementStatus::Funded;
        agreement.funded_at = Some(env.block.time);
    }

    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;

    Ok(Response::new()
        .add_attribute("action", "fund_agreement")
        .add_attribute("agreement_id", agreement_id.to_string())
        .add_attribute("escrow_funded", paid))
}

fn execute_start(
    deps: DepsMut, env: Env, info: MessageInfo, agreement_id: u64,
) -> Result<Response, ContractError> {
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    if info.sender != agreement.client && info.sender != agreement.provider {
        return Err(ContractError::Unauthorized {
            reason: "Only client or provider can start the agreement".to_string(),
        });
    }
    if agreement.status != AgreementStatus::Funded {
        return Err(ContractError::InvalidStatus {
            expected: "Funded".to_string(), actual: agreement.status.to_string(),
        });
    }

    agreement.status = AgreementStatus::InProgress;
    agreement.started_at = Some(env.block.time);
    if !agreement.milestones.is_empty() {
        agreement.milestones[0].status = MilestoneStatus::InProgress;
    }

    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;

    Ok(Response::new()
        .add_attribute("action", "start_agreement")
        .add_attribute("agreement_id", agreement_id.to_string()))
}

fn execute_submit_milestone(
    deps: DepsMut, env: Env, info: MessageInfo,
    agreement_id: u64, milestone_index: u32, deliverable_iri: String,
) -> Result<Response, ContractError> {
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    if info.sender != agreement.provider {
        return Err(ContractError::Unauthorized {
            reason: "Only the provider can submit milestones".to_string(),
        });
    }
    if agreement.status != AgreementStatus::InProgress {
        return Err(ContractError::InvalidStatus {
            expected: "InProgress".to_string(), actual: agreement.status.to_string(),
        });
    }
    if milestone_index != agreement.current_milestone {
        return Err(ContractError::InvalidMilestoneIndex {
            expected: agreement.current_milestone, got: milestone_index,
        });
    }
    let ms = &agreement.milestones[milestone_index as usize];
    if ms.status != MilestoneStatus::InProgress {
        return Err(ContractError::InvalidMilestoneStatus {
            index: milestone_index, expected_status: "InProgress".to_string(),
        });
    }

    agreement.milestones[milestone_index as usize].status = MilestoneStatus::Submitted;
    agreement.milestones[milestone_index as usize].deliverable_iri = Some(deliverable_iri.clone());
    agreement.milestones[milestone_index as usize].submitted_at = Some(env.block.time);
    agreement.status = AgreementStatus::MilestoneReview;

    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;

    Ok(Response::new()
        .add_attribute("action", "submit_milestone")
        .add_attribute("agreement_id", agreement_id.to_string())
        .add_attribute("milestone_index", milestone_index.to_string())
        .add_attribute("deliverable_iri", deliverable_iri))
}

fn execute_approve_milestone(
    deps: DepsMut, env: Env, info: MessageInfo,
    agreement_id: u64, milestone_index: u32,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    if info.sender != agreement.client {
        return Err(ContractError::Unauthorized {
            reason: "Only the client can approve milestones".to_string(),
        });
    }
    if agreement.status != AgreementStatus::MilestoneReview {
        return Err(ContractError::InvalidStatus {
            expected: "MilestoneReview".to_string(), actual: agreement.status.to_string(),
        });
    }
    if milestone_index != agreement.current_milestone {
        return Err(ContractError::InvalidMilestoneIndex {
            expected: agreement.current_milestone, got: milestone_index,
        });
    }
    if agreement.milestones[milestone_index as usize].status != MilestoneStatus::Submitted {
        return Err(ContractError::InvalidMilestoneStatus {
            index: milestone_index, expected_status: "Submitted".to_string(),
        });
    }

    let milestone_payment = agreement.milestones[milestone_index as usize].payment;
    let platform_fee = milestone_payment.multiply_ratio(config.platform_fee_rate_bps, 10_000u128);
    let provider_payment = milestone_payment - platform_fee;

    agreement.milestones[milestone_index as usize].status = MilestoneStatus::Approved;
    agreement.milestones[milestone_index as usize].approved_at = Some(env.block.time);
    agreement.total_released += provider_payment;
    agreement.total_fees += platform_fee;

    let mut msgs = vec![];

    if !provider_payment.is_zero() {
        msgs.push(BankMsg::Send {
            to_address: agreement.provider.to_string(),
            amount: vec![Coin { denom: config.denom.clone(), amount: provider_payment }],
        });
    }
    if !platform_fee.is_zero() {
        msgs.push(BankMsg::Send {
            to_address: config.community_pool.to_string(),
            amount: vec![Coin { denom: config.denom.clone(), amount: platform_fee }],
        });
    }

    let next_idx = milestone_index + 1;
    if next_idx >= agreement.milestones.len() as u32 {
        agreement.status = AgreementStatus::Completed;
        agreement.completed_at = Some(env.block.time);

        if !agreement.provider_bond.is_zero() {
            msgs.push(BankMsg::Send {
                to_address: agreement.provider.to_string(),
                amount: vec![Coin { denom: config.denom.clone(), amount: agreement.provider_bond }],
            });
        }
        let completion_fee = agreement.escrow_amount.multiply_ratio(config.platform_fee_rate_bps, 10_000u128);
        if !completion_fee.is_zero() {
            msgs.push(BankMsg::Send {
                to_address: config.community_pool.to_string(),
                amount: vec![Coin { denom: config.denom.clone(), amount: completion_fee }],
            });
            agreement.total_fees += completion_fee;
        }
    } else {
        agreement.current_milestone = next_idx;
        agreement.milestones[next_idx as usize].status = MilestoneStatus::InProgress;
        agreement.status = AgreementStatus::InProgress;
    }

    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;

    let mut resp = Response::new()
        .add_attribute("action", "approve_milestone")
        .add_attribute("agreement_id", agreement_id.to_string())
        .add_attribute("milestone_index", milestone_index.to_string())
        .add_attribute("provider_payment", provider_payment)
        .add_attribute("platform_fee", platform_fee);

    for msg in msgs { resp = resp.add_message(msg); }
    Ok(resp)
}

fn execute_revise_milestone(
    deps: DepsMut, env: Env, info: MessageInfo,
    agreement_id: u64, milestone_index: u32, deliverable_iri: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    if info.sender != agreement.provider {
        return Err(ContractError::Unauthorized {
            reason: "Only the provider can revise milestones".to_string(),
        });
    }
    if agreement.status != AgreementStatus::MilestoneReview {
        return Err(ContractError::InvalidStatus {
            expected: "MilestoneReview".to_string(), actual: agreement.status.to_string(),
        });
    }
    if milestone_index != agreement.current_milestone {
        return Err(ContractError::InvalidMilestoneIndex {
            expected: agreement.current_milestone, got: milestone_index,
        });
    }

    {
        let ms = &agreement.milestones[milestone_index as usize];
        if ms.status != MilestoneStatus::Submitted {
            return Err(ContractError::InvalidMilestoneStatus {
                index: milestone_index, expected_status: "Submitted".to_string(),
            });
        }
        if ms.revision_count >= config.max_revisions {
            return Err(ContractError::MaxRevisionsExceeded {
                max: config.max_revisions, index: milestone_index,
            });
        }
    }

    let idx = milestone_index as usize;
    agreement.milestones[idx].revision_count += 1;
    agreement.milestones[idx].deliverable_iri = Some(deliverable_iri.clone());
    agreement.milestones[idx].submitted_at = Some(env.block.time);

    let new_revision_count = agreement.milestones[idx].revision_count;

    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;

    Ok(Response::new()
        .add_attribute("action", "revise_milestone")
        .add_attribute("agreement_id", agreement_id.to_string())
        .add_attribute("milestone_index", milestone_index.to_string())
        .add_attribute("revision_count", new_revision_count.to_string())
        .add_attribute("deliverable_iri", deliverable_iri))
}

fn execute_dispute_milestone(
    deps: DepsMut, env: Env, info: MessageInfo,
    agreement_id: u64, milestone_index: u32, reason: String,
) -> Result<Response, ContractError> {
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    if info.sender != agreement.client {
        return Err(ContractError::Unauthorized {
            reason: "Only the client can raise a dispute".to_string(),
        });
    }
    if agreement.status != AgreementStatus::MilestoneReview {
        return Err(ContractError::InvalidStatus {
            expected: "MilestoneReview".to_string(), actual: agreement.status.to_string(),
        });
    }
    if milestone_index != agreement.current_milestone {
        return Err(ContractError::InvalidMilestoneIndex {
            expected: agreement.current_milestone, got: milestone_index,
        });
    }
    if DISPUTES.may_load(deps.storage, agreement_id)?.is_some() {
        return Err(ContractError::DisputeAlreadyExists);
    }

    agreement.milestones[milestone_index as usize].status = MilestoneStatus::Disputed;
    agreement.status = AgreementStatus::Disputed;

    let dispute = Dispute {
        agreement_id, milestone_index, reason: reason.clone(),
        raised_by: info.sender.clone(), raised_at: env.block.time,
        resolved_at: None, resolution: None,
    };

    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;
    DISPUTES.save(deps.storage, agreement_id, &dispute)?;

    Ok(Response::new()
        .add_attribute("action", "dispute_milestone")
        .add_attribute("agreement_id", agreement_id.to_string())
        .add_attribute("milestone_index", milestone_index.to_string())
        .add_attribute("reason", reason))
}

fn execute_resolve_dispute(
    deps: DepsMut, env: Env, info: MessageInfo,
    agreement_id: u64, resolution: DisputeResolution,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    if info.sender != config.arbiter_dao {
        return Err(ContractError::Unauthorized {
            reason: "Only the arbiter DAO can resolve disputes".to_string(),
        });
    }
    if agreement.status != AgreementStatus::Disputed {
        return Err(ContractError::InvalidStatus {
            expected: "Disputed".to_string(), actual: agreement.status.to_string(),
        });
    }

    let mut dispute = DISPUTES.load(deps.storage, agreement_id)
        .map_err(|_| ContractError::NoActiveDispute)?;

    if let DisputeResolution::Split { client_percent } = &resolution {
        if *client_percent == 0 || *client_percent >= 100 {
            return Err(ContractError::InvalidSplitPercent { got: *client_percent });
        }
    }

    let milestone_idx = dispute.milestone_index as usize;
    let disputed_amount = agreement.milestones[milestone_idx].payment;
    let arbiter_fee = disputed_amount.multiply_ratio(config.arbiter_fee_rate_bps, 10_000u128);

    let mut msgs = vec![];

    match &resolution {
        DisputeResolution::ClientWins => {
            let client_receives = disputed_amount - arbiter_fee;
            if !client_receives.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: agreement.client.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: client_receives }],
                });
            }
            let bond_half = agreement.provider_bond.multiply_ratio(1u128, 2u128);
            if !bond_half.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: agreement.client.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: bond_half }],
                });
                msgs.push(BankMsg::Send {
                    to_address: config.community_pool.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: bond_half }],
                });
            }
            if !arbiter_fee.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: config.community_pool.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: arbiter_fee }],
                });
            }
            let remaining = remaining_escrow(&agreement);
            if !remaining.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: agreement.client.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: remaining }],
                });
            }
            agreement.status = AgreementStatus::Completed;
            agreement.completed_at = Some(env.block.time);
        }
        DisputeResolution::ProviderWins => {
            let provider_receives = disputed_amount + agreement.provider_bond - arbiter_fee;
            if !provider_receives.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: agreement.provider.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: provider_receives }],
                });
            }
            if !arbiter_fee.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: config.community_pool.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: arbiter_fee }],
                });
            }
            let next_idx = dispute.milestone_index + 1;
            if next_idx < agreement.milestones.len() as u32 {
                agreement.current_milestone = next_idx;
                agreement.milestones[next_idx as usize].status = MilestoneStatus::InProgress;
                agreement.status = AgreementStatus::InProgress;
                agreement.provider_bond = Uint128::zero();
            } else {
                agreement.status = AgreementStatus::Completed;
                agreement.completed_at = Some(env.block.time);
            }
        }
        DisputeResolution::Split { client_percent } => {
            let client_share = disputed_amount.multiply_ratio(*client_percent as u128, 100u128);
            let provider_share = disputed_amount - client_share;
            let arbiter_fee_half = arbiter_fee.multiply_ratio(1u128, 2u128);

            let client_receives = client_share.saturating_sub(arbiter_fee_half);
            if !client_receives.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: agreement.client.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: client_receives }],
                });
            }
            let provider_receives = (provider_share + agreement.provider_bond).saturating_sub(arbiter_fee_half);
            if !provider_receives.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: agreement.provider.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: provider_receives }],
                });
            }
            if !arbiter_fee.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: config.community_pool.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: arbiter_fee }],
                });
            }
            let remaining = remaining_escrow(&agreement);
            if !remaining.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: agreement.client.to_string(),
                    amount: vec![Coin { denom: config.denom.clone(), amount: remaining }],
                });
            }
            agreement.status = AgreementStatus::Completed;
            agreement.completed_at = Some(env.block.time);
        }
    }

    agreement.milestones[milestone_idx].status = MilestoneStatus::Approved;
    dispute.resolved_at = Some(env.block.time);
    dispute.resolution = Some(resolution);

    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;
    DISPUTES.save(deps.storage, agreement_id, &dispute)?;

    let mut resp = Response::new()
        .add_attribute("action", "resolve_dispute")
        .add_attribute("agreement_id", agreement_id.to_string());
    for msg in msgs { resp = resp.add_message(msg); }
    Ok(resp)
}

fn execute_cancel(
    deps: DepsMut, env: Env, info: MessageInfo, agreement_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut agreement = load_agreement(deps.as_ref(), agreement_id)?;

    match agreement.status {
        AgreementStatus::Proposed => {
            if info.sender != agreement.client && info.sender != agreement.provider {
                return Err(ContractError::Unauthorized {
                    reason: "Only client or provider can cancel a proposed agreement".to_string(),
                });
            }
        }
        AgreementStatus::Funded => {
            if info.sender != agreement.client {
                return Err(ContractError::Unauthorized {
                    reason: "Only the client can cancel a funded agreement".to_string(),
                });
            }
        }
        _ => {
            return Err(ContractError::InvalidStatus {
                expected: "Proposed or Funded".to_string(), actual: agreement.status.to_string(),
            });
        }
    }

    let mut msgs = vec![];

    if agreement.status == AgreementStatus::Funded {
        let cancel_fee = agreement.escrow_amount.multiply_ratio(config.cancellation_fee_rate_bps, 10_000u128);
        let client_refund = agreement.escrow_amount - cancel_fee;

        if !client_refund.is_zero() {
            msgs.push(BankMsg::Send {
                to_address: agreement.client.to_string(),
                amount: vec![Coin { denom: config.denom.clone(), amount: client_refund }],
            });
        }
        if !cancel_fee.is_zero() {
            msgs.push(BankMsg::Send {
                to_address: config.community_pool.to_string(),
                amount: vec![Coin { denom: config.denom.clone(), amount: cancel_fee }],
            });
        }
        if !agreement.provider_bond.is_zero() {
            msgs.push(BankMsg::Send {
                to_address: agreement.provider.to_string(),
                amount: vec![Coin { denom: config.denom.clone(), amount: agreement.provider_bond }],
            });
        }
    } else {
        if agreement.client_funded {
            msgs.push(BankMsg::Send {
                to_address: agreement.client.to_string(),
                amount: vec![Coin { denom: config.denom.clone(), amount: agreement.escrow_amount }],
            });
        }
        if agreement.provider_accepted {
            msgs.push(BankMsg::Send {
                to_address: agreement.provider.to_string(),
                amount: vec![Coin { denom: config.denom.clone(), amount: agreement.provider_bond }],
            });
        }
    }

    agreement.status = AgreementStatus::Cancelled;
    agreement.completed_at = Some(env.block.time);
    AGREEMENTS.save(deps.storage, agreement_id, &agreement)?;

    let mut resp = Response::new()
        .add_attribute("action", "cancel_agreement")
        .add_attribute("agreement_id", agreement_id.to_string());
    for msg in msgs { resp = resp.add_message(msg); }
    Ok(resp)
}

fn execute_update_config(
    deps: DepsMut, info: MessageInfo,
    arbiter_dao: Option<String>, community_pool: Option<String>,
    provider_bond_ratio_bps: Option<u64>, platform_fee_rate_bps: Option<u64>,
    cancellation_fee_rate_bps: Option<u64>, arbiter_fee_rate_bps: Option<u64>,
    review_period_seconds: Option<u64>, max_milestones: Option<u32>, max_revisions: Option<u32>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "Only admin can update config".to_string(),
        });
    }

    if let Some(v) = arbiter_dao { config.arbiter_dao = deps.api.addr_validate(&v)?; }
    if let Some(v) = community_pool { config.community_pool = deps.api.addr_validate(&v)?; }
    if let Some(v) = provider_bond_ratio_bps { validate_bond_ratio(v)?; config.provider_bond_ratio_bps = v; }
    if let Some(v) = platform_fee_rate_bps { validate_fee_rate(v, MIN_PLATFORM_FEE, MAX_PLATFORM_FEE, "platform")?; config.platform_fee_rate_bps = v; }
    if let Some(v) = cancellation_fee_rate_bps { validate_fee_rate(v, MIN_CANCEL_FEE, MAX_CANCEL_FEE, "cancellation")?; config.cancellation_fee_rate_bps = v; }
    if let Some(v) = arbiter_fee_rate_bps { validate_fee_rate(v, MIN_ARBITER_FEE, MAX_ARBITER_FEE, "arbiter")?; config.arbiter_fee_rate_bps = v; }
    if let Some(v) = review_period_seconds { config.review_period_seconds = v; }
    if let Some(v) = max_milestones { config.max_milestones = v; }
    if let Some(v) = max_revisions { config.max_revisions = v; }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// ── Query ──────────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Agreement { agreement_id } => to_json_binary(&query_agreement(deps, agreement_id)?),
        QueryMsg::Agreements { status, start_after, limit } => to_json_binary(&query_agreements(deps, status, start_after, limit)?),
        QueryMsg::AgreementsByClient { client, start_after, limit } => to_json_binary(&query_agreements_by_client(deps, client, start_after, limit)?),
        QueryMsg::AgreementsByProvider { provider, start_after, limit } => to_json_binary(&query_agreements_by_provider(deps, provider, start_after, limit)?),
        QueryMsg::EscrowBalance { agreement_id } => to_json_binary(&query_escrow_balance(deps, agreement_id)?),
        QueryMsg::Milestones { agreement_id } => to_json_binary(&query_milestones(deps, agreement_id)?),
        QueryMsg::Dispute { agreement_id } => to_json_binary(&query_dispute(deps, agreement_id)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.to_string(), arbiter_dao: config.arbiter_dao.to_string(),
        community_pool: config.community_pool.to_string(),
        provider_bond_ratio_bps: config.provider_bond_ratio_bps,
        platform_fee_rate_bps: config.platform_fee_rate_bps,
        cancellation_fee_rate_bps: config.cancellation_fee_rate_bps,
        arbiter_fee_rate_bps: config.arbiter_fee_rate_bps,
        review_period_seconds: config.review_period_seconds,
        max_milestones: config.max_milestones, max_revisions: config.max_revisions,
        denom: config.denom,
    })
}

fn query_agreement(deps: Deps, agreement_id: u64) -> StdResult<AgreementResponse> {
    let agreement = AGREEMENTS.load(deps.storage, agreement_id)?;
    Ok(AgreementResponse { agreement })
}

fn query_agreements(deps: Deps, status: Option<AgreementStatus>, start_after: Option<u64>, limit: Option<u32>) -> StdResult<AgreementsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| s + 1).unwrap_or(0);
    let agreements: Vec<ServiceAgreement> = AGREEMENTS
        .range(deps.storage, Some(cw_storage_plus::Bound::inclusive(start)), None, Order::Ascending)
        .filter_map(|r| r.ok()).map(|(_, a)| a)
        .filter(|a| status.as_ref().is_none_or(|s| a.status == *s))
        .take(limit).collect();
    Ok(AgreementsResponse { agreements })
}

fn query_agreements_by_client(deps: Deps, client: String, start_after: Option<u64>, limit: Option<u32>) -> StdResult<AgreementsResponse> {
    let client_addr = deps.api.addr_validate(&client)?;
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| s + 1).unwrap_or(0);
    let agreements: Vec<ServiceAgreement> = AGREEMENTS
        .range(deps.storage, Some(cw_storage_plus::Bound::inclusive(start)), None, Order::Ascending)
        .filter_map(|r| r.ok()).map(|(_, a)| a)
        .filter(|a| a.client == client_addr)
        .take(limit).collect();
    Ok(AgreementsResponse { agreements })
}

fn query_agreements_by_provider(deps: Deps, provider: String, start_after: Option<u64>, limit: Option<u32>) -> StdResult<AgreementsResponse> {
    let provider_addr = deps.api.addr_validate(&provider)?;
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| s + 1).unwrap_or(0);
    let agreements: Vec<ServiceAgreement> = AGREEMENTS
        .range(deps.storage, Some(cw_storage_plus::Bound::inclusive(start)), None, Order::Ascending)
        .filter_map(|r| r.ok()).map(|(_, a)| a)
        .filter(|a| a.provider == provider_addr)
        .take(limit).collect();
    Ok(AgreementsResponse { agreements })
}

fn query_escrow_balance(deps: Deps, agreement_id: u64) -> StdResult<EscrowBalanceResponse> {
    let agreement = AGREEMENTS.load(deps.storage, agreement_id)?;
    let config = CONFIG.load(deps.storage)?;
    let remaining = agreement.escrow_amount.saturating_sub(agreement.total_released).saturating_sub(agreement.total_fees);
    Ok(EscrowBalanceResponse {
        agreement_id, escrow_amount: agreement.escrow_amount,
        provider_bond: agreement.provider_bond, total_released: agreement.total_released,
        total_fees: agreement.total_fees, remaining_escrow: remaining, denom: config.denom,
    })
}

fn query_milestones(deps: Deps, agreement_id: u64) -> StdResult<MilestonesResponse> {
    let agreement = AGREEMENTS.load(deps.storage, agreement_id)?;
    Ok(MilestonesResponse { agreement_id, milestones: agreement.milestones, current_milestone: agreement.current_milestone })
}

fn query_dispute(deps: Deps, agreement_id: u64) -> StdResult<DisputeResponse> {
    let dispute = DISPUTES.may_load(deps.storage, agreement_id)?;
    Ok(DisputeResponse { dispute })
}

// ── Helpers ────────────────────────────────────────────────────────────

fn load_agreement(deps: Deps, id: u64) -> Result<ServiceAgreement, ContractError> {
    AGREEMENTS.may_load(deps.storage, id)?.ok_or(ContractError::AgreementNotFound { id })
}

fn must_pay(info: &MessageInfo, expected_denom: &str) -> Result<Uint128, ContractError> {
    if info.funds.len() != 1 {
        return Err(ContractError::InsufficientFunds {
            required: format!("exactly one coin in {}", expected_denom),
            sent: format!("{} coins", info.funds.len()),
        });
    }
    let coin = &info.funds[0];
    if coin.denom != expected_denom {
        return Err(ContractError::WrongDenom { expected: expected_denom.to_string(), got: coin.denom.clone() });
    }
    Ok(coin.amount)
}

fn remaining_escrow(agreement: &ServiceAgreement) -> Uint128 {
    agreement.milestones.iter()
        .filter(|m| m.status != MilestoneStatus::Approved && m.status != MilestoneStatus::Disputed)
        .map(|m| m.payment).sum()
}

fn validate_bond_ratio(value: u64) -> Result<(), ContractError> {
    if value < MIN_BOND_RATIO || value > MAX_BOND_RATIO {
        return Err(ContractError::BondRatioOutOfRange { value, min: MIN_BOND_RATIO, max: MAX_BOND_RATIO });
    }
    Ok(())
}

fn validate_fee_rate(value: u64, min: u64, max: u64, _name: &str) -> Result<(), ContractError> {
    if value < min || value > max {
        return Err(ContractError::FeeRateOutOfRange { value, min, max });
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
    use cosmwasm_std::{Addr, Coin, Uint128};
    use crate::msg::MilestoneInput;

    const DENOM: &str = "uregen";

    fn addr(input: &str) -> Addr {
        MockApi::default().addr_make(input)
    }

    fn setup_contract(deps: DepsMut) -> MessageInfo {
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            arbiter_dao: addr("arbiter_dao").to_string(),
            community_pool: addr("community_pool").to_string(),
            provider_bond_ratio_bps: None,
            platform_fee_rate_bps: None,
            cancellation_fee_rate_bps: None,
            arbiter_fee_rate_bps: None,
            review_period_seconds: None,
            max_milestones: None,
            max_revisions: None,
            denom: DENOM.to_string(),
        };
        instantiate(deps, mock_env(), info.clone(), msg).unwrap();
        info
    }

    fn milestones_3() -> Vec<MilestoneInput> {
        vec![
            MilestoneInput { description: "Phase 1: Assessment".to_string(), payment_amount: Uint128::new(3000) },
            MilestoneInput { description: "Phase 2: Implementation".to_string(), payment_amount: Uint128::new(5000) },
            MilestoneInput { description: "Phase 3: Final Report".to_string(), payment_amount: Uint128::new(2000) },
        ]
    }

    fn propose_agreement(deps: DepsMut, client: &Addr, provider: &Addr) -> u64 {
        let info = message_info(client, &[]);
        let msg = ExecuteMsg::ProposeAgreement {
            provider: provider.to_string(),
            service_type: "ProjectVerification".to_string(),
            description: "Verify carbon credits".to_string(),
            milestones: milestones_3(),
        };
        let res = execute(deps, mock_env(), info, msg).unwrap();
        res.attributes.iter().find(|a| a.key == "agreement_id").unwrap().value.parse().unwrap()
    }

    fn fund_and_accept(deps: DepsMut, agreement_id: u64, _client: &Addr, provider: &Addr) {
        let accept_info = message_info(provider, &[Coin::new(1000u128, DENOM)]);
        execute(deps, mock_env(), accept_info, ExecuteMsg::AcceptAgreement { agreement_id }).unwrap();
    }

    fn fund_escrow(deps: DepsMut, agreement_id: u64, client: &Addr) {
        let fund_info = message_info(client, &[Coin::new(10000u128, DENOM)]);
        execute(deps, mock_env(), fund_info, ExecuteMsg::FundAgreement { agreement_id }).unwrap();
    }

    fn start_agreement(deps: DepsMut, agreement_id: u64, client: &Addr) {
        let info = message_info(client, &[]);
        execute(deps, mock_env(), info, ExecuteMsg::StartAgreement { agreement_id }).unwrap();
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let info = setup_contract(deps.as_mut());
        let config: ConfigResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.admin, info.sender.to_string());
        assert_eq!(config.provider_bond_ratio_bps, 1000);
        assert_eq!(config.platform_fee_rate_bps, 100);
        assert_eq!(config.cancellation_fee_rate_bps, 200);
        assert_eq!(config.arbiter_fee_rate_bps, 500);
        assert_eq!(config.review_period_seconds, 1_209_600);
        assert_eq!(config.max_milestones, 20);
        assert_eq!(config.max_revisions, 3);
        assert_eq!(config.denom, DENOM);
    }

    #[test]
    fn test_instantiate_custom_params() {
        let mut deps = mock_dependencies();
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            arbiter_dao: addr("arbiter").to_string(),
            community_pool: addr("pool").to_string(),
            provider_bond_ratio_bps: Some(1500), platform_fee_rate_bps: Some(200),
            cancellation_fee_rate_bps: Some(300), arbiter_fee_rate_bps: Some(1000),
            review_period_seconds: Some(604800), max_milestones: Some(10),
            max_revisions: Some(5), denom: "ustake".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        let config: ConfigResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.provider_bond_ratio_bps, 1500);
        assert_eq!(config.platform_fee_rate_bps, 200);
        assert_eq!(config.max_milestones, 10);
        assert_eq!(config.denom, "ustake");
    }

    #[test]
    fn test_instantiate_invalid_bond_ratio() {
        let mut deps = mock_dependencies();
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            arbiter_dao: addr("arbiter").to_string(), community_pool: addr("pool").to_string(),
            provider_bond_ratio_bps: Some(100), platform_fee_rate_bps: None,
            cancellation_fee_rate_bps: None, arbiter_fee_rate_bps: None,
            review_period_seconds: None, max_milestones: None, max_revisions: None,
            denom: DENOM.to_string(),
        };
        let err = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::BondRatioOutOfRange { .. }));
    }

    #[test]
    fn test_propose_agreement() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        assert_eq!(id, 1);
        let resp: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: 1 }).unwrap()).unwrap();
        let a = resp.agreement;
        assert_eq!(a.status, AgreementStatus::Proposed);
        assert_eq!(a.escrow_amount, Uint128::new(10000));
        assert_eq!(a.provider_bond, Uint128::new(1000));
        assert_eq!(a.milestones.len(), 3);
        assert_eq!(a.current_milestone, 0);
        assert!(!a.provider_accepted);
        assert!(!a.client_funded);
    }

    #[test]
    fn test_propose_self_agreement_fails() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let info = message_info(&client, &[]);
        let msg = ExecuteMsg::ProposeAgreement {
            provider: client.to_string(), service_type: "MRVSetup".to_string(),
            description: "Self deal".to_string(), milestones: milestones_3(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::SelfAgreement));
    }

    #[test]
    fn test_propose_too_many_milestones() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let info = message_info(&client, &[]);
        let many: Vec<MilestoneInput> = (0..21).map(|i| MilestoneInput { description: format!("M{}", i), payment_amount: Uint128::new(100) }).collect();
        let msg = ExecuteMsg::ProposeAgreement { provider: provider.to_string(), service_type: "MRVSetup".to_string(), description: "Too many".to_string(), milestones: many };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::InvalidMilestoneCount { max: 20, got: 21 }));
    }

    #[test]
    fn test_propose_zero_milestones() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let info = message_info(&client, &[]);
        let msg = ExecuteMsg::ProposeAgreement { provider: provider.to_string(), service_type: "MRVSetup".to_string(), description: "None".to_string(), milestones: vec![] };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::InvalidMilestoneCount { .. }));
    }

    #[test]
    fn test_accept_agreement() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        let accept_info = message_info(&provider, &[Coin::new(1000u128, DENOM)]);
        execute(deps.as_mut(), mock_env(), accept_info, ExecuteMsg::AcceptAgreement { agreement_id: id }).unwrap();
        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert!(a.agreement.provider_accepted);
        assert_eq!(a.agreement.status, AgreementStatus::Proposed);
    }

    #[test]
    fn test_accept_insufficient_bond() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        let accept_info = message_info(&provider, &[Coin::new(500u128, DENOM)]);
        let err = execute(deps.as_mut(), mock_env(), accept_info, ExecuteMsg::AcceptAgreement { agreement_id: id }).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientFunds { .. }));
    }

    #[test]
    fn test_accept_wrong_sender() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let rando = addr("rando");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        let info = message_info(&rando, &[Coin::new(1000u128, DENOM)]);
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::AcceptAgreement { agreement_id: id }).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));
    }

    #[test]
    fn test_fund_and_accept_transitions_to_funded() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Proposed);
        assert!(a.agreement.client_funded);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Funded);
    }

    #[test]
    fn test_happy_path_3_milestones() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        for ms_idx in 0..3u32 {
            let submit_info = message_info(&provider, &[]);
            execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: ms_idx, deliverable_iri: format!("regen:iri/phase{}", ms_idx) }).unwrap();
            let approve_info = message_info(&client, &[]);
            execute(deps.as_mut(), mock_env(), approve_info, ExecuteMsg::ApproveMilestone { agreement_id: id, milestone_index: ms_idx }).unwrap();
        }

        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Completed);
        assert!(a.agreement.completed_at.is_some());
    }

    #[test]
    fn test_cancel_proposed() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        let info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), info, ExecuteMsg::CancelAgreement { agreement_id: id }).unwrap();
        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Cancelled);
    }

    #[test]
    fn test_cancel_funded_applies_fee() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        let info = message_info(&client, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::CancelAgreement { agreement_id: id }).unwrap();
        assert_eq!(res.messages.len(), 3);
        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Cancelled);
    }

    #[test]
    fn test_cancel_in_progress_fails() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);
        let info = message_info(&client, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::CancelAgreement { agreement_id: id }).unwrap_err();
        assert!(matches!(err, ContractError::InvalidStatus { .. }));
    }

    #[test]
    fn test_dispute_client_wins() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/bad".to_string() }).unwrap();
        let dispute_info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), dispute_info, ExecuteMsg::DisputeMilestone { agreement_id: id, milestone_index: 0, reason: "Incomplete".to_string() }).unwrap();

        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Disputed);

        let d: DisputeResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Dispute { agreement_id: id }).unwrap()).unwrap();
        assert!(d.dispute.is_some());
        assert_eq!(d.dispute.as_ref().unwrap().milestone_index, 0);

        let arbiter = addr("arbiter_dao");
        let arbiter_info = message_info(&arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), arbiter_info, ExecuteMsg::ResolveDispute { agreement_id: id, resolution: DisputeResolution::ClientWins }).unwrap();
        assert!(!res.messages.is_empty());

        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Completed);
    }

    #[test]
    fn test_dispute_provider_wins() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/good".to_string() }).unwrap();
        let dispute_info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), dispute_info, ExecuteMsg::DisputeMilestone { agreement_id: id, milestone_index: 0, reason: "Unfounded".to_string() }).unwrap();

        let arbiter = addr("arbiter_dao");
        let arbiter_info = message_info(&arbiter, &[]);
        execute(deps.as_mut(), mock_env(), arbiter_info, ExecuteMsg::ResolveDispute { agreement_id: id, resolution: DisputeResolution::ProviderWins }).unwrap();

        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::InProgress);
        assert_eq!(a.agreement.current_milestone, 1);
    }

    #[test]
    fn test_dispute_split() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/partial".to_string() }).unwrap();
        let dispute_info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), dispute_info, ExecuteMsg::DisputeMilestone { agreement_id: id, milestone_index: 0, reason: "Partial".to_string() }).unwrap();

        let arbiter = addr("arbiter_dao");
        let arbiter_info = message_info(&arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), arbiter_info, ExecuteMsg::ResolveDispute { agreement_id: id, resolution: DisputeResolution::Split { client_percent: 60 } }).unwrap();
        assert!(!res.messages.is_empty());

        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Completed);
    }

    #[test]
    fn test_dispute_invalid_split_percent() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/x".to_string() }).unwrap();
        let dispute_info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), dispute_info, ExecuteMsg::DisputeMilestone { agreement_id: id, milestone_index: 0, reason: "Bad".to_string() }).unwrap();

        let arbiter = addr("arbiter_dao");
        let arbiter_info = message_info(&arbiter, &[]);
        let err = execute(deps.as_mut(), mock_env(), arbiter_info, ExecuteMsg::ResolveDispute { agreement_id: id, resolution: DisputeResolution::Split { client_percent: 100 } }).unwrap_err();
        assert!(matches!(err, ContractError::InvalidSplitPercent { got: 100 }));
    }

    #[test]
    fn test_dispute_unauthorized_resolver() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let rando = addr("rando");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/x".to_string() }).unwrap();
        let dispute_info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), dispute_info, ExecuteMsg::DisputeMilestone { agreement_id: id, milestone_index: 0, reason: "Bad".to_string() }).unwrap();

        let rando_info = message_info(&rando, &[]);
        let err = execute(deps.as_mut(), mock_env(), rando_info, ExecuteMsg::ResolveDispute { agreement_id: id, resolution: DisputeResolution::ClientWins }).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));
    }

    #[test]
    fn test_revise_milestone() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/v1".to_string() }).unwrap();

        let revise_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), revise_info, ExecuteMsg::ReviseMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/v2".to_string() }).unwrap();

        let ms: MilestonesResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Milestones { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(ms.milestones[0].revision_count, 1);
        assert_eq!(ms.milestones[0].deliverable_iri, Some("regen:iri/v2".to_string()));
    }

    #[test]
    fn test_max_revisions_exceeded() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/v1".to_string() }).unwrap();

        for i in 2..=5 {
            let revise_info = message_info(&provider, &[]);
            let result = execute(deps.as_mut(), mock_env(), revise_info, ExecuteMsg::ReviseMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: format!("regen:iri/v{}", i) });
            if i <= 4 {
                result.unwrap();
            } else {
                let err = result.unwrap_err();
                assert!(matches!(err, ContractError::MaxRevisionsExceeded { .. }));
            }
        }
    }

    #[test]
    fn test_submit_wrong_milestone_index() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        let err = execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 1, deliverable_iri: "regen:iri/wrong".to_string() }).unwrap_err();
        assert!(matches!(err, ContractError::InvalidMilestoneIndex { .. }));
    }

    #[test]
    fn test_query_escrow_balance() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let balance: EscrowBalanceResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::EscrowBalance { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(balance.escrow_amount, Uint128::new(10000));
        assert_eq!(balance.provider_bond, Uint128::new(1000));
        assert_eq!(balance.total_released, Uint128::zero());
        assert_eq!(balance.remaining_escrow, Uint128::new(10000));
        assert_eq!(balance.denom, DENOM);
    }

    #[test]
    fn test_query_agreements_by_client() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let p1 = addr("provider1");
        let p2 = addr("provider2");
        propose_agreement(deps.as_mut(), &client, &p1);
        propose_agreement(deps.as_mut(), &client, &p2);
        let resp: AgreementsResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::AgreementsByClient { client: client.to_string(), start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(resp.agreements.len(), 2);
    }

    #[test]
    fn test_query_agreements_by_provider() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let c1 = addr("client1");
        let c2 = addr("client2");
        let provider = addr("provider");
        propose_agreement(deps.as_mut(), &c1, &provider);
        propose_agreement(deps.as_mut(), &c2, &provider);
        let resp: AgreementsResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::AgreementsByProvider { provider: provider.to_string(), start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(resp.agreements.len(), 2);
    }

    #[test]
    fn test_query_agreements_by_status() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let p1 = addr("provider1");
        let p2 = addr("provider2");
        let id1 = propose_agreement(deps.as_mut(), &client, &p1);
        propose_agreement(deps.as_mut(), &client, &p2);
        let cancel_info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), cancel_info, ExecuteMsg::CancelAgreement { agreement_id: id1 }).unwrap();

        let resp: AgreementsResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreements { status: Some(AgreementStatus::Proposed), start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(resp.agreements.len(), 1);
        let resp: AgreementsResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreements { status: Some(AgreementStatus::Cancelled), start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(resp.agreements.len(), 1);
    }

    #[test]
    fn test_query_no_dispute() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        let d: DisputeResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Dispute { agreement_id: id }).unwrap()).unwrap();
        assert!(d.dispute.is_none());
    }

    #[test]
    fn test_query_agreement_not_found() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let result = query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: 99 });
        assert!(result.is_err());
    }

    #[test]
    fn test_update_config() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());
        let msg = ExecuteMsg::UpdateConfig {
            arbiter_dao: None, community_pool: None, provider_bond_ratio_bps: Some(1500),
            platform_fee_rate_bps: Some(200), cancellation_fee_rate_bps: None,
            arbiter_fee_rate_bps: None, review_period_seconds: Some(604800),
            max_milestones: Some(15), max_revisions: Some(5),
        };
        execute(deps.as_mut(), mock_env(), admin_info, msg).unwrap();
        let config: ConfigResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.provider_bond_ratio_bps, 1500);
        assert_eq!(config.platform_fee_rate_bps, 200);
        assert_eq!(config.review_period_seconds, 604800);
        assert_eq!(config.max_milestones, 15);
        assert_eq!(config.max_revisions, 5);
    }

    #[test]
    fn test_update_config_unauthorized() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let rando = addr("rando");
        let rando_info = message_info(&rando, &[]);
        let msg = ExecuteMsg::UpdateConfig {
            arbiter_dao: None, community_pool: None, provider_bond_ratio_bps: Some(1500),
            platform_fee_rate_bps: None, cancellation_fee_rate_bps: None,
            arbiter_fee_rate_bps: None, review_period_seconds: None,
            max_milestones: None, max_revisions: None,
        };
        let err = execute(deps.as_mut(), mock_env(), rando_info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));
    }

    #[test]
    fn test_double_dispute_fails() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/x".to_string() }).unwrap();
        let dispute_info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), dispute_info, ExecuteMsg::DisputeMilestone { agreement_id: id, milestone_index: 0, reason: "Bad".to_string() }).unwrap();
        let dispute_info2 = message_info(&client, &[]);
        let err = execute(deps.as_mut(), mock_env(), dispute_info2, ExecuteMsg::DisputeMilestone { agreement_id: id, milestone_index: 0, reason: "Really bad".to_string() }).unwrap_err();
        assert!(matches!(err, ContractError::InvalidStatus { .. }));
    }

    #[test]
    fn test_escrow_balance_after_approval() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        fund_and_accept(deps.as_mut(), id, &client, &provider);
        fund_escrow(deps.as_mut(), id, &client);
        start_agreement(deps.as_mut(), id, &client);

        let submit_info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), submit_info, ExecuteMsg::SubmitMilestone { agreement_id: id, milestone_index: 0, deliverable_iri: "regen:iri/m0".to_string() }).unwrap();
        let approve_info = message_info(&client, &[]);
        execute(deps.as_mut(), mock_env(), approve_info, ExecuteMsg::ApproveMilestone { agreement_id: id, milestone_index: 0 }).unwrap();

        let balance: EscrowBalanceResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::EscrowBalance { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(balance.total_released, Uint128::new(2970));
        assert_eq!(balance.total_fees, Uint128::new(30));
        assert_eq!(balance.remaining_escrow, Uint128::new(7000));
    }

    #[test]
    fn test_provider_can_cancel_proposed() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        let info = message_info(&provider, &[]);
        execute(deps.as_mut(), mock_env(), info, ExecuteMsg::CancelAgreement { agreement_id: id }).unwrap();
        let a: AgreementResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreement { agreement_id: id }).unwrap()).unwrap();
        assert_eq!(a.agreement.status, AgreementStatus::Cancelled);
    }

    #[test]
    fn test_agreement_ids_increment() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let p1 = addr("p1");
        let p2 = addr("p2");
        let p3 = addr("p3");
        let id1 = propose_agreement(deps.as_mut(), &client, &p1);
        let id2 = propose_agreement(deps.as_mut(), &client, &p2);
        let id3 = propose_agreement(deps.as_mut(), &client, &p3);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_query_agreements_pagination() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        for i in 0..5 {
            let p = addr(&format!("provider{}", i));
            propose_agreement(deps.as_mut(), &client, &p);
        }
        let resp: AgreementsResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreements { status: None, start_after: None, limit: Some(2) }).unwrap()).unwrap();
        assert_eq!(resp.agreements.len(), 2);
        assert_eq!(resp.agreements[0].id, 1);
        assert_eq!(resp.agreements[1].id, 2);
        let resp: AgreementsResponse = cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Agreements { status: None, start_after: Some(2), limit: Some(2) }).unwrap()).unwrap();
        assert_eq!(resp.agreements.len(), 2);
        assert_eq!(resp.agreements[0].id, 3);
        assert_eq!(resp.agreements[1].id, 4);
    }

    #[test]
    fn test_fund_wrong_denom() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        let client = addr("client");
        let provider = addr("provider");
        let id = propose_agreement(deps.as_mut(), &client, &provider);
        let fund_info = message_info(&client, &[Coin::new(10000u128, "uatom")]);
        let err = execute(deps.as_mut(), mock_env(), fund_info, ExecuteMsg::FundAgreement { agreement_id: id }).unwrap_err();
        assert!(matches!(err, ContractError::WrongDenom { .. }));
    }
}
