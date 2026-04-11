use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

// ── Constants ─────────────────────────────────────────────────────────

const DENOM: &str = "uregen";

// ── Address helpers ───────────────────────────────────────────────────

fn addr(label: &str) -> Addr {
    MockApi::default().addr_make(label)
}

// ── Contract wrappers ─────────────────────────────────────────────────

fn attestation_bonding_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        attestation_bonding::contract::execute,
        attestation_bonding::contract::instantiate,
        attestation_bonding::contract::query,
    ))
}

fn credit_class_voting_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        credit_class_voting::contract::execute,
        credit_class_voting::contract::instantiate,
        credit_class_voting::contract::query,
    ))
}

fn marketplace_curation_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        marketplace_curation::contract::execute,
        marketplace_curation::contract::instantiate,
        marketplace_curation::contract::query,
    ))
}

fn service_escrow_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        service_escrow::contract::execute,
        service_escrow::contract::instantiate,
        service_escrow::contract::query,
    ))
}

fn contribution_rewards_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        contribution_rewards::contract::execute,
        contribution_rewards::contract::instantiate,
        contribution_rewards::contract::query,
    ))
}

fn validator_governance_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        validator_governance::contract::execute,
        validator_governance::contract::instantiate,
        validator_governance::contract::query,
    ))
}

// ── App builder helper ────────────────────────────────────────────────

/// Build an App with initial balances for the given addresses.
fn build_app(balances: &[(&Addr, u128)]) -> App {
    AppBuilder::new().build(|router, _api, storage| {
        for (addr, amount) in balances {
            router
                .bank
                .init_balance(
                    storage,
                    addr,
                    vec![Coin::new(*amount, DENOM)],
                )
                .unwrap();
        }
    })
}

