use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
use cosmwasm_std::{from_json, Addr, Coin, Timestamp, Uint128};

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;

// ---------------------------------------------------------------------------
// Helpers — use MockApi::addr_make to produce bech32-valid addresses
// ---------------------------------------------------------------------------

fn admin() -> Addr {
    MockApi::default().addr_make("admin")
}

fn signaler_a() -> Addr {
    MockApi::default().addr_make("signaler_a")
}

fn signaler_b() -> Addr {
    MockApi::default().addr_make("signaler_b")
}

fn challenger() -> Addr {
    MockApi::default().addr_make("challenger")
}

fn arbiter() -> Addr {
    MockApi::default().addr_make("arbiter")
}

fn random_addr(label: &str) -> Addr {
    MockApi::default().addr_make(label)
}

fn default_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        admin: admin().to_string(),
        activation_delay_seconds: Some(100), // short for tests
        challenge_window_seconds: Some(10_000),
        resolution_deadline_seconds: Some(1_000),
        challenge_bond_denom: Some("uregen".to_string()),
        challenge_bond_amount: Some(Uint128::zero()),
        decay_half_life_seconds: Some(1_000_000), // large so decay is negligible in tests
        default_min_stake: Some(Uint128::zero()),
    }
}

fn default_evidence() -> Evidence {
    Evidence {
        koi_links: vec!["koi://note/test".to_string()],
        ledger_refs: vec!["ledger://tx/1".to_string()],
        web_links: vec![],
    }
}

fn env_at(seconds: u64) -> cosmwasm_std::Env {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(seconds);
    env
}

/// Instantiate the contract and return deps.
fn setup() -> cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    cosmwasm_std::testing::MockQuerier,
> {
    let mut deps = mock_dependencies();
    let env = env_at(1_000_000);
    let info = message_info(&admin(), &[]);
    instantiate(deps.as_mut(), env, info, default_instantiate_msg()).unwrap();
    deps
}

fn submit_signal(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    >,
    sender: &Addr,
    level: u8,
    time: u64,
) -> u64 {
    let env = env_at(time);
    let info = message_info(sender, &[]);
    let resp = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitSignal {
            subject_type: SubjectType::Project,
            subject_id: "P-001".to_string(),
            category: "quality".to_string(),
            endorsement_level: level,
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Extract signal_id from attributes
    resp.attributes
        .iter()
        .find(|a| a.key == "signal_id")
        .unwrap()
        .value
        .parse::<u64>()
        .unwrap()
}

fn activate_signal(
    deps: &mut cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
    >,
    signal_id: u64,
    time: u64,
) {
    let env = env_at(time);
    let info = message_info(&random_addr("crank"), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::ActivateSignal { signal_id },
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Instantiation
// ---------------------------------------------------------------------------

#[test]
fn test_instantiate() {
    let deps = setup();
    let env = env_at(1_000_000);
    let res: ConfigResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(res.config.admin, admin());
    assert_eq!(res.config.activation_delay_seconds, 100);
    assert_eq!(res.config.challenge_window_seconds, 10_000);
    assert_eq!(res.config.resolution_deadline_seconds, 1_000);
}

// ---------------------------------------------------------------------------
// Signal submission
// ---------------------------------------------------------------------------

#[test]
fn test_submit_signal() {
    let mut deps = setup();
    let id = submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    assert_eq!(id, 1);

    // Query signal
    let env = env_at(1_000_000);
    let res: SignalResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::Signal { signal_id: 1 }).unwrap()).unwrap();
    assert_eq!(res.signal.endorsement_level, 4);
    assert!(matches!(res.signal.status, SignalStatus::Submitted));
    assert_eq!(res.signal.signaler, signaler_a());
}

#[test]
fn test_invalid_endorsement_level_zero() {
    let mut deps = setup();
    let env = env_at(1_000_000);
    let info = message_info(&signaler_a(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitSignal {
            subject_type: SubjectType::Project,
            subject_id: "P-001".to_string(),
            category: "quality".to_string(),
            endorsement_level: 0,
            evidence: default_evidence(),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidEndorsementLevel { level: 0 }
    );
}

#[test]
fn test_invalid_endorsement_level_six() {
    let mut deps = setup();
    let env = env_at(1_000_000);
    let info = message_info(&signaler_a(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitSignal {
            subject_type: SubjectType::Project,
            subject_id: "P-001".to_string(),
            category: "quality".to_string(),
            endorsement_level: 6,
            evidence: default_evidence(),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidEndorsementLevel { level: 6 }
    );
}

#[test]
fn test_submit_signal_no_evidence() {
    let mut deps = setup();
    let env = env_at(1_000_000);
    let info = message_info(&signaler_a(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitSignal {
            subject_type: SubjectType::Project,
            subject_id: "P-001".to_string(),
            category: "quality".to_string(),
            endorsement_level: 3,
            evidence: Evidence {
                koi_links: vec![],
                ledger_refs: vec![],
                web_links: vec!["https://example.com".to_string()],
            },
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InsufficientEvidence {});
}

// ---------------------------------------------------------------------------
// Activation
// ---------------------------------------------------------------------------

#[test]
fn test_activate_signal_before_delay() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);

    // Try to activate too early (delay is 100s, so at 1_000_050 should fail)
    let env = env_at(1_000_050);
    let info = message_info(&random_addr("crank"), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::ActivateSignal { signal_id: 1 },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::SignalNotYetActive { id: 1 });
}

#[test]
fn test_activate_signal_after_delay() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    let env = env_at(1_000_101);
    let res: SignalResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::Signal { signal_id: 1 }).unwrap()).unwrap();
    assert!(matches!(res.signal.status, SignalStatus::Active));
}

// ---------------------------------------------------------------------------
// Submitted signals do not contribute to score
// ---------------------------------------------------------------------------

#[test]
fn test_submitted_signal_does_not_score() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 5, 1_000_000);

    let env = env_at(1_000_000);
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(res.score, 0);
    assert_eq!(res.contributing_signals, 0);
    assert_eq!(res.total_signals, 1);
}

