use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult, Timestamp, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, ProposalResponse, ProposalsResponse, QueryMsg,
};
use crate::state::{
    AgentRecommendation, Config, Proposal, ProposalStatus, CONFIG, NEXT_PROPOSAL_ID, PROPOSALS,
    VOTES,
};

const CONTRACT_NAME: &str = "crates.io:credit-class-voting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

/// Default deposit: 1000 REGEN = 1_000_000_000 uregen
const DEFAULT_DEPOSIT: u128 = 1_000_000_000;
/// Default voting period: 7 days
const DEFAULT_VOTING_PERIOD: u64 = 604_800;
/// Default agent review timeout: 24 hours
const DEFAULT_AGENT_REVIEW_TIMEOUT: u64 = 86_400;
/// Default override window: 6 hours
const DEFAULT_OVERRIDE_WINDOW: u64 = 21_600;

/// Slash 20% of deposit on rejection
const REJECT_SLASH_BPS: u128 = 2000;
/// Slash 5% of deposit on expiry
const EXPIRE_SLASH_BPS: u128 = 500;

// ── Instantiate ────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let community_pool = msg
        .community_pool
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let config = Config {
        admin: info.sender.clone(),
        registry_agent: deps.api.addr_validate(&msg.registry_agent)?,
        deposit_amount: msg.deposit_amount.unwrap_or(Uint128::new(DEFAULT_DEPOSIT)),
        denom: msg.denom.unwrap_or_else(|| "uregen".to_string()),
        voting_period_seconds: msg.voting_period_seconds.unwrap_or(DEFAULT_VOTING_PERIOD),
        agent_review_timeout_seconds: msg
            .agent_review_timeout_seconds
            .unwrap_or(DEFAULT_AGENT_REVIEW_TIMEOUT),
        override_window_seconds: msg
            .override_window_seconds
            .unwrap_or(DEFAULT_OVERRIDE_WINDOW),
        community_pool,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    NEXT_PROPOSAL_ID.save(deps.storage, &1u64)?;

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
        ExecuteMsg::SubmitProposal {
            admin_address,
            credit_type,
            methodology_iri,
        } => execute_submit_proposal(deps, env, info, admin_address, credit_type, methodology_iri),
        ExecuteMsg::SubmitAgentScore {
            proposal_id,
            score,
            confidence,
            recommendation,
        } => execute_submit_agent_score(deps, env, info, proposal_id, score, confidence, recommendation),
        ExecuteMsg::OverrideAgentReject { proposal_id } => {
            execute_override_agent_reject(deps, env, info, proposal_id)
        }
        ExecuteMsg::CastVote {
            proposal_id,
            vote_yes,
        } => execute_cast_vote(deps, env, info, proposal_id, vote_yes),
        ExecuteMsg::FinalizeProposal { proposal_id } => {
            execute_finalize_proposal(deps, env, info, proposal_id)
        }
        ExecuteMsg::UpdateConfig {
            registry_agent,
            deposit_amount,
            voting_period_seconds,
            agent_review_timeout_seconds,
            override_window_seconds,
            community_pool,
        } => execute_update_config(
            deps,
            info,
            registry_agent,
            deposit_amount,
            voting_period_seconds,
            agent_review_timeout_seconds,
            override_window_seconds,
            community_pool,
        ),
    }
}

// ── Submit Proposal ───────────────────────────────────────────────────

fn execute_submit_proposal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    admin_address: String,
    credit_type: String,
    methodology_iri: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Validate inputs
    if credit_type.is_empty() {
        return Err(ContractError::EmptyCreditType);
    }
    if methodology_iri.is_empty() {
        return Err(ContractError::EmptyMethodologyIri);
    }

    // Validate deposit
    let sent = info
        .funds
        .iter()
        .find(|c| c.denom == config.denom)
        .map(|c| c.amount)
        .unwrap_or(Uint128::zero());

    if sent < config.deposit_amount {
        return Err(ContractError::InsufficientFunds {
            required: config.deposit_amount.to_string(),
            sent: sent.to_string(),
        });
    }

    // Check denom — reject if wrong denom sent
    for coin in &info.funds {
        if coin.denom != config.denom {
            return Err(ContractError::WrongDenom {
                expected: config.denom.clone(),
                got: coin.denom.clone(),
            });
        }
    }

    let admin_addr = deps.api.addr_validate(&admin_address)?;

    let id = NEXT_PROPOSAL_ID.load(deps.storage)?;
    NEXT_PROPOSAL_ID.save(deps.storage, &(id + 1))?;

    let proposal = Proposal {
        id,
        proposer: info.sender.clone(),
        admin_address: admin_addr,
        credit_type: credit_type.clone(),
        methodology_iri,
        status: ProposalStatus::AgentReview,
        deposit_amount: config.deposit_amount,
        created_at: env.block.time,
        agent_score: None,
        agent_confidence: None,
        agent_recommendation: None,
        agent_scored_at: None,
        voting_ends_at: None,
        yes_votes: 0,
        no_votes: 0,
        completed_at: None,
    };

    PROPOSALS.save(deps.storage, id, &proposal)?;

    Ok(Response::new()
        .add_attribute("action", "submit_proposal")
        .add_attribute("proposal_id", id.to_string())
        .add_attribute("proposer", info.sender)
        .add_attribute("credit_type", credit_type)
        .add_attribute("status", "AgentReview"))
}