// ═══════════════════════════════════════════════════════════════════════
// Test 1: Attestation -> Quality Score -> Curation Flow (M008 + M011)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_attestation_quality_score_curation_flow() {
    let admin = addr("admin");
    let arbiter = addr("arbiter");
    let community = addr("community");
    let attester = addr("attester");
    let curator = addr("curator");
    let challenger = addr("challenger");

    let mut app = build_app(&[
        (&admin, 10_000_000_000),
        (&attester, 10_000_000_000),
        (&curator, 10_000_000_000),
        (&challenger, 10_000_000_000),
    ]);

    // ── Deploy attestation-bonding ───────────────────────────────────
    let ab_code_id = app.store_code(attestation_bonding_contract());
    let ab_addr = app
        .instantiate_contract(
            ab_code_id,
            admin.clone(),
            &attestation_bonding::msg::InstantiateMsg {
                arbiter_dao: arbiter.to_string(),
                community_pool: community.to_string(),
                denom: DENOM.to_string(),
                challenge_deposit_ratio_bps: Some(1000), // 10%
                arbiter_fee_ratio_bps: Some(500),         // 5%
                activation_delay_seconds: Some(100),
            },
            &[],
            "attestation-bonding",
            None,
        )
        .unwrap();

    // ── Deploy marketplace-curation ──────────────────────────────────
    let mc_code_id = app.store_code(marketplace_curation_contract());
    let mc_addr = app
        .instantiate_contract(
            mc_code_id,
            admin.clone(),
            &marketplace_curation::msg::InstantiateMsg {
                community_pool: community.to_string(),
                min_curation_bond: Some(Uint128::new(1_000_000_000)),
                curation_fee_rate_bps: Some(50),
                challenge_deposit: Some(Uint128::new(100_000_000)),
                slash_percentage_bps: Some(2000),
                activation_delay_seconds: Some(100),
                unbonding_period_seconds: Some(200),
                bond_top_up_window_seconds: Some(100),
                min_quality_score: Some(300),
                max_collections_per_curator: Some(5),
                denom: DENOM.to_string(),
            },
            &[],
            "marketplace-curation",
            None,
        )
        .unwrap();

    // ── Step 1: Create attestation with bond ─────────────────────────
    let create_msg = attestation_bonding::msg::ExecuteMsg::CreateAttestation {
        attestation_type: attestation_bonding::state::AttestationType::ProjectBoundary,
        iri: "regen:attestation/boundary/1".to_string(),
        beneficiary: None,
    };
    app.execute_contract(
        attester.clone(),
        ab_addr.clone(),
        &create_msg,
        &[Coin::new(500_000_000u128, DENOM)],
    )
    .unwrap();

    // Verify attestation is Bonded
    let att_resp: attestation_bonding::msg::AttestationResponse = app
        .wrap()
        .query_wasm_smart(
            ab_addr.clone(),
            &attestation_bonding::msg::QueryMsg::Attestation { attestation_id: 1 },
        )
        .unwrap();
    assert_eq!(
        att_resp.attestation.status,
        attestation_bonding::state::AttestationStatus::Bonded
    );

    // ── Step 2: Advance time and activate attestation ────────────────
    app.update_block(|block| {
        block.time = block.time.plus_seconds(101);
    });

    app.execute_contract(
        admin.clone(),
        ab_addr.clone(),
        &attestation_bonding::msg::ExecuteMsg::ActivateAttestation { attestation_id: 1 },
        &[],
    )
    .unwrap();

    let att_resp: attestation_bonding::msg::AttestationResponse = app
        .wrap()
        .query_wasm_smart(
            ab_addr.clone(),
            &attestation_bonding::msg::QueryMsg::Attestation { attestation_id: 1 },
        )
        .unwrap();
    assert_eq!(
        att_resp.attestation.status,
        attestation_bonding::state::AttestationStatus::Active
    );

    // ── Step 3: Submit a quality score for a batch on curation ───────
    let batch_denom = "regen:batch/C01-001-20250101-20251231-001";
    app.execute_contract(
        admin.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::SubmitQualityScore {
            batch_denom: batch_denom.to_string(),
            score: 750,
            confidence: 900,
        },
        &[],
    )
    .unwrap();

    // Verify quality score
    let qs_resp: marketplace_curation::msg::QualityScoreResponse = app
        .wrap()
        .query_wasm_smart(
            mc_addr.clone(),
            &marketplace_curation::msg::QueryMsg::QualityScore {
                batch_denom: batch_denom.to_string(),
            },
        )
        .unwrap();
    assert_eq!(qs_resp.quality_score.as_ref().unwrap().score, 750);

    // ── Step 4: Create a curated collection ──────────────────────────
    app.execute_contract(
        curator.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::CreateCollection {
            name: "Verified Carbon Credits".to_string(),
            criteria: "Score >= 700, active attestation required".to_string(),
        },
        &[Coin::new(1_000_000_000u128, DENOM)],
    )
    .unwrap();

    // Activate the collection after delay
    app.update_block(|block| {
        block.time = block.time.plus_seconds(101);
    });

    app.execute_contract(
        curator.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::ActivateCollection { collection_id: 1 },
        &[],
    )
    .unwrap();

    // ── Step 5: Add batch to collection (passes quality check) ───────
    app.execute_contract(
        curator.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::AddToCollection {
            collection_id: 1,
            batch_denom: batch_denom.to_string(),
        },
        &[],
    )
    .unwrap();

    // Verify batch is in collection
    let coll_resp: marketplace_curation::msg::CollectionResponse = app
        .wrap()
        .query_wasm_smart(
            mc_addr.clone(),
            &marketplace_curation::msg::QueryMsg::Collection { collection_id: 1 },
        )
        .unwrap();
    assert!(coll_resp.collection.batches.contains(&batch_denom.to_string()));

    // ── Step 6: Challenge the batch inclusion ────────────────────────
    app.execute_contract(
        challenger.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::ChallengeBatchInclusion {
            collection_id: 1,
            batch_denom: batch_denom.to_string(),
            evidence: "The boundary attestation has not been independently verified".to_string(),
        },
        &[Coin::new(100_000_000u128, DENOM)],
    )
    .unwrap();

    // Verify collection is UnderReview
    let coll_resp: marketplace_curation::msg::CollectionResponse = app
        .wrap()
        .query_wasm_smart(
            mc_addr.clone(),
            &marketplace_curation::msg::QueryMsg::Collection { collection_id: 1 },
        )
        .unwrap();
    assert_eq!(
        coll_resp.collection.status,
        marketplace_curation::state::CollectionStatus::UnderReview
    );

    // ── Step 7: Resolve the challenge (curator wins) ─────────────────
    app.execute_contract(
        admin.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::ResolveChallenge {
            challenge_id: 1,
            resolution: marketplace_curation::state::ChallengeResolution::CuratorWins,
        },
        &[],
    )
    .unwrap();

    // After curator wins, collection should go back to Active
    let coll_resp: marketplace_curation::msg::CollectionResponse = app
        .wrap()
        .query_wasm_smart(
            mc_addr.clone(),
            &marketplace_curation::msg::QueryMsg::Collection { collection_id: 1 },
        )
        .unwrap();
    assert_eq!(
        coll_resp.collection.status,
        marketplace_curation::state::CollectionStatus::Active
    );

    // Verify challenge resolved
    let ch_resp: marketplace_curation::msg::ChallengeResponse = app
        .wrap()
        .query_wasm_smart(
            mc_addr.clone(),
            &marketplace_curation::msg::QueryMsg::Challenge { challenge_id: 1 },
        )
        .unwrap();
    assert_eq!(
        ch_resp.challenge.resolution,
        Some(marketplace_curation::state::ChallengeResolution::CuratorWins)
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 2: Credit Class Approval Flow (M001-ENH)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_credit_class_approval_flow() {
    let admin = addr("admin");
    let agent = addr("registry_agent");
    let proposer = addr("proposer");
    let voter1 = addr("voter1");
    let voter2 = addr("voter2");
    let voter3 = addr("voter3");

    let mut app = build_app(&[
        (&admin, 10_000_000_000),
        (&proposer, 10_000_000_000),
    ]);

    // ── Deploy credit-class-voting ───────────────────────────────────
    let ccv_code_id = app.store_code(credit_class_voting_contract());
    let ccv_addr = app
        .instantiate_contract(
            ccv_code_id,
            admin.clone(),
            &credit_class_voting::msg::InstantiateMsg {
                registry_agent: agent.to_string(),
                deposit_amount: Some(Uint128::new(1_000_000_000)),
                denom: Some(DENOM.to_string()),
                voting_period_seconds: Some(200), // short for testing
                agent_review_timeout_seconds: Some(100),
                override_window_seconds: Some(50),
                community_pool: None,
            },
            &[],
            "credit-class-voting",
            None,
        )
        .unwrap();

    // ── Step 1: Submit proposal with deposit ─────────────────────────
    app.execute_contract(
        proposer.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::SubmitProposal {
            admin_address: admin.to_string(),
            credit_type: "C".to_string(),
            methodology_iri: "regen:methodology/carbon-v1".to_string(),
        },
        &[Coin::new(1_000_000_000u128, DENOM)],
    )
    .unwrap();

    // Verify proposal is in AgentReview
    let prop_resp: credit_class_voting::msg::ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            ccv_addr.clone(),
            &credit_class_voting::msg::QueryMsg::Proposal { proposal_id: 1 },
        )
        .unwrap();
    assert_eq!(
        prop_resp.proposal.status,
        credit_class_voting::state::ProposalStatus::AgentReview
    );

    // ── Step 2: Agent submits score > 700 (auto-advances to Voting) ──
    app.execute_contract(
        agent.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::SubmitAgentScore {
            proposal_id: 1,
            score: 800,
            confidence: 900,
            recommendation: credit_class_voting::state::AgentRecommendation::Approve,
        },
        &[],
    )
    .unwrap();

    let prop_resp: credit_class_voting::msg::ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            ccv_addr.clone(),
            &credit_class_voting::msg::QueryMsg::Proposal { proposal_id: 1 },
        )
        .unwrap();
    assert_eq!(
        prop_resp.proposal.status,
        credit_class_voting::state::ProposalStatus::Voting
    );
    assert_eq!(prop_resp.proposal.agent_score, Some(800));

    // ── Step 3: Cast votes (yes majority) ────────────────────────────
    app.execute_contract(
        voter1.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::CastVote {
            proposal_id: 1,
            vote_yes: true,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        voter2.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::CastVote {
            proposal_id: 1,
            vote_yes: true,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        voter3.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::CastVote {
            proposal_id: 1,
            vote_yes: false,
        },
        &[],
    )
    .unwrap();

    // Verify vote tallies
    let prop_resp: credit_class_voting::msg::ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            ccv_addr.clone(),
            &credit_class_voting::msg::QueryMsg::Proposal { proposal_id: 1 },
        )
        .unwrap();
    assert_eq!(prop_resp.proposal.yes_votes, 2);
    assert_eq!(prop_resp.proposal.no_votes, 1);

    // ── Step 4: Advance past voting period and finalize ──────────────
    app.update_block(|block| {
        block.time = block.time.plus_seconds(201);
    });

    let proposer_balance_before = app.wrap().query_balance(&proposer, DENOM).unwrap();

    app.execute_contract(
        admin.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::FinalizeProposal { proposal_id: 1 },
        &[],
    )
    .unwrap();

    // Verify: Approved, deposit refunded
    let prop_resp: credit_class_voting::msg::ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            ccv_addr.clone(),
            &credit_class_voting::msg::QueryMsg::Proposal { proposal_id: 1 },
        )
        .unwrap();
    assert_eq!(
        prop_resp.proposal.status,
        credit_class_voting::state::ProposalStatus::Approved
    );

    let proposer_balance_after = app.wrap().query_balance(&proposer, DENOM).unwrap();
    assert_eq!(
        proposer_balance_after.amount - proposer_balance_before.amount,
        Uint128::new(1_000_000_000),
        "Deposit should be refunded on approval"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 3: Service Escrow Lifecycle (M009)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_service_escrow_lifecycle() {
    let admin = addr("admin");
    let arbiter = addr("arbiter");
    let community = addr("community");
    let client = addr("client");
    let provider = addr("provider");

    let mut app = build_app(&[
        (&admin, 10_000_000_000),
        (&client, 10_000_000_000),
        (&provider, 10_000_000_000),
    ]);

    // ── Deploy service-escrow ────────────────────────────────────────
    let se_code_id = app.store_code(service_escrow_contract());
    let se_addr = app
        .instantiate_contract(
            se_code_id,
            admin.clone(),
            &service_escrow::msg::InstantiateMsg {
                arbiter_dao: arbiter.to_string(),
                community_pool: community.to_string(),
                provider_bond_ratio_bps: Some(1000), // 10%
                platform_fee_rate_bps: Some(100),     // 1%
                cancellation_fee_rate_bps: Some(200), // 2%
                arbiter_fee_rate_bps: Some(500),      // 5%
                review_period_seconds: Some(200),
                max_milestones: Some(10),
                max_revisions: Some(3),
                denom: DENOM.to_string(),
            },
            &[],
            "service-escrow",
            None,
        )
        .unwrap();

    // ── Step 1: Client proposes agreement ────────────────────────────
    let milestone1_payment = Uint128::new(600_000_000); // 600 REGEN
    let milestone2_payment = Uint128::new(400_000_000); // 400 REGEN
    let total_escrow = milestone1_payment + milestone2_payment; // 1000 REGEN

    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::ProposeAgreement {
            provider: provider.to_string(),
            service_type: "Ecological monitoring".to_string(),
            description: "6-month soil carbon measurement program".to_string(),
            milestones: vec![
                service_escrow::msg::MilestoneInput {
                    description: "Install sensors and baseline measurement".to_string(),
                    payment_amount: milestone1_payment,
                },
                service_escrow::msg::MilestoneInput {
                    description: "Final report and data delivery".to_string(),
                    payment_amount: milestone2_payment,
                },
            ],
        },
        &[],
    )
    .unwrap();

    // Verify agreement is Proposed
    let agree_resp: service_escrow::msg::AgreementResponse = app
        .wrap()
        .query_wasm_smart(
            se_addr.clone(),
            &service_escrow::msg::QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap();
    assert_eq!(
        agree_resp.agreement.status,
        service_escrow::state::AgreementStatus::Proposed
    );
    assert_eq!(agree_resp.agreement.milestones.len(), 2);

    // ── Step 2: Provider accepts with bond (10% of escrow = 100M) ───
    let bond_amount = total_escrow.multiply_ratio(1000u128, 10_000u128); // 10%
    app.execute_contract(
        provider.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::AcceptAgreement { agreement_id: 1 },
        &[Coin::new(bond_amount.u128(), DENOM)],
    )
    .unwrap();

    // ── Step 3: Client funds the escrow ──────────────────────────────
    // Fund slightly more than escrow to cover the completion fee (1% of escrow)
    let completion_fee = total_escrow.multiply_ratio(100u128, 10_000u128); // 1%
    let fund_amount = total_escrow + completion_fee;
    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::FundAgreement { agreement_id: 1 },
        &[Coin::new(fund_amount.u128(), DENOM)],
    )
    .unwrap();

    // ── Step 4: Start the agreement ──────────────────────────────────
    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::StartAgreement { agreement_id: 1 },
        &[],
    )
    .unwrap();

    let agree_resp: service_escrow::msg::AgreementResponse = app
        .wrap()
        .query_wasm_smart(
            se_addr.clone(),
            &service_escrow::msg::QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap();
    assert_eq!(
        agree_resp.agreement.status,
        service_escrow::state::AgreementStatus::InProgress
    );

    // ── Step 5: Provider submits milestone 0 ─────────────────────────
    app.execute_contract(
        provider.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::SubmitMilestone {
            agreement_id: 1,
            milestone_index: 0,
            deliverable_iri: "regen:deliverable/sensors-installed".to_string(),
        },
        &[],
    )
    .unwrap();

    // ── Step 6: Client approves milestone 0 ──────────────────────────
    let provider_balance_before = app.wrap().query_balance(&provider, DENOM).unwrap();

    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::ApproveMilestone {
            agreement_id: 1,
            milestone_index: 0,
        },
        &[],
    )
    .unwrap();

    // Provider should receive milestone payment minus platform fee
    let provider_balance_after = app.wrap().query_balance(&provider, DENOM).unwrap();
    let provider_received = provider_balance_after.amount - provider_balance_before.amount;
    // Payment is 600M, platform fee is 1% = 6M, so provider gets 594M
    assert_eq!(provider_received, Uint128::new(594_000_000));

    // ── Step 7: Submit and approve milestone 1 to complete ───────────
    app.execute_contract(
        provider.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::SubmitMilestone {
            agreement_id: 1,
            milestone_index: 1,
            deliverable_iri: "regen:deliverable/final-report".to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::ApproveMilestone {
            agreement_id: 1,
            milestone_index: 1,
        },
        &[],
    )
    .unwrap();

    // Verify agreement completed
    let agree_resp: service_escrow::msg::AgreementResponse = app
        .wrap()
        .query_wasm_smart(
            se_addr.clone(),
            &service_escrow::msg::QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap();
    assert_eq!(
        agree_resp.agreement.status,
        service_escrow::state::AgreementStatus::Completed
    );

    // Verify escrow balance
    let escrow_resp: service_escrow::msg::EscrowBalanceResponse = app
        .wrap()
        .query_wasm_smart(
            se_addr.clone(),
            &service_escrow::msg::QueryMsg::EscrowBalance { agreement_id: 1 },
        )
        .unwrap();
    assert_eq!(escrow_resp.remaining_escrow, Uint128::zero());
}

// ═══════════════════════════════════════════════════════════════════════
// Test 4: Validator Governance + Compensation (M014)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_validator_governance_and_compensation() {
    let admin = addr("admin");
    let community = addr("community");
    let val1 = addr("validator_infra_1");
    let val2 = addr("validator_refi_1");
    let val3 = addr("validator_eco_1");
    let participant = addr("participant");

    let mut app = build_app(&[
        (&admin, 50_000_000_000),
        (&community, 50_000_000_000),
    ]);

    // ── Deploy validator-governance ──────────────────────────────────
    let vg_code_id = app.store_code(validator_governance_contract());
    let vg_addr = app
        .instantiate_contract(
            vg_code_id,
            admin.clone(),
            &validator_governance::msg::InstantiateMsg {
                min_validators: Some(1),       // low for testing
                max_validators: Some(21),
                term_length_seconds: Some(1000),
                probation_period_seconds: Some(100),
                min_uptime_bps: Some(9950),
                performance_threshold_bps: Some(7000),
                uptime_weight_bps: Some(4000),
                governance_weight_bps: Some(3000),
                ecosystem_weight_bps: Some(3000),
                base_compensation_share_bps: Some(9000),
                performance_bonus_share_bps: Some(1000),
                min_per_category: Some(0), // no per-category minimum for test
                denom: DENOM.to_string(),
            },
            &[],
            "validator-governance",
            None,
        )
        .unwrap();

    // ── Deploy contribution-rewards ──────────────────────────────────
    let cr_code_id = app.store_code(contribution_rewards_contract());
    let cr_addr = app
        .instantiate_contract(
            cr_code_id,
            admin.clone(),
            &contribution_rewards::msg::InstantiateMsg {
                community_pool_addr: community.to_string(),
                denom: DENOM.to_string(),
            },
            &[],
            "contribution-rewards",
            None,
        )
        .unwrap();

    // ── Step 1: Apply, approve, activate 3 validators (one per category)
    // Validator 1 — InfrastructureBuilders
    app.execute_contract(
        val1.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ApplyForValidator {
            category: validator_governance::state::ValidatorCategory::InfrastructureBuilders,
            application_data: "Core developer, 5 years experience".to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ApproveValidator {
            applicant: val1.to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ActivateValidator {
            validator: val1.to_string(),
        },
        &[],
    )
    .unwrap();

    // Validator 2 — TrustedRefiPartners
    app.execute_contract(
        val2.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ApplyForValidator {
            category: validator_governance::state::ValidatorCategory::TrustedRefiPartners,
            application_data: "ReFi partner, Toucan Protocol integrator".to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ApproveValidator {
            applicant: val2.to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ActivateValidator {
            validator: val2.to_string(),
        },
        &[],
    )
    .unwrap();

    // Validator 3 — EcologicalDataStewards
    app.execute_contract(
        val3.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ApplyForValidator {
            category: validator_governance::state::ValidatorCategory::EcologicalDataStewards,
            application_data: "Ecological data scientist, peer-reviewed publications".to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ApproveValidator {
            applicant: val3.to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::ActivateValidator {
            validator: val3.to_string(),
        },
        &[],
    )
    .unwrap();

    // Verify all 3 active
    let state_resp: validator_governance::msg::ModuleStateResponse = app
        .wrap()
        .query_wasm_smart(
            vg_addr.clone(),
            &validator_governance::msg::QueryMsg::ModuleState {},
        )
        .unwrap();
    assert_eq!(state_resp.state.total_active, 3);

    // Verify composition breakdown
    let comp_resp: validator_governance::msg::CompositionBreakdownResponse = app
        .wrap()
        .query_wasm_smart(
            vg_addr.clone(),
            &validator_governance::msg::QueryMsg::CompositionBreakdown {},
        )
        .unwrap();
    assert_eq!(comp_resp.infrastructure_builders, 1);
    assert_eq!(comp_resp.trusted_refi_partners, 1);
    assert_eq!(comp_resp.ecological_data_stewards, 1);

    // ── Step 2: Submit performance reports ────────────────────────────
    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::SubmitPerformanceReport {
            validator: val1.to_string(),
            uptime_bps: 9990,
            governance_participation_bps: 8000,
            ecosystem_contribution_bps: 7000,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::SubmitPerformanceReport {
            validator: val2.to_string(),
            uptime_bps: 9970,
            governance_participation_bps: 9000,
            ecosystem_contribution_bps: 8500,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::SubmitPerformanceReport {
            validator: val3.to_string(),
            uptime_bps: 9980,
            governance_participation_bps: 7500,
            ecosystem_contribution_bps: 9000,
        },
        &[],
    )
    .unwrap();

    // ── Step 3: Fund the validator fund and distribute compensation ──
    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::UpdateValidatorFund {},
        &[Coin::new(3_000_000_000u128, DENOM)],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::DistributeCompensation {},
        &[],
    )
    .unwrap();

    // Verify each validator has compensation due
    for val_addr in [&val1, &val2, &val3] {
        let val_resp: validator_governance::msg::ValidatorResponse = app
            .wrap()
            .query_wasm_smart(
                vg_addr.clone(),
                &validator_governance::msg::QueryMsg::Validator {
                    address: val_addr.to_string(),
                },
            )
            .unwrap();
        assert!(
            !val_resp.validator.compensation_due.is_zero(),
            "Validator {} should have compensation due",
            val_addr
        );
    }

    // ── Step 4: Record activity and trigger distribution on contribution-rewards
    // Initialize mechanism
    app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::InitializeMechanism {},
        &[],
    )
    .unwrap();

    // Activate distribution
    app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::ActivateDistribution {},
        &[],
    )
    .unwrap();

    // Record activity for a participant
    app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::RecordActivity {
            participant: participant.to_string(),
            credit_purchase_value: Uint128::new(500_000_000),
            credit_retirement_value: Uint128::new(200_000_000),
            platform_facilitation_value: Uint128::new(100_000_000),
            governance_votes: 5,
            proposal_credits: 200,
        },
        &[],
    )
    .unwrap();

    // Verify mechanism state before distribution
    let mech_resp: contribution_rewards::msg::MechanismStateResponse = app
        .wrap()
        .query_wasm_smart(
            cr_addr.clone(),
            &contribution_rewards::msg::QueryMsg::MechanismState {},
        )
        .unwrap();
    let current_period = mech_resp.current_period;

    // Trigger distribution with community pool inflow
    let dist_res = app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::TriggerDistribution {
            community_pool_inflow: Uint128::new(1_000_000_000),
        },
        &[Coin::new(1_000_000_000u128, DENOM)],
    );
    assert!(dist_res.is_ok(), "TriggerDistribution failed: {:?}", dist_res.err());

    // Verify distribution record exists (use the period we read before triggering)
    let dist_resp: contribution_rewards::msg::DistributionRecordResponse = app
        .wrap()
        .query_wasm_smart(
            cr_addr.clone(),
            &contribution_rewards::msg::QueryMsg::DistributionRecord { period: current_period },
        )
        .unwrap();
    assert_eq!(
        dist_resp.record.community_pool_inflow,
        Uint128::new(1_000_000_000)
    );
    assert!(!dist_resp.record.activity_pool.is_zero());
}

// ═══════════════════════════════════════════════════════════════════════
// Test 5: Full Ecosystem Flow — all 6 contracts
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_full_ecosystem_flow() {
    let admin = addr("admin");
    let arbiter = addr("arbiter");
    let community = addr("community");
    let agent = addr("registry_agent");
    let attester = addr("attester");
    let curator = addr("curator");
    let proposer = addr("proposer");
    let voter1 = addr("voter1");
    let voter2 = addr("voter2");
    let client = addr("client");
    let provider = addr("provider");
    let participant = addr("participant");
    let val1 = addr("val_infra");
    let val2 = addr("val_refi");
    let val3 = addr("val_eco");

    let mut app = build_app(&[
        (&admin, 100_000_000_000),
        (&attester, 10_000_000_000),
        (&curator, 10_000_000_000),
        (&proposer, 10_000_000_000),
        (&client, 10_000_000_000),
        (&provider, 10_000_000_000),
        (&community, 50_000_000_000),
    ]);

    // ── Deploy all 6 contracts ───────────────────────────────────────

    let ab_code_id = app.store_code(attestation_bonding_contract());
    let ab_addr = app
        .instantiate_contract(
            ab_code_id,
            admin.clone(),
            &attestation_bonding::msg::InstantiateMsg {
                arbiter_dao: arbiter.to_string(),
                community_pool: community.to_string(),
                denom: DENOM.to_string(),
                challenge_deposit_ratio_bps: Some(1000),
                arbiter_fee_ratio_bps: Some(500),
                activation_delay_seconds: Some(100),
            },
            &[],
            "attestation-bonding",
            None,
        )
        .unwrap();

    let mc_code_id = app.store_code(marketplace_curation_contract());
    let mc_addr = app
        .instantiate_contract(
            mc_code_id,
            admin.clone(),
            &marketplace_curation::msg::InstantiateMsg {
                community_pool: community.to_string(),
                min_curation_bond: Some(Uint128::new(1_000_000_000)),
                curation_fee_rate_bps: Some(50),
                challenge_deposit: Some(Uint128::new(100_000_000)),
                slash_percentage_bps: Some(2000),
                activation_delay_seconds: Some(100),
                unbonding_period_seconds: Some(200),
                bond_top_up_window_seconds: Some(100),
                min_quality_score: Some(300),
                max_collections_per_curator: Some(5),
                denom: DENOM.to_string(),
            },
            &[],
            "marketplace-curation",
            None,
        )
        .unwrap();

    let ccv_code_id = app.store_code(credit_class_voting_contract());
    let ccv_addr = app
        .instantiate_contract(
            ccv_code_id,
            admin.clone(),
            &credit_class_voting::msg::InstantiateMsg {
                registry_agent: agent.to_string(),
                deposit_amount: Some(Uint128::new(1_000_000_000)),
                denom: Some(DENOM.to_string()),
                voting_period_seconds: Some(200),
                agent_review_timeout_seconds: Some(100),
                override_window_seconds: Some(50),
                community_pool: None,
            },
            &[],
            "credit-class-voting",
            None,
        )
        .unwrap();

    let se_code_id = app.store_code(service_escrow_contract());
    let se_addr = app
        .instantiate_contract(
            se_code_id,
            admin.clone(),
            &service_escrow::msg::InstantiateMsg {
                arbiter_dao: arbiter.to_string(),
                community_pool: community.to_string(),
                provider_bond_ratio_bps: Some(1000),
                platform_fee_rate_bps: Some(100),
                cancellation_fee_rate_bps: Some(200),
                arbiter_fee_rate_bps: Some(500),
                review_period_seconds: Some(200),
                max_milestones: Some(10),
                max_revisions: Some(3),
                denom: DENOM.to_string(),
            },
            &[],
            "service-escrow",
            None,
        )
        .unwrap();

    let cr_code_id = app.store_code(contribution_rewards_contract());
    let cr_addr = app
        .instantiate_contract(
            cr_code_id,
            admin.clone(),
            &contribution_rewards::msg::InstantiateMsg {
                community_pool_addr: community.to_string(),
                denom: DENOM.to_string(),
            },
            &[],
            "contribution-rewards",
            None,
        )
        .unwrap();

    let vg_code_id = app.store_code(validator_governance_contract());
    let vg_addr = app
        .instantiate_contract(
            vg_code_id,
            admin.clone(),
            &validator_governance::msg::InstantiateMsg {
                min_validators: Some(1),
                max_validators: Some(21),
                term_length_seconds: Some(1000),
                probation_period_seconds: Some(100),
                min_uptime_bps: Some(9950),
                performance_threshold_bps: Some(7000),
                uptime_weight_bps: Some(4000),
                governance_weight_bps: Some(3000),
                ecosystem_weight_bps: Some(3000),
                base_compensation_share_bps: Some(9000),
                performance_bonus_share_bps: Some(1000),
                min_per_category: Some(0),
                denom: DENOM.to_string(),
            },
            &[],
            "validator-governance",
            None,
        )
        .unwrap();

    // ═══ Phase 1: Attestation -> Quality Score -> Curated Collection (M008 + M011) ═══

    // Create attestation
    app.execute_contract(
        attester.clone(),
        ab_addr.clone(),
        &attestation_bonding::msg::ExecuteMsg::CreateAttestation {
            attestation_type: attestation_bonding::state::AttestationType::BaselineMeasurement,
            iri: "regen:attestation/baseline/1".to_string(),
            beneficiary: None,
        },
        &[Coin::new(1_000_000_000u128, DENOM)],
    )
    .unwrap();

    // Activate after delay
    app.update_block(|block| {
        block.time = block.time.plus_seconds(101);
    });

    app.execute_contract(
        admin.clone(),
        ab_addr.clone(),
        &attestation_bonding::msg::ExecuteMsg::ActivateAttestation { attestation_id: 1 },
        &[],
    )
    .unwrap();

    // Submit quality score
    let batch_denom = "regen:batch/C02-001-20250301-20260301-001";
    app.execute_contract(
        admin.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::SubmitQualityScore {
            batch_denom: batch_denom.to_string(),
            score: 850,
            confidence: 950,
        },
        &[],
    )
    .unwrap();

    // Create and activate collection
    app.execute_contract(
        curator.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::CreateCollection {
            name: "Premium Verified Credits".to_string(),
            criteria: "Score >= 800, attestation bonded".to_string(),
        },
        &[Coin::new(1_000_000_000u128, DENOM)],
    )
    .unwrap();

    app.update_block(|block| {
        block.time = block.time.plus_seconds(101);
    });

    app.execute_contract(
        curator.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::ActivateCollection { collection_id: 1 },
        &[],
    )
    .unwrap();

    // Add batch to collection
    app.execute_contract(
        curator.clone(),
        mc_addr.clone(),
        &marketplace_curation::msg::ExecuteMsg::AddToCollection {
            collection_id: 1,
            batch_denom: batch_denom.to_string(),
        },
        &[],
    )
    .unwrap();

    let coll_resp: marketplace_curation::msg::CollectionResponse = app
        .wrap()
        .query_wasm_smart(
            mc_addr.clone(),
            &marketplace_curation::msg::QueryMsg::Collection { collection_id: 1 },
        )
        .unwrap();
    assert_eq!(
        coll_resp.collection.status,
        marketplace_curation::state::CollectionStatus::Active
    );
    assert!(coll_resp.collection.batches.contains(&batch_denom.to_string()));

    // ═══ Phase 2: Credit class proposal -> agent screen -> vote -> approve (M001-ENH) ═══

    app.execute_contract(
        proposer.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::SubmitProposal {
            admin_address: admin.to_string(),
            credit_type: "BIO".to_string(),
            methodology_iri: "regen:methodology/biodiversity-v2".to_string(),
        },
        &[Coin::new(1_000_000_000u128, DENOM)],
    )
    .unwrap();

    // Agent approves
    app.execute_contract(
        agent.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::SubmitAgentScore {
            proposal_id: 1,
            score: 850,
            confidence: 920,
            recommendation: credit_class_voting::state::AgentRecommendation::Approve,
        },
        &[],
    )
    .unwrap();

    // Votes
    app.execute_contract(
        voter1.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::CastVote {
            proposal_id: 1,
            vote_yes: true,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        voter2.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::CastVote {
            proposal_id: 1,
            vote_yes: true,
        },
        &[],
    )
    .unwrap();

    // Advance past voting period and finalize
    app.update_block(|block| {
        block.time = block.time.plus_seconds(201);
    });

    app.execute_contract(
        admin.clone(),
        ccv_addr.clone(),
        &credit_class_voting::msg::ExecuteMsg::FinalizeProposal { proposal_id: 1 },
        &[],
    )
    .unwrap();

    let prop_resp: credit_class_voting::msg::ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            ccv_addr.clone(),
            &credit_class_voting::msg::QueryMsg::Proposal { proposal_id: 1 },
        )
        .unwrap();
    assert_eq!(
        prop_resp.proposal.status,
        credit_class_voting::state::ProposalStatus::Approved
    );

    // ═══ Phase 3: Commission service via escrow (M009) ═══

    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::ProposeAgreement {
            provider: provider.to_string(),
            service_type: "Biodiversity monitoring".to_string(),
            description: "Quarterly species survey".to_string(),
            milestones: vec![service_escrow::msg::MilestoneInput {
                description: "Complete survey and deliver report".to_string(),
                payment_amount: Uint128::new(500_000_000),
            }],
        },
        &[],
    )
    .unwrap();

    // Provider accepts with bond
    let bond = Uint128::new(50_000_000); // 10% of 500M
    app.execute_contract(
        provider.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::AcceptAgreement { agreement_id: 1 },
        &[Coin::new(bond.u128(), DENOM)],
    )
    .unwrap();

    // Client funds (extra to cover completion fee of 1% of escrow)
    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::FundAgreement { agreement_id: 1 },
        &[Coin::new(505_000_000u128, DENOM)],
    )
    .unwrap();

    // Start
    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::StartAgreement { agreement_id: 1 },
        &[],
    )
    .unwrap();

    // Submit and approve milestone
    app.execute_contract(
        provider.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::SubmitMilestone {
            agreement_id: 1,
            milestone_index: 0,
            deliverable_iri: "regen:deliverable/survey-q1".to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        client.clone(),
        se_addr.clone(),
        &service_escrow::msg::ExecuteMsg::ApproveMilestone {
            agreement_id: 1,
            milestone_index: 0,
        },
        &[],
    )
    .unwrap();

    let agree_resp: service_escrow::msg::AgreementResponse = app
        .wrap()
        .query_wasm_smart(
            se_addr.clone(),
            &service_escrow::msg::QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap();
    assert_eq!(
        agree_resp.agreement.status,
        service_escrow::state::AgreementStatus::Completed
    );

    // ═══ Phase 4: Record ecosystem activity -> distribute rewards (M015) ═══

    // Initialize and activate contribution rewards
    app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::InitializeMechanism {},
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::ActivateDistribution {},
        &[],
    )
    .unwrap();

    // Record activity for participants reflecting the ecosystem work done above
    app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::RecordActivity {
            participant: participant.to_string(),
            credit_purchase_value: Uint128::new(1_000_000_000),
            credit_retirement_value: Uint128::new(500_000_000),
            platform_facilitation_value: Uint128::new(200_000_000),
            governance_votes: 3,
            proposal_credits: 100,
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::RecordActivity {
            participant: curator.to_string(),
            credit_purchase_value: Uint128::new(200_000_000),
            credit_retirement_value: Uint128::new(100_000_000),
            platform_facilitation_value: Uint128::new(800_000_000),
            governance_votes: 2,
            proposal_credits: 50,
        },
        &[],
    )
    .unwrap();

    // Read current period before triggering distribution
    let mech_resp: contribution_rewards::msg::MechanismStateResponse = app
        .wrap()
        .query_wasm_smart(
            cr_addr.clone(),
            &contribution_rewards::msg::QueryMsg::MechanismState {},
        )
        .unwrap();
    let dist_period = mech_resp.current_period;

    // Trigger distribution
    app.execute_contract(
        admin.clone(),
        cr_addr.clone(),
        &contribution_rewards::msg::ExecuteMsg::TriggerDistribution {
            community_pool_inflow: Uint128::new(2_000_000_000),
        },
        &[Coin::new(2_000_000_000u128, DENOM)],
    )
    .unwrap();

    let dist_resp: contribution_rewards::msg::DistributionRecordResponse = app
        .wrap()
        .query_wasm_smart(
            cr_addr.clone(),
            &contribution_rewards::msg::QueryMsg::DistributionRecord { period: dist_period },
        )
        .unwrap();
    assert!(!dist_resp.record.activity_pool.is_zero());
    assert_eq!(
        dist_resp.record.community_pool_inflow,
        Uint128::new(2_000_000_000)
    );

    // ═══ Phase 5: Verify validator governance operational (M014) ═══

    // Apply, approve, activate validators
    for (val, cat, data) in [
        (
            &val1,
            validator_governance::state::ValidatorCategory::InfrastructureBuilders,
            "Core dev",
        ),
        (
            &val2,
            validator_governance::state::ValidatorCategory::TrustedRefiPartners,
            "ReFi integrator",
        ),
        (
            &val3,
            validator_governance::state::ValidatorCategory::EcologicalDataStewards,
            "Data scientist",
        ),
    ] {
        app.execute_contract(
            val.clone(),
            vg_addr.clone(),
            &validator_governance::msg::ExecuteMsg::ApplyForValidator {
                category: cat,
                application_data: data.to_string(),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            admin.clone(),
            vg_addr.clone(),
            &validator_governance::msg::ExecuteMsg::ApproveValidator {
                applicant: val.to_string(),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            admin.clone(),
            vg_addr.clone(),
            &validator_governance::msg::ExecuteMsg::ActivateValidator {
                validator: val.to_string(),
            },
            &[],
        )
        .unwrap();
    }

    // Verify 3 active
    let state_resp: validator_governance::msg::ModuleStateResponse = app
        .wrap()
        .query_wasm_smart(
            vg_addr.clone(),
            &validator_governance::msg::QueryMsg::ModuleState {},
        )
        .unwrap();
    assert_eq!(state_resp.state.total_active, 3);

    // Submit performance and distribute compensation
    for val in [&val1, &val2, &val3] {
        app.execute_contract(
            admin.clone(),
            vg_addr.clone(),
            &validator_governance::msg::ExecuteMsg::SubmitPerformanceReport {
                validator: val.to_string(),
                uptime_bps: 9990,
                governance_participation_bps: 8500,
                ecosystem_contribution_bps: 8000,
            },
            &[],
        )
        .unwrap();
    }

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::UpdateValidatorFund {},
        &[Coin::new(3_000_000_000u128, DENOM)],
    )
    .unwrap();

    app.execute_contract(
        admin.clone(),
        vg_addr.clone(),
        &validator_governance::msg::ExecuteMsg::DistributeCompensation {},
        &[],
    )
    .unwrap();

    // Verify all validators have compensation
    for val in [&val1, &val2, &val3] {
        let val_resp: validator_governance::msg::ValidatorResponse = app
            .wrap()
            .query_wasm_smart(
                vg_addr.clone(),
                &validator_governance::msg::QueryMsg::Validator {
                    address: val.to_string(),
                },
            )
            .unwrap();
        assert!(
            !val_resp.validator.compensation_due.is_zero(),
            "Validator {} should have compensation",
            val
        );
        assert_eq!(
            val_resp.validator.status,
            validator_governance::state::ValidatorStatus::Active
        );
    }

    // ═══ Final verification: all contracts deployed and operational ═══

    // Attestation bonding: attestation active
    let att_resp: attestation_bonding::msg::AttestationResponse = app
        .wrap()
        .query_wasm_smart(
            ab_addr.clone(),
            &attestation_bonding::msg::QueryMsg::Attestation { attestation_id: 1 },
        )
        .unwrap();
    assert_eq!(
        att_resp.attestation.status,
        attestation_bonding::state::AttestationStatus::Active
    );

    // Marketplace curation: collection active with batch
    let coll_resp: marketplace_curation::msg::CollectionResponse = app
        .wrap()
        .query_wasm_smart(
            mc_addr.clone(),
            &marketplace_curation::msg::QueryMsg::Collection { collection_id: 1 },
        )
        .unwrap();
    assert_eq!(
        coll_resp.collection.status,
        marketplace_curation::state::CollectionStatus::Active
    );

    // Credit class voting: proposal approved
    let prop_resp: credit_class_voting::msg::ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            ccv_addr.clone(),
            &credit_class_voting::msg::QueryMsg::Proposal { proposal_id: 1 },
        )
        .unwrap();
    assert_eq!(
        prop_resp.proposal.status,
        credit_class_voting::state::ProposalStatus::Approved
    );

    // Service escrow: agreement completed
    let agree_resp: service_escrow::msg::AgreementResponse = app
        .wrap()
        .query_wasm_smart(
            se_addr.clone(),
            &service_escrow::msg::QueryMsg::Agreement { agreement_id: 1 },
        )
        .unwrap();
    assert_eq!(
        agree_resp.agreement.status,
        service_escrow::state::AgreementStatus::Completed
    );

    // Contribution rewards: distribution executed
    let mech_resp: contribution_rewards::msg::MechanismStateResponse = app
        .wrap()
        .query_wasm_smart(
            cr_addr.clone(),
            &contribution_rewards::msg::QueryMsg::MechanismState {},
        )
        .unwrap();
    assert_eq!(mech_resp.status, "Distributing");

    // Validator governance: 3 active, fund distributed
    let state_resp: validator_governance::msg::ModuleStateResponse = app
        .wrap()
        .query_wasm_smart(
            vg_addr.clone(),
            &validator_governance::msg::QueryMsg::ModuleState {},
        )
        .unwrap();
    assert_eq!(state_resp.state.total_active, 3);
    assert!(state_resp.state.last_compensation_distribution.is_some());
}