// ---------------------------------------------------------------------------
// Reputation score computation
// ---------------------------------------------------------------------------

#[test]
fn test_reputation_score_single_signal() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    let env = env_at(1_000_101);
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    // 4/5 = 0.8, scaled to 1000 = 800
    assert_eq!(res.score, 800);
    assert_eq!(res.contributing_signals, 1);
}

#[test]
fn test_reputation_score_multiple_signals() {
    let mut deps = setup();
    // Signal 1: level 5
    submit_signal(&mut deps, &signaler_a(), 5, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Signal 2: level 3
    submit_signal(&mut deps, &signaler_b(), 3, 1_000_000);
    activate_signal(&mut deps, 2, 1_000_101);

    let env = env_at(1_000_101);
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();

    // Same submit time, large half-life => equal decay
    // Average: (5/5 + 3/5) / 2 = (1.0 + 0.6) / 2 = 0.8 => 800
    assert_eq!(res.score, 800);
    assert_eq!(res.contributing_signals, 2);
}

// ---------------------------------------------------------------------------
// Withdrawal
// ---------------------------------------------------------------------------

#[test]
fn test_withdraw_signal() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Withdraw
    let env = env_at(1_000_200);
    let info = message_info(&signaler_a(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::WithdrawSignal { signal_id: 1 },
    )
    .unwrap();

    // Score should drop to 0
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.score, 0);
    assert_eq!(res.contributing_signals, 0);
}

#[test]
fn test_withdraw_not_owner() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);

    let env = env_at(1_000_200);
    let info = message_info(&signaler_b(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::WithdrawSignal { signal_id: 1 },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::NotSignalOwner { id: 1 });
}

// ---------------------------------------------------------------------------
// Challenge submission
// ---------------------------------------------------------------------------

#[test]
fn test_submit_challenge() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Signal should be Challenged
    let res: SignalResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::Signal { signal_id: 1 }).unwrap())
            .unwrap();
    assert!(matches!(res.signal.status, SignalStatus::Challenged));

    // Score should be 0 (paused)
    let score_res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(score_res.score, 0);
}

#[test]
fn test_challenge_self_signal() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    let env = env_at(1_000_200);
    let info = message_info(&signaler_a(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "I am challenging my own signal for some reason that I should not be able to.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::SelfChallenge {});
}

#[test]
fn test_challenge_no_evidence() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: Evidence {
                koi_links: vec![],
                ledger_refs: vec![],
                web_links: vec!["https://example.com".to_string()],
            },
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InsufficientEvidence {});
}

#[test]
fn test_challenge_rationale_too_short() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "Too short".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::RationaleTooShort {});
}