// ── Submit Agent Score ────────────────────────────────────────────────

fn execute_submit_agent_score(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    score: u32,
    confidence: u32,
    recommendation: AgentRecommendation,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only registry agent can submit scores
    if info.sender != config.registry_agent {
        return Err(ContractError::Unauthorized {
            reason: "only registry agent can submit scores".to_string(),
        });
    }

    // Validate score and confidence ranges
    if score > 1000 {
        return Err(ContractError::InvalidAgentScore { score });
    }
    if confidence > 1000 {
        return Err(ContractError::InvalidAgentConfidence { confidence });
    }

    let mut proposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::ProposalNotFound { id: proposal_id })?;

    // Must be in AgentReview status
    if proposal.status != ProposalStatus::AgentReview {
        return Err(ContractError::InvalidStatus {
            expected: "AgentReview".to_string(),
            actual: proposal.status.to_string(),
        });
    }

    proposal.agent_score = Some(score);
    proposal.agent_confidence = Some(confidence);
    proposal.agent_recommendation = Some(recommendation.clone());
    proposal.agent_scored_at = Some(env.block.time);

    // Decision logic:
    // score >= 700 → auto-advance to Voting
    // score < 300 && confidence > 900 → auto-reject (with override window)
    // else → advance to Voting
    let action_detail = if score >= 700 {
        proposal.status = ProposalStatus::Voting;
        proposal.voting_ends_at = Some(Timestamp::from_seconds(
            env.block.time.seconds() + config.voting_period_seconds,
        ));
        "auto_advance_high_score"
    } else if score < 300 && confidence > 900 {
        proposal.status = ProposalStatus::AutoRejected;
        "auto_reject_low_score_high_confidence"
    } else {
        proposal.status = ProposalStatus::Voting;
        proposal.voting_ends_at = Some(Timestamp::from_seconds(
            env.block.time.seconds() + config.voting_period_seconds,
        ));
        "advance_to_voting"
    };

    PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

    Ok(Response::new()
        .add_attribute("action", "submit_agent_score")
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("score", score.to_string())
        .add_attribute("confidence", confidence.to_string())
        .add_attribute("recommendation", recommendation.to_string())
        .add_attribute("result", action_detail)
        .add_attribute("status", proposal.status.to_string()))
}

// ── Override Agent Reject ─────────────────────────────────────────────

fn execute_override_agent_reject(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only admin can override
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "only admin can override agent rejections".to_string(),
        });
    }

    let mut proposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::ProposalNotFound { id: proposal_id })?;

    // Must be AutoRejected
    if proposal.status != ProposalStatus::AutoRejected {
        return Err(ContractError::InvalidStatus {
            expected: "AutoRejected".to_string(),
            actual: proposal.status.to_string(),
        });
    }

    // Must be within override window
    let scored_at = proposal.agent_scored_at.ok_or(ContractError::InvalidStatus {
        expected: "agent_scored_at set".to_string(),
        actual: "agent_scored_at not set".to_string(),
    })?;
    let window_end = scored_at.seconds() + config.override_window_seconds;
    if env.block.time.seconds() > window_end {
        return Err(ContractError::OverrideWindowExpired);
    }

    // Force to Voting
    proposal.status = ProposalStatus::Voting;
    proposal.voting_ends_at = Some(Timestamp::from_seconds(
        env.block.time.seconds() + config.voting_period_seconds,
    ));

    PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

    Ok(Response::new()
        .add_attribute("action", "override_agent_reject")
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("status", "Voting"))
}

// ── Cast Vote ─────────────────────────────────────────────────────────

fn execute_cast_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    vote_yes: bool,
) -> Result<Response, ContractError> {
    let proposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::ProposalNotFound { id: proposal_id })?;

    // Must be in Voting status
    if proposal.status != ProposalStatus::Voting {
        return Err(ContractError::InvalidStatus {
            expected: "Voting".to_string(),
            actual: proposal.status.to_string(),
        });
    }

    // Must be within voting period
    if let Some(ends_at) = proposal.voting_ends_at {
        if env.block.time.seconds() > ends_at.seconds() {
            return Err(ContractError::VotingPeriodEnded);
        }
    }

    // Check for duplicate vote
    if VOTES.may_load(deps.storage, (proposal_id, &info.sender))?.is_some() {
        return Err(ContractError::AlreadyVoted { id: proposal_id });
    }

    // Record vote
    VOTES.save(deps.storage, (proposal_id, &info.sender), &vote_yes)?;

    // Update tally
    let mut proposal = proposal;
    if vote_yes {
        proposal.yes_votes += 1;
    } else {
        proposal.no_votes += 1;
    }

    PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

    Ok(Response::new()
        .add_attribute("action", "cast_vote")
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("voter", info.sender)
        .add_attribute("vote", if vote_yes { "yes" } else { "no" })
        .add_attribute("yes_votes", proposal.yes_votes.to_string())
        .add_attribute("no_votes", proposal.no_votes.to_string()))
}

