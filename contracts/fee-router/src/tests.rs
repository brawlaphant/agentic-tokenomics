use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{from_json, Addr, Decimal, Uint128};

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{
    CalculateFeeResponse, ExecuteMsg, FeeConfigResponse, InstantiateMsg, PoolBalancesResponse,
    QueryMsg, TxType,
};

/// Helper to build the default v0 Model A InstantiateMsg.
fn default_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        issuance_rate: Decimal::percent(2),      // 0.02
        transfer_rate: Decimal::permille(1),      // 0.001
        retirement_rate: Decimal::permille(5),    // 0.005
        trade_rate: Decimal::percent(1),          // 0.01
        burn_share: Decimal::percent(30),         // 0.30
        validator_share: Decimal::percent(40),    // 0.40
        community_share: Decimal::percent(25),    // 0.25
        agent_share: Decimal::percent(5),         // 0.05
        min_fee: Uint128::new(1_000_000),         // 1 REGEN = 1,000,000 uregen
    }
}

// =========================================================================
// Test 1: Instantiation with valid params
// =========================================================================
#[test]
fn test_instantiation_valid() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let msg = default_instantiate_msg();
    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(res.attributes.len(), 2);
    assert_eq!(res.attributes[0].value, "instantiate");
    assert_eq!(res.attributes[1].value, "admin");

    // Verify config was stored correctly
    let config_resp: FeeConfigResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::FeeConfig {}).unwrap()).unwrap();
    assert_eq!(config_resp.admin, "admin");
    assert_eq!(config_resp.issuance_rate, Decimal::percent(2));
    assert_eq!(config_resp.transfer_rate, Decimal::permille(1));
    assert_eq!(config_resp.retirement_rate, Decimal::permille(5));
    assert_eq!(config_resp.trade_rate, Decimal::percent(1));
    assert_eq!(config_resp.min_fee, Uint128::new(1_000_000));

    // Verify pools initialized to zero
    let pools_resp: PoolBalancesResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::PoolBalances {}).unwrap()).unwrap();
    assert_eq!(pools_resp.burn_pool, Uint128::zero());
    assert_eq!(pools_resp.validator_fund, Uint128::zero());
    assert_eq!(pools_resp.community_pool, Uint128::zero());
    assert_eq!(pools_resp.agent_infra, Uint128::zero());
}

// =========================================================================
// Test 2: Instantiation fails when shares don't sum to 1.0
// =========================================================================
#[test]
fn test_instantiation_fails_bad_shares() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.agent_share = Decimal::percent(10); // sum = 0.30 + 0.40 + 0.25 + 0.10 = 1.05

    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    match err {
        ContractError::ShareSumNotUnity { .. } => {}
        e => panic!("Expected ShareSumNotUnity, got: {:?}", e),
    }
}

// =========================================================================
// Test 3: Fee calculation matches M013 test vectors
// =========================================================================

/// Test vector 1: Credit issuance of 5,000,000,000 uregen at 2% = 100,000,000 uregen fee
#[test]
fn test_fee_calc_credit_issuance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info, default_instantiate_msg()).unwrap();

    let resp: CalculateFeeResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::CalculateFee {
                tx_type: TxType::CreditIssuance,
                value: Uint128::new(5_000_000_000),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.fee_amount, Uint128::new(100_000_000));
    assert!(!resp.min_fee_applied);
}

/// Test vector 2: Credit transfer of 100,000,000 uregen at 0.1% -> 100,000 < min_fee -> clamped to 1,000,000
#[test]
fn test_fee_calc_credit_transfer_min_fee() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info, default_instantiate_msg()).unwrap();

    let resp: CalculateFeeResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::CalculateFee {
                tx_type: TxType::CreditTransfer,
                value: Uint128::new(100_000_000),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.fee_amount, Uint128::new(1_000_000));
    assert!(resp.min_fee_applied);
}

/// Test vector 3: Credit retirement of 1,000,000,000 uregen at 0.5% = 5,000,000 uregen fee
#[test]
fn test_fee_calc_credit_retirement() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info, default_instantiate_msg()).unwrap();

    let resp: CalculateFeeResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::CalculateFee {
                tx_type: TxType::CreditRetirement,
                value: Uint128::new(1_000_000_000),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.fee_amount, Uint128::new(5_000_000));
    assert!(!resp.min_fee_applied);
}

/// Test vector 4: Marketplace trade of 2,500,000,000 uregen at 1% = 25,000,000 uregen fee
#[test]
fn test_fee_calc_marketplace_trade() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info, default_instantiate_msg()).unwrap();

    let resp: CalculateFeeResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::CalculateFee {
                tx_type: TxType::MarketplaceTrade,
                value: Uint128::new(2_500_000_000),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.fee_amount, Uint128::new(25_000_000));
    assert!(!resp.min_fee_applied);
}

// =========================================================================
// Test 4: Fee distribution — 100M fee -> burn 30M, validator 40M, community 25M, agent 5M
// =========================================================================
#[test]
fn test_fee_distribution() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), default_instantiate_msg()).unwrap();

    // Collect fee: issuance of 5B uregen -> fee = 100M uregen
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::CollectFee {
            tx_type: TxType::CreditIssuance,
            value: Uint128::new(5_000_000_000),
        },
    )
    .unwrap();

    let pools_resp: PoolBalancesResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::PoolBalances {}).unwrap()).unwrap();

    assert_eq!(pools_resp.burn_pool, Uint128::new(30_000_000));
    assert_eq!(pools_resp.validator_fund, Uint128::new(40_000_000));
    assert_eq!(pools_resp.community_pool, Uint128::new(25_000_000));
    assert_eq!(pools_resp.agent_infra, Uint128::new(5_000_000));
}