#[test]
fn test_challenge_expired_window() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Challenge window is 10_000s, submitted at 1_000_000
    // So window expires at 1_010_000
    let env = env_at(1_010_001);
    let info = message_info(&challenger(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::ChallengeWindowExpired { id: 1 });
}

#[test]
fn test_challenge_withdrawn_signal() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Withdraw
    let env = env_at(1_000_200);
    let info = message_info(&signaler_a(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::WithdrawSignal { signal_id: 1 },
    )
    .unwrap();

    // Try to challenge withdrawn signal
    let env = env_at(1_000_300);
    let info = message_info(&challenger(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::SignalNotChallengeable { id: 1 });
}

#[test]
fn test_challenge_already_challenged() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // First challenge
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Second challenge attempt
    let env = env_at(1_000_300);
    let info = message_info(&random_addr("another_challenger"), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "Another challenge on the same signal which should not be allowed because one is pending.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::SignalAlreadyChallenged { id: 1 });
}

#[test]
fn test_challenge_during_submitted_state() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 5, 1_000_000);

    // Challenge during SUBMITTED state (before activation)
    let env = env_at(1_000_050);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This signal was submitted by a known bad actor and should be challenged before activation.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Signal should be Challenged, score still 0 (never contributed)
    let res: SignalResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::Signal { signal_id: 1 }).unwrap())
            .unwrap();
    assert!(matches!(res.signal.status, SignalStatus::Challenged));
}

// ---------------------------------------------------------------------------
// Withdraw during challenge
// ---------------------------------------------------------------------------

#[test]
fn test_cannot_withdraw_while_challenged() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Challenge
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Try to withdraw
    let env = env_at(1_000_300);
    let info = message_info(&signaler_a(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::WithdrawSignal { signal_id: 1 },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::WithdrawWhileChallenged { id: 1 });
}

// ---------------------------------------------------------------------------
// Challenge resolution
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_challenge_valid() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Challenge
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Resolve as valid (admin)
    let env = env_at(1_000_500);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ResolveChallenge {
            challenge_id: 1,
            outcome_valid: true,
            rationale: "Signal verified correct.".to_string(),
        },
    )
    .unwrap();

    // Signal should be ResolvedValid
    let res: SignalResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::Signal { signal_id: 1 }).unwrap())
            .unwrap();
    assert!(matches!(res.signal.status, SignalStatus::ResolvedValid));

    // Score should be restored: 4/5 = 0.8 => 800
    let score_res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(score_res.score, 800);
}

#[test]
fn test_resolve_challenge_invalid() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 5, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Challenge
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on false claims that have been conclusively disproven by audit.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Resolve as invalid
    let env = env_at(1_000_500);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ResolveChallenge {
            challenge_id: 1,
            outcome_valid: false,
            rationale: "Confirmed invalid.".to_string(),
        },
    )
    .unwrap();

    // Signal should be ResolvedInvalid
    let res: SignalResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::Signal { signal_id: 1 }).unwrap())
            .unwrap();
    assert!(matches!(res.signal.status, SignalStatus::ResolvedInvalid));

    // Score should be 0 (permanently removed)
    let score_res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(score_res.score, 0);
}

#[test]
fn test_resolve_by_arbiter() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Add arbiter
    let env = env_at(1_000_150);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::AddArbiter {
            address: arbiter().to_string(),
        },
    )
    .unwrap();

    // Challenge
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Arbiter resolves
    let env = env_at(1_000_500);
    let info = message_info(&arbiter(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ResolveChallenge {
            challenge_id: 1,
            outcome_valid: true,
            rationale: "Signal verified by arbiter.".to_string(),
        },
    )
    .unwrap();

    let res: SignalResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::Signal { signal_id: 1 }).unwrap()).unwrap();
    assert!(matches!(res.signal.status, SignalStatus::ResolvedValid));
}

#[test]
fn test_resolve_by_unauthorized() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Challenge
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Non-admin/non-arbiter tries to resolve
    let env = env_at(1_000_500);
    let info = message_info(&random_addr("random"), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::ResolveChallenge {
            challenge_id: 1,
            outcome_valid: true,
            rationale: "Unauthorized resolution attempt.".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::NotResolver {});
}

// ---------------------------------------------------------------------------
// Escalation
// ---------------------------------------------------------------------------