// ── Finalize Proposal ─────────────────────────────────────────────────

fn execute_finalize_proposal(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut proposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::ProposalNotFound { id: proposal_id })?;

    let mut messages: Vec<BankMsg> = vec![];

    // Slash recipient: community_pool if configured, otherwise admin
    let slash_recipient = config
        .community_pool
        .as_ref()
        .map(|a| a.to_string())
        .unwrap_or_else(|| config.admin.to_string());

    match proposal.status {
        ProposalStatus::Voting => {
            // Must be after voting period
            if let Some(ends_at) = proposal.voting_ends_at {
                if env.block.time.seconds() <= ends_at.seconds() {
                    return Err(ContractError::VotingPeriodNotEnded);
                }
            }

            let total_votes = proposal.yes_votes + proposal.no_votes;

            if total_votes == 0 {
                // No votes cast — treat as expired, slash 5%
                let slash = proposal
                    .deposit_amount
                    .multiply_ratio(EXPIRE_SLASH_BPS, 10_000u128);
                let refund = proposal.deposit_amount - slash;

                proposal.status = ProposalStatus::Expired;
                proposal.completed_at = Some(env.block.time);

                if !refund.is_zero() {
                    messages.push(BankMsg::Send {
                        to_address: proposal.proposer.to_string(),
                        amount: vec![Coin {
                            denom: config.denom.clone(),
                            amount: refund,
                        }],
                    });
                }
                if !slash.is_zero() {
                    messages.push(BankMsg::Send {
                        to_address: slash_recipient.clone(),
                        amount: vec![Coin {
                            denom: config.denom.clone(),
                            amount: slash,
                        }],
                    });
                }
            } else if proposal.yes_votes > proposal.no_votes {
                // Approved — refund full deposit
                proposal.status = ProposalStatus::Approved;
                proposal.completed_at = Some(env.block.time);

                messages.push(BankMsg::Send {
                    to_address: proposal.proposer.to_string(),
                    amount: vec![Coin {
                        denom: config.denom.clone(),
                        amount: proposal.deposit_amount,
                    }],
                });
            } else {
                // Rejected — slash 20%
                let slash = proposal
                    .deposit_amount
                    .multiply_ratio(REJECT_SLASH_BPS, 10_000u128);
                let refund = proposal.deposit_amount - slash;

                proposal.status = ProposalStatus::Rejected;
                proposal.completed_at = Some(env.block.time);

                if !refund.is_zero() {
                    messages.push(BankMsg::Send {
                        to_address: proposal.proposer.to_string(),
                        amount: vec![Coin {
                            denom: config.denom.clone(),
                            amount: refund,
                        }],
                    });
                }
                if !slash.is_zero() {
                    messages.push(BankMsg::Send {
                        to_address: slash_recipient.clone(),
                        amount: vec![Coin {
                            denom: config.denom.clone(),
                            amount: slash,
                        }],
                    });
                }
            }
        }
        ProposalStatus::AutoRejected => {
            // Finalize an auto-rejected proposal after override window expires
            let scored_at = proposal.agent_scored_at.ok_or(ContractError::InvalidStatus {
                expected: "agent_scored_at set".to_string(),
                actual: "agent_scored_at not set".to_string(),
            })?;
            let window_end = scored_at.seconds() + config.override_window_seconds;
            if env.block.time.seconds() <= window_end {
                return Err(ContractError::InvalidStatus {
                    expected: "override window expired".to_string(),
                    actual: "override window still active".to_string(),
                });
            }

            // Slash 20% on auto-reject finalization
            let slash = proposal
                .deposit_amount
                .multiply_ratio(REJECT_SLASH_BPS, 10_000u128);
            let refund = proposal.deposit_amount - slash;

            proposal.status = ProposalStatus::Rejected;
            proposal.completed_at = Some(env.block.time);

            if !refund.is_zero() {
                messages.push(BankMsg::Send {
                    to_address: proposal.proposer.to_string(),
                    amount: vec![Coin {
                        denom: config.denom.clone(),
                        amount: refund,
                    }],
                });
            }
            if !slash.is_zero() {
                messages.push(BankMsg::Send {
                    to_address: slash_recipient.clone(),
                    amount: vec![Coin {
                        denom: config.denom.clone(),
                        amount: slash,
                    }],
                });
            }
        }
        _ => {
            return Err(ContractError::InvalidStatus {
                expected: "Voting or AutoRejected".to_string(),
                actual: proposal.status.to_string(),
            });
        }
    }

    PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

    let mut resp = Response::new()
        .add_attribute("action", "finalize_proposal")
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("status", proposal.status.to_string());

    for msg in messages {
        resp = resp.add_message(msg);
    }

    Ok(resp)
}