// =========================================================================
// Test 5: Fee conservation — sum of pools == fee collected
// =========================================================================
#[test]
fn test_fee_conservation() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), default_instantiate_msg()).unwrap();

    // Collect fees for all four tx types from test vectors
    let test_cases = vec![
        (TxType::CreditIssuance, 5_000_000_000u128, 100_000_000u128),
        (TxType::CreditTransfer, 100_000_000, 1_000_000),     // min_fee applied
        (TxType::CreditRetirement, 1_000_000_000, 5_000_000),
        (TxType::MarketplaceTrade, 2_500_000_000, 25_000_000),
        (TxType::CreditTransfer, 500_000, 1_000_000),         // min_fee applied
    ];

    let mut total_expected_fees = Uint128::zero();
    for (tx_type, value, expected_fee) in test_cases {
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::CollectFee {
                tx_type,
                value: Uint128::new(value),
            },
        )
        .unwrap();
        total_expected_fees += Uint128::new(expected_fee);
    }

    let pools_resp: PoolBalancesResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::PoolBalances {}).unwrap()).unwrap();

    let pool_sum = pools_resp.burn_pool
        + pools_resp.validator_fund
        + pools_resp.community_pool
        + pools_resp.agent_infra;

    // Fee Conservation invariant: sum of pools == total fees collected
    assert_eq!(pool_sum, total_expected_fees);

    // Additionally verify the expected totals from the test vector KPIs:
    // total_fees = 132,000,000
    assert_eq!(total_expected_fees, Uint128::new(132_000_000));
    // distribution_by_pool: burn 39,600,000 / validator 52,800,000 / community 33,000,000 / agent 6,600,000
    assert_eq!(pools_resp.burn_pool, Uint128::new(39_600_000));
    assert_eq!(pools_resp.validator_fund, Uint128::new(52_800_000));
    assert_eq!(pools_resp.community_pool, Uint128::new(33_000_000));
    assert_eq!(pools_resp.agent_infra, Uint128::new(6_600_000));
}

// =========================================================================
// Test 6: Rate update rejected above 10%
// =========================================================================
#[test]
fn test_rate_update_rejected_above_cap() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), default_instantiate_msg()).unwrap();

    // Try to set issuance rate to 11% (exceeds 10% cap)
    let err = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::UpdateFeeRate {
            tx_type: TxType::CreditIssuance,
            rate: Decimal::percent(11),
        },
    )
    .unwrap_err();

    match err {
        ContractError::RateExceedsCap { .. } => {}
        e => panic!("Expected RateExceedsCap, got: {:?}", e),
    }

    // Exactly 10% should succeed
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::UpdateFeeRate {
            tx_type: TxType::CreditIssuance,
            rate: Decimal::percent(10),
        },
    )
    .unwrap();
}

// =========================================================================
// Test 7: Share update rejected when sum != 1.0
// =========================================================================
#[test]
fn test_share_update_rejected_bad_sum() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), default_instantiate_msg()).unwrap();

    // Shares sum to 0.95 (not 1.0)
    let err = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::UpdateDistribution {
            burn_share: Decimal::percent(25),
            validator_share: Decimal::percent(40),
            community_share: Decimal::percent(25),
            agent_share: Decimal::percent(5),
        },
    )
    .unwrap_err();

    match err {
        ContractError::ShareSumNotUnity { .. } => {}
        e => panic!("Expected ShareSumNotUnity, got: {:?}", e),
    }

    // Valid shares (sum = 1.0) should succeed
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::UpdateDistribution {
            burn_share: Decimal::percent(20),
            validator_share: Decimal::percent(45),
            community_share: Decimal::percent(30),
            agent_share: Decimal::percent(5),
        },
    )
    .unwrap();
}

// =========================================================================
// Additional: Unauthorized tests
// =========================================================================
#[test]
fn test_unauthorized_update_fee_rate() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin_info = message_info(&Addr::unchecked("admin"), &[]);
    let other_info = message_info(&Addr::unchecked("other"), &[]);
    instantiate(
        deps.as_mut(),
        env.clone(),
        admin_info,
        default_instantiate_msg(),
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        env,
        other_info,
        ExecuteMsg::UpdateFeeRate {
            tx_type: TxType::CreditIssuance,
            rate: Decimal::percent(3),
        },
    )
    .unwrap_err();

    match err {
        ContractError::Unauthorized {} => {}
        e => panic!("Expected Unauthorized, got: {:?}", e),
    }
}

#[test]
fn test_unauthorized_update_distribution() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin_info = message_info(&Addr::unchecked("admin"), &[]);
    let other_info = message_info(&Addr::unchecked("other"), &[]);
    instantiate(
        deps.as_mut(),
        env.clone(),
        admin_info,
        default_instantiate_msg(),
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        env,
        other_info,
        ExecuteMsg::UpdateDistribution {
            burn_share: Decimal::percent(30),
            validator_share: Decimal::percent(40),
            community_share: Decimal::percent(25),
            agent_share: Decimal::percent(5),
        },
    )
    .unwrap_err();

    match err {
        ContractError::Unauthorized {} => {}
        e => panic!("Expected Unauthorized, got: {:?}", e),
    }
}

#[test]
fn test_collect_fee_zero_value_rejected() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info.clone(), default_instantiate_msg()).unwrap();

    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::CollectFee {
            tx_type: TxType::CreditIssuance,
            value: Uint128::zero(),
        },
    )
    .unwrap_err();

    match err {
        ContractError::ZeroValue {} => {}
        e => panic!("Expected ZeroValue, got: {:?}", e),
    }
}