#[test]
fn test_escalate_after_deadline() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Challenge at 1_000_200
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Escalation deadline = 1_000_200 + 1_000 = 1_001_200
    // Try before deadline
    let env = env_at(1_001_100);
    let info = message_info(&random_addr("crank"), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::EscalateChallenge { challenge_id: 1 },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::DeadlineNotExceeded { id: 1 });

    // After deadline
    let env = env_at(1_001_201);
    let info = message_info(&random_addr("crank"), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::EscalateChallenge { challenge_id: 1 },
    )
    .unwrap();

    // Signal should be Escalated
    let res: SignalResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::Signal { signal_id: 1 }).unwrap())
            .unwrap();
    assert!(matches!(res.signal.status, SignalStatus::Escalated));

    // Score still 0 (paused during escalation)
    let score_res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(score_res.score, 0);
}

// ---------------------------------------------------------------------------
// Admin invalidation
// ---------------------------------------------------------------------------

#[test]
fn test_admin_invalidate() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    let env = env_at(1_000_200);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::InvalidateSignal {
            signal_id: 1,
            rationale: "Administrative override due to policy violation.".to_string(),
        },
    )
    .unwrap();

    let res: SignalResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::Signal { signal_id: 1 }).unwrap())
            .unwrap();
    assert!(matches!(res.signal.status, SignalStatus::Invalidated));

    // Score 0
    let score_res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(score_res.score, 0);
}

#[test]
fn test_invalidate_not_admin() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);

    let env = env_at(1_000_200);
    let info = message_info(&signaler_b(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::InvalidateSignal {
            signal_id: 1,
            rationale: "Unauthorized invalidation attempt.".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

#[test]
fn test_invalidate_no_rationale() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);

    let env = env_at(1_000_200);
    let info = message_info(&admin(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::InvalidateSignal {
            signal_id: 1,
            rationale: "".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InvalidationRationaleRequired {});
}

// ---------------------------------------------------------------------------
// Multi-signal: one challenged, others unaffected
// ---------------------------------------------------------------------------

#[test]
fn test_multi_signal_partial_challenge() {
    let mut deps = setup();

    // Signal 1: level 5 from signaler_a
    submit_signal(&mut deps, &signaler_a(), 5, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Signal 2: level 3 from signaler_b
    submit_signal(&mut deps, &signaler_b(), 3, 1_000_000);
    activate_signal(&mut deps, 2, 1_000_101);

    // Score: (5/5 + 3/5) / 2 = 0.8 => 800
    let env = env_at(1_000_101);
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.score, 800);

    // Challenge signal 2 only
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 2,
            rationale: "Signal B is based on incorrect methodology assessment and should be challenged.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Score should only reflect signal 1: 5/5 = 1.0 => 1000
    let env = env_at(1_000_200);
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.score, 1000);
    assert_eq!(res.contributing_signals, 1);

    // Resolve challenge as valid -- signal 2 restored
    let env = env_at(1_000_500);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ResolveChallenge {
            challenge_id: 1,
            outcome_valid: true,
            rationale: "Methodology confirmed correct.".to_string(),
        },
    )
    .unwrap();

    // Score back to ~800
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.score, 800);
    assert_eq!(res.contributing_signals, 2);
}

// ---------------------------------------------------------------------------
// Config updates
// ---------------------------------------------------------------------------

#[test]
fn test_update_config() {
    let mut deps = setup();

    let env = env_at(1_000_000);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::UpdateConfig {
            activation_delay_seconds: Some(200),
            challenge_window_seconds: None,
            resolution_deadline_seconds: None,
            challenge_bond_denom: None,
            challenge_bond_amount: Some(Uint128::new(1000)),
            decay_half_life_seconds: None,
            default_min_stake: None,
        },
    )
    .unwrap();

    let res: ConfigResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(res.config.activation_delay_seconds, 200);
    assert_eq!(res.config.challenge_bond_amount, Uint128::new(1000));
}