// ── Update Config ─────────────────────────────────────────────────────

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    registry_agent: Option<String>,
    deposit_amount: Option<Uint128>,
    voting_period_seconds: Option<u64>,
    agent_review_timeout_seconds: Option<u64>,
    override_window_seconds: Option<u64>,
    community_pool: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {
            reason: "only admin can update config".to_string(),
        });
    }

    if let Some(agent) = registry_agent {
        config.registry_agent = deps.api.addr_validate(&agent)?;
    }
    if let Some(amount) = deposit_amount {
        config.deposit_amount = amount;
    }
    if let Some(seconds) = voting_period_seconds {
        config.voting_period_seconds = seconds;
    }
    if let Some(seconds) = agent_review_timeout_seconds {
        config.agent_review_timeout_seconds = seconds;
    }
    if let Some(seconds) = override_window_seconds {
        config.override_window_seconds = seconds;
    }
    if let Some(pool) = community_pool {
        config.community_pool = Some(deps.api.addr_validate(&pool)?);
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// ── Query ─────────────────────────────────────────────────────────────

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Proposal { proposal_id } => to_json_binary(&query_proposal(deps, proposal_id)?),
        QueryMsg::Proposals {
            status,
            start_after,
            limit,
        } => to_json_binary(&query_proposals(deps, status, start_after, limit)?),
        QueryMsg::ProposalsByAdmin {
            admin_address,
            start_after,
            limit,
        } => to_json_binary(&query_proposals_by_admin(
            deps,
            admin_address,
            start_after,
            limit,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.to_string(),
        registry_agent: config.registry_agent.to_string(),
        deposit_amount: config.deposit_amount,
        denom: config.denom,
        voting_period_seconds: config.voting_period_seconds,
        agent_review_timeout_seconds: config.agent_review_timeout_seconds,
        override_window_seconds: config.override_window_seconds,
        community_pool: config.community_pool.map(|a| a.to_string()),
    })
}

fn query_proposal(deps: Deps, proposal_id: u64) -> StdResult<ProposalResponse> {
    let proposal = PROPOSALS.load(deps.storage, proposal_id)?;
    Ok(ProposalResponse { proposal })
}

fn query_proposals(
    deps: Deps,
    status: Option<ProposalStatus>,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| cw_storage_plus::Bound::exclusive(s));

    let proposals: Vec<Proposal> = PROPOSALS
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|item| {
            let (_, proposal) = item.ok()?;
            if let Some(ref s) = status {
                if &proposal.status != s {
                    return None;
                }
            }
            Some(proposal)
        })
        .take(limit)
        .collect();

    Ok(ProposalsResponse { proposals })
}