#[test]
fn test_update_config_not_admin() {
    let mut deps = setup();

    let env = env_at(1_000_000);
    let info = message_info(&signaler_a(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::UpdateConfig {
            activation_delay_seconds: Some(200),
            challenge_window_seconds: None,
            resolution_deadline_seconds: None,
            challenge_bond_denom: None,
            challenge_bond_amount: None,
            decay_half_life_seconds: None,
            default_min_stake: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
}

// ---------------------------------------------------------------------------
// Category min stake
// ---------------------------------------------------------------------------

#[test]
fn test_set_category_min_stake() {
    let mut deps = setup();

    let env = env_at(1_000_000);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::SetCategoryMinStake {
            category: "quality".to_string(),
            min_stake: Uint128::new(5000),
        },
    )
    .unwrap();

    let res: CategoryMinStakeResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::CategoryMinStake {
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.min_stake, Uint128::new(5000));
}

// ---------------------------------------------------------------------------
// Arbiter management
// ---------------------------------------------------------------------------

#[test]
fn test_add_remove_arbiter() {
    let mut deps = setup();

    // Add arbiter
    let env = env_at(1_000_000);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::AddArbiter {
            address: arbiter().to_string(),
        },
    )
    .unwrap();

    let res: ConfigResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(res.config.arbiters.len(), 1);
    assert_eq!(res.config.arbiters[0], arbiter());

    // Remove arbiter
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::RemoveArbiter {
            address: arbiter().to_string(),
        },
    )
    .unwrap();

    let res: ConfigResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(res.config.arbiters.len(), 0);
}

// ---------------------------------------------------------------------------
// Active challenges query
// ---------------------------------------------------------------------------

#[test]
fn test_query_active_challenges() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    submit_signal(&mut deps, &signaler_b(), 3, 1_000_000);
    activate_signal(&mut deps, 2, 1_000_101);

    // Challenge signal 1
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    let env = env_at(1_000_300);
    let res: ActiveChallengesResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ActiveChallenges {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.challenges.len(), 1);
    assert_eq!(res.challenges[0].signal_id, 1);
}

// ---------------------------------------------------------------------------
// Signals by subject query
// ---------------------------------------------------------------------------

#[test]
fn test_query_signals_by_subject() {
    let mut deps = setup();
    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    submit_signal(&mut deps, &signaler_b(), 3, 1_000_000);

    let env = env_at(1_000_000);
    let res: SignalsBySubjectResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SignalsBySubject {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.signals.len(), 2);
}

// ---------------------------------------------------------------------------
// Bond enforcement (v1-ready)
// ---------------------------------------------------------------------------

#[test]
fn test_challenge_bond_required() {
    let mut deps = setup();

    // Update config to require bond
    let env = env_at(1_000_000);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::UpdateConfig {
            activation_delay_seconds: None,
            challenge_window_seconds: None,
            resolution_deadline_seconds: None,
            challenge_bond_denom: Some("uregen".to_string()),
            challenge_bond_amount: Some(Uint128::new(1000)),
            decay_half_life_seconds: None,
            default_min_stake: None,
        },
    )
    .unwrap();

    submit_signal(&mut deps, &signaler_a(), 4, 1_000_000);
    activate_signal(&mut deps, 1, 1_000_101);

    // Challenge without bond
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InsufficientBond {
            required: "1000".to_string(),
            sent: "0".to_string(),
        }
    );

    // Challenge with bond
    let env = env_at(1_000_200);
    let info = message_info(
        &challenger(),
        &[Coin {
            denom: "uregen".to_string(),
            amount: Uint128::new(1000),
        }],
    );
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This endorsement is based on outdated information that no longer applies to the project.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Full lifecycle: submit -> activate -> challenge -> resolve invalid -> score gone
// ---------------------------------------------------------------------------

#[test]
fn test_full_lifecycle_invalid() {
    let mut deps = setup();

    // Submit
    let id = submit_signal(&mut deps, &signaler_a(), 5, 1_000_000);
    assert_eq!(id, 1);

    // Activate
    activate_signal(&mut deps, 1, 1_000_101);

    // Verify score
    let env = env_at(1_000_101);
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.score, 1000);

    // Challenge
    let env = env_at(1_000_200);
    let info = message_info(&challenger(), &[]);
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SubmitChallenge {
            signal_id: 1,
            rationale: "This signal was submitted with fabricated evidence and should be invalidated.".to_string(),
            evidence: default_evidence(),
        },
    )
    .unwrap();

    // Score paused
    let env = env_at(1_000_200);
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.score, 0);

    // Resolve invalid
    let env = env_at(1_000_500);
    let info = message_info(&admin(), &[]);
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ResolveChallenge {
            challenge_id: 1,
            outcome_valid: false,
            rationale: "Fabricated evidence confirmed.".to_string(),
        },
    )
    .unwrap();

    // Score permanently 0
    let res: ReputationScoreResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::ReputationScore {
                subject_type: SubjectType::Project,
                subject_id: "P-001".to_string(),
                category: "quality".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.score, 0);
    assert_eq!(res.contributing_signals, 0);
}