fn query_proposals_by_admin(
    deps: Deps,
    admin_address: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalsResponse> {
    let admin_addr = deps.api.addr_validate(&admin_address)?;
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| cw_storage_plus::Bound::exclusive(s));

    let proposals: Vec<Proposal> = PROPOSALS
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|item| {
            let (_, proposal) = item.ok()?;
            if proposal.admin_address != admin_addr {
                return None;
            }
            Some(proposal)
        })
        .take(limit)
        .collect();

    Ok(ProposalsResponse { proposals })
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
    use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};

    const DENOM: &str = "uregen";
    const DEPOSIT: u128 = 1_000_000_000;

    fn addr(input: &str) -> Addr {
        MockApi::default().addr_make(input)
    }

    fn setup_contract(deps: DepsMut) -> MessageInfo {
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let msg = InstantiateMsg {
            registry_agent: addr("agent").to_string(),
            deposit_amount: Some(Uint128::new(DEPOSIT)),
            denom: Some(DENOM.to_string()),
            voting_period_seconds: Some(604_800),
            agent_review_timeout_seconds: Some(86_400),
            override_window_seconds: Some(21_600),
            community_pool: None,
        };
        instantiate(deps, mock_env(), info.clone(), msg).unwrap();
        info
    }

    fn submit_proposal(deps: DepsMut, proposer: &Addr) -> u64 {
        let deposit_coins = vec![Coin::new(DEPOSIT, DENOM)];
        let info = message_info(proposer, &deposit_coins);
        let msg = ExecuteMsg::SubmitProposal {
            admin_address: proposer.to_string(),
            credit_type: "C".to_string(),
            methodology_iri: "regen:13toVfvC2YxrrfSXWB5h2BGHiC9iJ8j7kp9KXcPoPMKH3p4TvG3r8Fh".to_string(),
        };
        let res = execute(deps, mock_env(), info, msg).unwrap();
        res.attributes
            .iter()
            .find(|a| a.key == "proposal_id")
            .unwrap()
            .value
            .parse()
            .unwrap()
    }

    fn agent_score(
        deps: DepsMut,
        proposal_id: u64,
        score: u32,
        confidence: u32,
        recommendation: AgentRecommendation,
    ) -> Response {
        let agent = addr("agent");
        let info = message_info(&agent, &[]);
        let msg = ExecuteMsg::SubmitAgentScore {
            proposal_id,
            score,
            confidence,
            recommendation,
        };
        execute(deps, mock_env(), info, msg).unwrap()
    }

    fn env_at(seconds: u64) -> Env {
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(seconds);
        env
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
        assert_eq!(config.registry_agent, addr("agent").to_string());
        assert_eq!(config.deposit_amount, Uint128::new(DEPOSIT));
        assert_eq!(config.denom, DENOM);
        assert_eq!(config.voting_period_seconds, 604_800);
        assert_eq!(config.agent_review_timeout_seconds, 86_400);
        assert_eq!(config.override_window_seconds, 21_600);
    }

    // ── Test 2: Submit Proposal ───────────────────────────────────────

    #[test]
    fn test_submit_proposal() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);
        assert_eq!(id, 1);

        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::AgentReview);
        assert_eq!(resp.proposal.proposer, proposer);
        assert_eq!(resp.proposal.credit_type, "C");
        assert_eq!(resp.proposal.deposit_amount, Uint128::new(DEPOSIT));
        assert_eq!(resp.proposal.yes_votes, 0);
        assert_eq!(resp.proposal.no_votes, 0);
    }

    // ── Test 3: Agent Score → Auto-Advance to Voting ──────────────────

    #[test]
    fn test_agent_score_auto_advance() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        // High score → auto advance to Voting
        let res = agent_score(
            deps.as_mut(),
            id,
            800,
            950,
            AgentRecommendation::Approve,
        );
        assert!(res
            .attributes
            .iter()
            .any(|a| a.key == "result" && a.value == "auto_advance_high_score"));

        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::Voting);
        assert_eq!(resp.proposal.agent_score, Some(800));
        assert_eq!(resp.proposal.agent_confidence, Some(950));
        assert_eq!(
            resp.proposal.agent_recommendation,
            Some(AgentRecommendation::Approve)
        );
        assert!(resp.proposal.voting_ends_at.is_some());
    }

    // ── Test 4: Agent Auto-Reject + Admin Override ────────────────────

    #[test]
    fn test_agent_auto_reject_and_override() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        // Low score + high confidence → auto-reject
        let agent = addr("agent");
        let agent_info = message_info(&agent, &[]);
        let score_msg = ExecuteMsg::SubmitAgentScore {
            proposal_id: id,
            score: 150,
            confidence: 950,
            recommendation: AgentRecommendation::Reject,
        };

        let base_time = mock_env().block.time.seconds();
        let mut env_now = mock_env();
        env_now.block.time = Timestamp::from_seconds(base_time);
        execute(deps.as_mut(), env_now, agent_info, score_msg).unwrap();

        // Verify auto-rejected
        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::AutoRejected);

        // Non-admin cannot override
        let rando = addr("rando");
        let rando_info = message_info(&rando, &[]);
        let override_msg = ExecuteMsg::OverrideAgentReject { proposal_id: id };
        let err = execute(
            deps.as_mut(),
            env_at(base_time + 100),
            rando_info,
            override_msg,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));

        // Admin override within 6h window
        let admin = addr("admin");
        let admin_info = message_info(&admin, &[]);
        let override_msg = ExecuteMsg::OverrideAgentReject { proposal_id: id };
        execute(
            deps.as_mut(),
            env_at(base_time + 100),
            admin_info,
            override_msg,
        )
        .unwrap();

        // Verify now in Voting
        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::Voting);
        assert!(resp.proposal.voting_ends_at.is_some());
    }

    // ── Test 5: Voting + Finalize (Approval) ──────────────────────────

    #[test]
    fn test_voting_and_finalize_approve() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        // Agent advances to voting
        agent_score(
            deps.as_mut(),
            id,
            750,
            900,
            AgentRecommendation::Approve,
        );

        let base_time = mock_env().block.time.seconds();

        // Voter 1 votes yes
        let voter1 = addr("voter1");
        let v1_info = message_info(&voter1, &[]);
        execute(
            deps.as_mut(),
            env_at(base_time + 100),
            v1_info,
            ExecuteMsg::CastVote {
                proposal_id: id,
                vote_yes: true,
            },
        )
        .unwrap();

        // Voter 2 votes yes
        let voter2 = addr("voter2");
        let v2_info = message_info(&voter2, &[]);
        execute(
            deps.as_mut(),
            env_at(base_time + 200),
            v2_info,
            ExecuteMsg::CastVote {
                proposal_id: id,
                vote_yes: true,
            },
        )
        .unwrap();

        // Voter 3 votes no
        let voter3 = addr("voter3");
        let v3_info = message_info(&voter3, &[]);
        execute(
            deps.as_mut(),
            env_at(base_time + 300),
            v3_info,
            ExecuteMsg::CastVote {
                proposal_id: id,
                vote_yes: false,
            },
        )
        .unwrap();

        // Duplicate vote should fail
        let v1_info_dup = message_info(&voter1, &[]);
        let err = execute(
            deps.as_mut(),
            env_at(base_time + 400),
            v1_info_dup,
            ExecuteMsg::CastVote {
                proposal_id: id,
                vote_yes: false,
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::AlreadyVoted { .. }));

        // Cannot finalize before voting period ends
        let finalizer = addr("finalizer");
        let fin_info = message_info(&finalizer, &[]);
        let err = execute(
            deps.as_mut(),
            env_at(base_time + 500),
            fin_info,
            ExecuteMsg::FinalizeProposal { proposal_id: id },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::VotingPeriodNotEnded));

        // Finalize after voting period
        let fin_info2 = message_info(&addr("finalizer"), &[]);
        let res = execute(
            deps.as_mut(),
            env_at(base_time + 604_800 + 1),
            fin_info2,
            ExecuteMsg::FinalizeProposal { proposal_id: id },
        )
        .unwrap();

        // Should be Approved with refund message
        assert!(res
            .attributes
            .iter()
            .any(|a| a.key == "status" && a.value == "Approved"));
        assert_eq!(res.messages.len(), 1); // refund

        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::Approved);
        assert_eq!(resp.proposal.yes_votes, 2);
        assert_eq!(resp.proposal.no_votes, 1);
        assert!(resp.proposal.completed_at.is_some());
    }

    // ── Test 6: Voting + Finalize (Rejection with slash) ──────────────

    #[test]
    fn test_voting_and_finalize_reject() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        agent_score(
            deps.as_mut(),
            id,
            500,
            700,
            AgentRecommendation::Conditional,
        );

        let base_time = mock_env().block.time.seconds();

        // 1 yes, 2 no → rejected
        let v1 = addr("v1");
        execute(
            deps.as_mut(),
            env_at(base_time + 100),
            message_info(&v1, &[]),
            ExecuteMsg::CastVote {
                proposal_id: id,
                vote_yes: true,
            },
        )
        .unwrap();

        let v2 = addr("v2");
        execute(
            deps.as_mut(),
            env_at(base_time + 200),
            message_info(&v2, &[]),
            ExecuteMsg::CastVote {
                proposal_id: id,
                vote_yes: false,
            },
        )
        .unwrap();

        let v3 = addr("v3");
        execute(
            deps.as_mut(),
            env_at(base_time + 300),
            message_info(&v3, &[]),
            ExecuteMsg::CastVote {
                proposal_id: id,
                vote_yes: false,
            },
        )
        .unwrap();

        // Finalize
        let res = execute(
            deps.as_mut(),
            env_at(base_time + 604_800 + 1),
            message_info(&addr("anyone"), &[]),
            ExecuteMsg::FinalizeProposal { proposal_id: id },
        )
        .unwrap();

        assert!(res
            .attributes
            .iter()
            .any(|a| a.key == "status" && a.value == "Rejected"));
        // Refund message (80% of deposit) + slash transfer (20% to admin)
        assert_eq!(res.messages.len(), 2);

        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::Rejected);
    }

    // ── Test 7: Override window expired ───────────────────────────────

    #[test]
    fn test_override_window_expired() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        // Agent auto-rejects
        let agent = addr("agent");
        let base_time = mock_env().block.time.seconds();
        let env_now = env_at(base_time);
        execute(
            deps.as_mut(),
            env_now,
            message_info(&agent, &[]),
            ExecuteMsg::SubmitAgentScore {
                proposal_id: id,
                score: 100,
                confidence: 950,
                recommendation: AgentRecommendation::Reject,
            },
        )
        .unwrap();

        // Try override after 6h window
        let admin = addr("admin");
        let err = execute(
            deps.as_mut(),
            env_at(base_time + 21_601),
            message_info(&admin, &[]),
            ExecuteMsg::OverrideAgentReject { proposal_id: id },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::OverrideWindowExpired));
    }

    // ── Test 8: Agent score unauthorized ──────────────────────────────

    #[test]
    fn test_agent_score_unauthorized() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        let rando = addr("rando");
        let info = message_info(&rando, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::SubmitAgentScore {
                proposal_id: id,
                score: 800,
                confidence: 900,
                recommendation: AgentRecommendation::Approve,
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));
    }

    // ── Test 9: Submit proposal insufficient funds ────────────────────

    #[test]
    fn test_submit_proposal_insufficient_funds() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let info = message_info(&proposer, &[Coin::new(100u128, DENOM)]);
        let msg = ExecuteMsg::SubmitProposal {
            admin_address: proposer.to_string(),
            credit_type: "C".to_string(),
            methodology_iri: "regen:13toVfvC2YxrrfSXWB5h2BGHiC9iJ8j7kp9KXcPoPMKH3p4TvG3r8Fh"
                .to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientFunds { .. }));
    }

    // ── Test 10: Middle score advances to voting ──────────────────────

    #[test]
    fn test_middle_score_advances_to_voting() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        // Score between 300-699 should still go to Voting
        let res = agent_score(
            deps.as_mut(),
            id,
            450,
            600,
            AgentRecommendation::Conditional,
        );
        assert!(res
            .attributes
            .iter()
            .any(|a| a.key == "result" && a.value == "advance_to_voting"));

        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::Voting);
    }

    // ── Test 11: Update config ────────────────────────────────────────

    #[test]
    fn test_update_config() {
        let mut deps = mock_dependencies();
        let admin_info = setup_contract(deps.as_mut());

        let new_agent = addr("new_agent");
        let msg = ExecuteMsg::UpdateConfig {
            registry_agent: Some(new_agent.to_string()),
            deposit_amount: Some(Uint128::new(500_000_000)),
            voting_period_seconds: Some(300_000),
            agent_review_timeout_seconds: None,
            override_window_seconds: None,
            community_pool: None,
        };
        execute(deps.as_mut(), mock_env(), admin_info, msg).unwrap();

        let config: ConfigResponse =
            cosmwasm_std::from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap())
                .unwrap();
        assert_eq!(config.registry_agent, new_agent.to_string());
        assert_eq!(config.deposit_amount, Uint128::new(500_000_000));
        assert_eq!(config.voting_period_seconds, 300_000);
        // Unchanged values
        assert_eq!(config.agent_review_timeout_seconds, 86_400);
        assert_eq!(config.override_window_seconds, 21_600);
    }

    // ── Test 12: Update config unauthorized ───────────────────────────

    #[test]
    fn test_update_config_unauthorized() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let rando = addr("rando");
        let info = message_info(&rando, &[]);
        let msg = ExecuteMsg::UpdateConfig {
            registry_agent: None,
            deposit_amount: None,
            voting_period_seconds: None,
            agent_review_timeout_seconds: None,
            override_window_seconds: None,
            community_pool: None,
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized { .. }));
    }

    // ── Test 13: Query proposals by status ────────────────────────────

    #[test]
    fn test_query_proposals_by_status() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let p1 = addr("p1");
        let p2 = addr("p2");
        submit_proposal(deps.as_mut(), &p1);
        let id2 = submit_proposal(deps.as_mut(), &p2);

        // Advance id2 to Voting
        agent_score(
            deps.as_mut(),
            id2,
            800,
            900,
            AgentRecommendation::Approve,
        );

        // Query AgentReview — should only get proposal 1
        let resp: ProposalsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposals {
                    status: Some(ProposalStatus::AgentReview),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposals.len(), 1);
        assert_eq!(resp.proposals[0].id, 1);

        // Query Voting — should only get proposal 2
        let resp: ProposalsResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposals {
                    status: Some(ProposalStatus::Voting),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposals.len(), 1);
        assert_eq!(resp.proposals[0].id, 2);
    }

    // ── Test 14: Proposal IDs increment ───────────────────────────────

    #[test]
    fn test_proposal_ids_increment() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let p1 = addr("p1");
        let p2 = addr("p2");
        let p3 = addr("p3");

        let id1 = submit_proposal(deps.as_mut(), &p1);
        let id2 = submit_proposal(deps.as_mut(), &p2);
        let id3 = submit_proposal(deps.as_mut(), &p3);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    // ── Test 15: Expired proposal (no votes) ──────────────────────────

    #[test]
    fn test_finalize_expired_no_votes() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        agent_score(
            deps.as_mut(),
            id,
            600,
            700,
            AgentRecommendation::Conditional,
        );

        let base_time = mock_env().block.time.seconds();

        // Finalize after voting ends with no votes → Expired, 5% slash
        let res = execute(
            deps.as_mut(),
            env_at(base_time + 604_800 + 1),
            message_info(&addr("anyone"), &[]),
            ExecuteMsg::FinalizeProposal { proposal_id: id },
        )
        .unwrap();

        assert!(res
            .attributes
            .iter()
            .any(|a| a.key == "status" && a.value == "Expired"));

        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::Expired);
    }

    // ── Test 16: Reject slash transferred to admin (fallback) ────────

    #[test]
    fn test_reject_slash_transferred_to_admin() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        agent_score(
            deps.as_mut(),
            id,
            500,
            700,
            AgentRecommendation::Conditional,
        );

        let base_time = mock_env().block.time.seconds();

        // 1 yes, 2 no → rejected
        for (name, vote_yes) in [("v1", true), ("v2", false), ("v3", false)] {
            let voter = addr(name);
            execute(
                deps.as_mut(),
                env_at(base_time + 100),
                message_info(&voter, &[]),
                ExecuteMsg::CastVote {
                    proposal_id: id,
                    vote_yes,
                },
            )
            .unwrap();
        }

        let res = execute(
            deps.as_mut(),
            env_at(base_time + 604_800 + 1),
            message_info(&addr("anyone"), &[]),
            ExecuteMsg::FinalizeProposal { proposal_id: id },
        )
        .unwrap();

        // 2 messages: refund (80%) + slash (20%) to admin
        assert_eq!(res.messages.len(), 2);

        let admin = addr("admin");
        let expected_slash = Uint128::new(DEPOSIT).multiply_ratio(2000u128, 10_000u128);
        let expected_refund = Uint128::new(DEPOSIT) - expected_slash;

        // First message: refund to proposer
        if let cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
            &res.messages[0].msg
        {
            assert_eq!(to_address, &proposer.to_string());
            assert_eq!(amount[0].amount, expected_refund);
        } else {
            panic!("Expected BankMsg::Send for refund");
        }

        // Second message: slash to admin (no community_pool configured)
        if let cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
            &res.messages[1].msg
        {
            assert_eq!(to_address, &admin.to_string());
            assert_eq!(amount[0].amount, expected_slash);
        } else {
            panic!("Expected BankMsg::Send for slash");
        }
    }

    // ── Test 17: Expired slash transferred to admin ──────────────────

    #[test]
    fn test_expired_slash_transferred_to_admin() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        agent_score(
            deps.as_mut(),
            id,
            600,
            700,
            AgentRecommendation::Conditional,
        );

        let base_time = mock_env().block.time.seconds();

        // Finalize with no votes → Expired, 5% slash
        let res = execute(
            deps.as_mut(),
            env_at(base_time + 604_800 + 1),
            message_info(&addr("anyone"), &[]),
            ExecuteMsg::FinalizeProposal { proposal_id: id },
        )
        .unwrap();

        // 2 messages: refund (95%) + slash (5%)
        assert_eq!(res.messages.len(), 2);

        let admin = addr("admin");
        let expected_slash = Uint128::new(DEPOSIT).multiply_ratio(500u128, 10_000u128);
        let expected_refund = Uint128::new(DEPOSIT) - expected_slash;

        if let cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
            &res.messages[0].msg
        {
            assert_eq!(to_address, &proposer.to_string());
            assert_eq!(amount[0].amount, expected_refund);
        } else {
            panic!("Expected BankMsg::Send for refund");
        }

        if let cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
            &res.messages[1].msg
        {
            assert_eq!(to_address, &admin.to_string());
            assert_eq!(amount[0].amount, expected_slash);
        } else {
            panic!("Expected BankMsg::Send for slash");
        }
    }

    // ── Test 18: Slash goes to community_pool when configured ────────

    #[test]
    fn test_slash_transferred_to_community_pool() {
        let mut deps = mock_dependencies();
        let admin = addr("admin");
        let info = message_info(&admin, &[]);
        let pool = addr("community_pool");
        let msg = InstantiateMsg {
            registry_agent: addr("agent").to_string(),
            deposit_amount: Some(Uint128::new(DEPOSIT)),
            denom: Some(DENOM.to_string()),
            voting_period_seconds: Some(604_800),
            agent_review_timeout_seconds: Some(86_400),
            override_window_seconds: Some(21_600),
            community_pool: Some(pool.to_string()),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        agent_score(
            deps.as_mut(),
            id,
            500,
            700,
            AgentRecommendation::Conditional,
        );

        let base_time = mock_env().block.time.seconds();

        // 0 yes, 1 no → rejected
        let v1 = addr("v1");
        execute(
            deps.as_mut(),
            env_at(base_time + 100),
            message_info(&v1, &[]),
            ExecuteMsg::CastVote {
                proposal_id: id,
                vote_yes: false,
            },
        )
        .unwrap();

        let res = execute(
            deps.as_mut(),
            env_at(base_time + 604_800 + 1),
            message_info(&addr("anyone"), &[]),
            ExecuteMsg::FinalizeProposal { proposal_id: id },
        )
        .unwrap();

        assert_eq!(res.messages.len(), 2);

        let expected_slash = Uint128::new(DEPOSIT).multiply_ratio(2000u128, 10_000u128);

        // Slash goes to community_pool, not admin
        if let cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
            &res.messages[1].msg
        {
            assert_eq!(to_address, &pool.to_string());
            assert_eq!(amount[0].amount, expected_slash);
        } else {
            panic!("Expected BankMsg::Send for slash to community_pool");
        }
    }

    // ── Test 19: Auto-reject finalization transfers slash ─────────────

    #[test]
    fn test_auto_reject_finalize_slash_transferred() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let proposer = addr("proposer");
        let id = submit_proposal(deps.as_mut(), &proposer);

        // Agent auto-rejects
        let agent = addr("agent");
        let base_time = mock_env().block.time.seconds();
        execute(
            deps.as_mut(),
            env_at(base_time),
            message_info(&agent, &[]),
            ExecuteMsg::SubmitAgentScore {
                proposal_id: id,
                score: 100,
                confidence: 950,
                recommendation: AgentRecommendation::Reject,
            },
        )
        .unwrap();

        // Finalize after override window (6h = 21600s)
        let res = execute(
            deps.as_mut(),
            env_at(base_time + 21_601),
            message_info(&addr("anyone"), &[]),
            ExecuteMsg::FinalizeProposal { proposal_id: id },
        )
        .unwrap();

        // 2 messages: refund (80%) + slash (20%) to admin
        assert_eq!(res.messages.len(), 2);

        let admin = addr("admin");
        let expected_slash = Uint128::new(DEPOSIT).multiply_ratio(2000u128, 10_000u128);
        let expected_refund = Uint128::new(DEPOSIT) - expected_slash;

        if let cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
            &res.messages[0].msg
        {
            assert_eq!(to_address, &proposer.to_string());
            assert_eq!(amount[0].amount, expected_refund);
        } else {
            panic!("Expected BankMsg::Send for refund");
        }

        if let cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) =
            &res.messages[1].msg
        {
            assert_eq!(to_address, &admin.to_string());
            assert_eq!(amount[0].amount, expected_slash);
        } else {
            panic!("Expected BankMsg::Send for slash");
        }

        let resp: ProposalResponse = cosmwasm_std::from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Proposal { proposal_id: id },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.proposal.status, ProposalStatus::Rejected);
    }
}
