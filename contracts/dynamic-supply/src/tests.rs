use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{from_json, Addr, Decimal, Uint128};

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, MintBurnRecordResponse, QueryMsg, SimulatePeriodResponse,
    SupplyParamsResponse, SupplyStateResponse,
};
use crate::state::{M014Phase, SupplyPhase};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Default instantiation: 221M REGEN cap, 200M initial supply, 2% regrowth,
/// ecological multiplier disabled (v0), M014 Inactive.
fn default_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        hard_cap: Uint128::new(221_000_000_000_000),       // 221M REGEN in uregen
        initial_supply: Uint128::new(200_000_000_000_000), // 200M REGEN in uregen
        base_regrowth_rate: Decimal::percent(2),           // 0.02
        ecological_multiplier_enabled: false,
        ecological_reference_value: Decimal::from_atomics(50u128, 0).unwrap(), // 50 ppm
        m014_phase: M014Phase::Inactive,
        equilibrium_threshold: Uint128::new(1_000_000_000), // 1000 REGEN tolerance
        equilibrium_periods_required: 12,
    }
}

fn setup_contract() -> (cosmwasm_std::OwnedDeps<cosmwasm_std::MemoryStorage, cosmwasm_std::testing::MockApi, cosmwasm_std::testing::MockQuerier>, cosmwasm_std::Env) {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);
    instantiate(deps.as_mut(), env.clone(), info, default_instantiate_msg()).unwrap();
    (deps, env)
}

// =========================================================================
// SPEC Acceptance Test 1: Basic mint/burn
// Given a supply state with known staking ratio, compute M[t] and B[t];
// verify S[t+1] = S[t] + M[t] - B[t].
// =========================================================================
#[test]
fn test_basic_mint_burn() {
    let (mut deps, env) = setup_contract();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // 200M supply, 221M cap, headroom = 21M REGEN = 21_000_000_000_000 uregen
    // staked = 100M (50% staking ratio) -> staking_multiplier = 1.5
    // M014 Inactive -> effective_multiplier = staking_multiplier = 1.5
    // ecological disabled -> eco_mult = 1.0
    // r = 0.02 * 1.5 * 1.0 = 0.03
    // M[t] = 0.03 * 21_000_000_000_000 = 630_000_000_000
    // burn = 500_000_000_000 (500M uregen = 500 REGEN)
    let burn = Uint128::new(500_000_000_000);
    let staked = Uint128::new(100_000_000_000_000); // 100M REGEN

    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ExecutePeriod {
            burn_amount: burn,
            staked_amount: staked,
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::SupplyState {}).unwrap()).unwrap();

    // S[t+1] = 200_000_000_000_000 + 630_000_000_000 - 500_000_000_000
    //        = 200_130_000_000_000
    assert_eq!(state.current_supply, Uint128::new(200_130_000_000_000));
    assert_eq!(state.total_minted, Uint128::new(630_000_000_000));
    assert_eq!(state.total_burned, Uint128::new(500_000_000_000));
    assert_eq!(state.period_count, 1);

    // Verify history record
    let record: MintBurnRecordResponse = from_json(
        query(deps.as_ref(), env, QueryMsg::PeriodHistory { period_id: 1 }).unwrap(),
    )
    .unwrap();
    assert_eq!(record.record.minted, Uint128::new(630_000_000_000));
    assert_eq!(record.record.burned, Uint128::new(500_000_000_000));
    assert_eq!(
        record.record.supply_before,
        Uint128::new(200_000_000_000_000)
    );
    assert_eq!(
        record.record.supply_after,
        Uint128::new(200_130_000_000_000)
    );
}

// =========================================================================
// SPEC Acceptance Test 2: Cap enforcement
// If S[t] + M[t] - B[t] > C, then S[t+1] = C (cap inviolability).
// =========================================================================
#[test]
fn test_cap_enforcement() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // Set supply very close to cap: cap = 221M, supply = 220.9M
    let msg = InstantiateMsg {
        hard_cap: Uint128::new(221_000_000_000_000),
        initial_supply: Uint128::new(220_900_000_000_000),
        base_regrowth_rate: Decimal::percent(10), // 10% to force near-cap
        ecological_multiplier_enabled: false,
        ecological_reference_value: Decimal::from_atomics(50u128, 0).unwrap(),
        m014_phase: M014Phase::Inactive,
        equilibrium_threshold: Uint128::new(1_000_000_000),
        equilibrium_periods_required: 12,
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // headroom = 100_000_000_000 (0.1M REGEN)
    // staked = 220.9M (100%) -> multiplier = 2.0
    // r = 0.10 * 2.0 = 0.20
    // M[t] = 0.20 * 100_000_000_000 = 20_000_000_000
    // burn = 0
    // S[t+1] = 220.9M + 20_000_000_000 = 220_920_000_000_000 <= 221M cap -> OK no clamping
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::zero(),
            staked_amount: Uint128::new(220_900_000_000_000),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyState {}).unwrap()).unwrap();

    // Supply must not exceed hard cap
    assert!(state.current_supply <= Uint128::new(221_000_000_000_000));
}

// =========================================================================
// SPEC Acceptance Test 3: Non-negative supply
// If B[t] > S[t] + M[t], then S[t+1] = 0.
// =========================================================================
#[test]
fn test_non_negative_supply() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // Small supply, massive burn
    let msg = InstantiateMsg {
        hard_cap: Uint128::new(221_000_000_000_000),
        initial_supply: Uint128::new(1_000_000), // 1 REGEN
        base_regrowth_rate: Decimal::percent(2),
        ecological_multiplier_enabled: false,
        ecological_reference_value: Decimal::from_atomics(50u128, 0).unwrap(),
        m014_phase: M014Phase::Inactive,
        equilibrium_threshold: Uint128::new(1_000_000_000),
        equilibrium_periods_required: 12,
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Burn way more than supply + mint
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::new(999_999_999_999_999),
            staked_amount: Uint128::zero(),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyState {}).unwrap()).unwrap();

    assert_eq!(state.current_supply, Uint128::zero());
}

// =========================================================================
// SPEC Acceptance Test 4: Staking multiplier range
// 0% staked -> 1.0, 100% staked -> 2.0, 50% staked -> 1.5
// =========================================================================
#[test]
fn test_staking_multiplier_range() {
    let (deps, env) = setup_contract();

    // 0% staked -> multiplier = 1.0, r = 0.02 * 1.0 = 0.02
    // headroom = 21_000_000_000_000
    // M = 0.02 * 21_000_000_000_000 = 420_000_000_000
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::zero(),
                stability_committed: Uint128::zero(),
                delta_co2: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(resp.effective_multiplier, Decimal::one());
    assert_eq!(resp.mint_amount, Uint128::new(420_000_000_000));

    // 50% staked -> multiplier = 1.5, r = 0.02 * 1.5 = 0.03
    // M = 0.03 * 21_000_000_000_000 = 630_000_000_000
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::new(100_000_000_000_000), // 50% of 200M
                stability_committed: Uint128::zero(),
                delta_co2: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(resp.effective_multiplier, Decimal::percent(150));
    assert_eq!(resp.mint_amount, Uint128::new(630_000_000_000));

    // 100% staked -> multiplier = 2.0, r = 0.02 * 2.0 = 0.04
    // M = 0.04 * 21_000_000_000_000 = 840_000_000_000
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::new(200_000_000_000_000), // 100% of 200M
                stability_committed: Uint128::zero(),
                delta_co2: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(resp.effective_multiplier, Decimal::percent(200));
    assert_eq!(resp.mint_amount, Uint128::new(840_000_000_000));
}

// =========================================================================
// SPEC Acceptance Test 5: Near-cap deceleration
// When supply is at 99% of cap, minted amount is very small.
// =========================================================================
#[test]
fn test_near_cap_deceleration() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // Supply at 99% of cap
    let cap = Uint128::new(221_000_000_000_000);
    let supply_99pct = Uint128::new(218_790_000_000_000); // 99% of 221M

    let msg = InstantiateMsg {
        hard_cap: cap,
        initial_supply: supply_99pct,
        base_regrowth_rate: Decimal::percent(2),
        ecological_multiplier_enabled: false,
        ecological_reference_value: Decimal::from_atomics(50u128, 0).unwrap(),
        m014_phase: M014Phase::Inactive,
        equilibrium_threshold: Uint128::new(1_000_000_000),
        equilibrium_periods_required: 12,
    };
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // headroom = 1% of 221M = 2_210_000_000_000
    // r = 0.02 * 1.0 = 0.02 (no staking)
    // M = 0.02 * 2_210_000_000_000 = 44_200_000_000
    // This is much smaller than the mint at 200M supply (420B)
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::zero(),
                stability_committed: Uint128::zero(),
                delta_co2: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.mint_amount, Uint128::new(44_200_000_000));
    // This is about 10.5% of what would be minted at 200M supply (420B)
    assert!(resp.mint_amount < Uint128::new(420_000_000_000));
}

// =========================================================================
// SPEC Acceptance Test 6: INACTIVE phase — only staking multiplier
// =========================================================================
#[test]
fn test_phase_inactive_uses_staking_only() {
    let (deps, env) = setup_contract();

    // M014 Inactive: even with stability_committed, only staking matters
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::new(100_000_000_000_000), // 50% staked
                stability_committed: Uint128::new(180_000_000_000_000), // 90% stability
                delta_co2: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    // Should use staking_multiplier = 1.5, NOT stability = 1.9
    assert_eq!(resp.effective_multiplier, Decimal::percent(150));
}

// =========================================================================
// SPEC Acceptance Test 7: TRANSITION phase — max(staking, stability)
// =========================================================================
#[test]
fn test_phase_transition_uses_max() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.m014_phase = M014Phase::Transition;
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // staking = 50% -> multiplier = 1.5
    // stability = 80% -> multiplier = 1.8
    // TRANSITION: max(1.5, 1.8) = 1.8
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::new(100_000_000_000_000),  // 50%
                stability_committed: Uint128::new(160_000_000_000_000), // 80%
                delta_co2: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.effective_multiplier, Decimal::percent(180));

    // Now staking > stability: staking = 90%, stability = 30%
    // staking_mult = 1.9, stability_mult = 1.3
    // TRANSITION: max(1.9, 1.3) = 1.9
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::new(180_000_000_000_000),  // 90%
                stability_committed: Uint128::new(60_000_000_000_000), // 30%
                delta_co2: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.effective_multiplier, Decimal::percent(190));
}

// =========================================================================
// SPEC Acceptance Test 8: ACTIVE phase — only stability multiplier
// =========================================================================
#[test]
fn test_phase_active_uses_stability_only() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.m014_phase = M014Phase::Active;
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // staking = 90% -> multiplier = 1.9 (should be ignored)
    // stability = 40% -> multiplier = 1.4 (should be used)
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::new(180_000_000_000_000),  // 90% staked
                stability_committed: Uint128::new(80_000_000_000_000), // 40% stability
                delta_co2: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    // Should use stability_multiplier = 1.4, NOT staking = 1.9
    assert_eq!(resp.effective_multiplier, Decimal::percent(140));
}

// =========================================================================
// SPEC Acceptance Test 9: Ecological multiplier disabled (v0)
// =========================================================================
#[test]
fn test_ecological_multiplier_disabled() {
    let (deps, env) = setup_contract();

    // Even with delta_co2 provided, eco_mult should be 1.0 when disabled
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::zero(),
                stability_committed: Uint128::zero(),
                delta_co2: Some(Decimal::from_atomics(25u128, 0).unwrap()),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.ecological_multiplier, Decimal::one());
}

// =========================================================================
// SPEC Acceptance Test 10: Ecological multiplier enabled
// delta_co2 = 25 ppm, reference = 50 ppm -> eco_mult = 0.5
// =========================================================================
#[test]
fn test_ecological_multiplier_enabled() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.ecological_multiplier_enabled = true;
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // delta_co2 = 25, reference = 50 -> eco_mult = max(0, 1 - 25/50) = 0.5
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::zero(),
                stability_committed: Uint128::zero(),
                delta_co2: Some(Decimal::from_atomics(25u128, 0).unwrap()),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.ecological_multiplier, Decimal::percent(50));

    // Verify mint is halved: normal mint with eco=1.0 would be 420B
    // With eco=0.5: r = 0.02 * 1.0 * 0.5 = 0.01, M = 0.01 * 21T = 210B
    assert_eq!(resp.mint_amount, Uint128::new(210_000_000_000));
}

// =========================================================================
// SPEC Acceptance Test 11: Ecological multiplier floor at 0
// delta_co2 = 100 ppm, reference = 50 ppm -> eco_mult = 0 (not negative)
// =========================================================================
#[test]
fn test_ecological_multiplier_floor() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.ecological_multiplier_enabled = true;
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // delta_co2 = 100, reference = 50 -> eco_mult = max(0, 1 - 100/50) = max(0, -1) = 0
    let resp: SimulatePeriodResponse = from_json(
        query(
            deps.as_ref(),
            env,
            QueryMsg::SimulatePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::zero(),
                stability_committed: Uint128::zero(),
                delta_co2: Some(Decimal::from_atomics(100u128, 0).unwrap()),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resp.ecological_multiplier, Decimal::zero());
    assert_eq!(resp.mint_amount, Uint128::zero());
}

// =========================================================================
// SPEC Acceptance Test 12-13: State machine TRANSITION -> DYNAMIC
// Requires first burn period complete.
// =========================================================================
#[test]
fn test_phase_transition_to_dynamic() {
    let (mut deps, env) = setup_contract();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // Initial phase should be Transition
    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::SupplyState {}).unwrap()).unwrap();
    assert_eq!(state.phase, SupplyPhase::Transition);

    // Execute period with zero burn -> stays in Transition
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::zero(),
            staked_amount: Uint128::zero(),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::SupplyState {}).unwrap()).unwrap();
    assert_eq!(state.phase, SupplyPhase::Transition);

    // Execute period with non-zero burn -> transitions to Dynamic
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::new(1_000_000),
            staked_amount: Uint128::zero(),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyState {}).unwrap()).unwrap();
    assert_eq!(state.phase, SupplyPhase::Dynamic);
}

// =========================================================================
// SPEC Acceptance Test 14: DYNAMIC -> EQUILIBRIUM
// 12 consecutive near-balance periods.
// =========================================================================
#[test]
fn test_phase_dynamic_to_equilibrium() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.equilibrium_threshold = Uint128::new(1_000_000_000); // 1000 REGEN
    msg.equilibrium_periods_required = 3; // Use 3 for test speed
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // First: transition to Dynamic with a burn
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::new(420_000_000_000), // ~ equal to mint
            staked_amount: Uint128::zero(),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::SupplyState {}).unwrap()).unwrap();
    assert_eq!(state.phase, SupplyPhase::Dynamic);

    // Now run 3 periods with near-equal mint and burn
    // Current supply ~ 200M, headroom ~21M, r=0.02, M~420B
    // Set burn close to expected mint for near-equilibrium
    for _ in 0..3 {
        // Query current state to compute expected mint
        let sim: SimulatePeriodResponse = from_json(
            query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::SimulatePeriod {
                    burn_amount: Uint128::zero(),
                    staked_amount: Uint128::zero(),
                    stability_committed: Uint128::zero(),
                    delta_co2: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // Burn exactly equal to expected mint -> diff = 0 < threshold
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::ExecutePeriod {
                burn_amount: sim.mint_amount,
                staked_amount: Uint128::zero(),
                stability_committed: Uint128::zero(),
                delta_co2: None,
            },
        )
        .unwrap();
    }

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyState {}).unwrap()).unwrap();
    assert_eq!(state.phase, SupplyPhase::Equilibrium);
}

// =========================================================================
// SPEC Acceptance Test 15: EQUILIBRIUM -> DYNAMIC on shock
// =========================================================================
#[test]
fn test_phase_equilibrium_to_dynamic_on_shock() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.equilibrium_threshold = Uint128::new(1_000_000_000);
    msg.equilibrium_periods_required = 2; // Quick equilibrium for test
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Transition to Dynamic
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::new(420_000_000_000),
            staked_amount: Uint128::zero(),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    // Reach equilibrium with 2 balanced periods
    for _ in 0..2 {
        let sim: SimulatePeriodResponse = from_json(
            query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::SimulatePeriod {
                    burn_amount: Uint128::zero(),
                    staked_amount: Uint128::zero(),
                    stability_committed: Uint128::zero(),
                    delta_co2: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::ExecutePeriod {
                burn_amount: sim.mint_amount,
                staked_amount: Uint128::zero(),
                stability_committed: Uint128::zero(),
                delta_co2: None,
            },
        )
        .unwrap();
    }

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::SupplyState {}).unwrap()).unwrap();
    assert_eq!(state.phase, SupplyPhase::Equilibrium);

    // External shock: massive burn with no corresponding mint
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::new(10_000_000_000_000), // 10M REGEN burn shock
            staked_amount: Uint128::zero(),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyState {}).unwrap()).unwrap();
    assert_eq!(state.phase, SupplyPhase::Dynamic);
    assert_eq!(state.consecutive_equilibrium_periods, 0);
}

// =========================================================================
// SPEC Invariant 16: Cap inviolability — S[t] <= hard_cap at all times
// =========================================================================
#[test]
fn test_cap_inviolability_invariant() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // Start very close to cap with aggressive regrowth
    let msg = InstantiateMsg {
        hard_cap: Uint128::new(100_000_000),
        initial_supply: Uint128::new(99_000_000), // 1M headroom
        base_regrowth_rate: Decimal::percent(10),  // Max rate
        ecological_multiplier_enabled: false,
        ecological_reference_value: Decimal::from_atomics(50u128, 0).unwrap(),
        m014_phase: M014Phase::Inactive,
        equilibrium_threshold: Uint128::new(1_000),
        equilibrium_periods_required: 12,
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Execute with 100% staked (max multiplier 2.0), zero burn
    // r = 0.10 * 2.0 = 0.20, headroom = 1M, M = 200_000
    // Even after repeated executions, supply must never exceed cap
    for _ in 0..20 {
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::ExecutePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::new(99_000_000),
                stability_committed: Uint128::zero(),
                delta_co2: None,
            },
        )
        .unwrap();

        let state: SupplyStateResponse =
            from_json(query(deps.as_ref(), env.clone(), QueryMsg::SupplyState {}).unwrap())
                .unwrap();
        assert!(state.current_supply <= Uint128::new(100_000_000));
    }
}

// =========================================================================
// SPEC Invariant 17: Non-negative supply — S[t] >= 0 at all times
// =========================================================================
#[test]
fn test_non_negative_supply_invariant() {
    let (mut deps, env) = setup_contract();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // Burn far more than exists
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::new(999_000_000_000_000_000),
            staked_amount: Uint128::zero(),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyState {}).unwrap()).unwrap();
    assert_eq!(state.current_supply, Uint128::zero());
}

// =========================================================================
// SPEC Invariant 20: Parameter bound safety — r_base in [0, 0.10]
// =========================================================================
#[test]
fn test_regrowth_rate_bound_safety() {
    let (mut deps, env) = setup_contract();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // Try to set rate to 11% -> should fail
    let err = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::UpdateRegrowthRate {
            rate: Decimal::percent(11),
        },
    )
    .unwrap_err();

    match err {
        ContractError::RegrowthRateExceedsBound { .. } => {}
        e => panic!("Expected RegrowthRateExceedsBound, got: {:?}", e),
    }

    // Exactly 10% should succeed
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::UpdateRegrowthRate {
            rate: Decimal::percent(10),
        },
    )
    .unwrap();

    // 0% should succeed
    execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::UpdateRegrowthRate {
            rate: Decimal::zero(),
        },
    )
    .unwrap();
}

// =========================================================================
// Admin authorization tests
// =========================================================================
#[test]
fn test_unauthorized_execute_period() {
    let (mut deps, env) = setup_contract();
    let other = message_info(&Addr::unchecked("other"), &[]);

    let err = execute(
        deps.as_mut(),
        env,
        other,
        ExecuteMsg::ExecutePeriod {
            burn_amount: Uint128::zero(),
            staked_amount: Uint128::zero(),
            stability_committed: Uint128::zero(),
            delta_co2: None,
        },
    )
    .unwrap_err();

    match err {
        ContractError::Unauthorized {} => {}
        e => panic!("Expected Unauthorized, got: {:?}", e),
    }
}

#[test]
fn test_unauthorized_update_regrowth_rate() {
    let (mut deps, env) = setup_contract();
    let other = message_info(&Addr::unchecked("other"), &[]);

    let err = execute(
        deps.as_mut(),
        env,
        other,
        ExecuteMsg::UpdateRegrowthRate {
            rate: Decimal::percent(5),
        },
    )
    .unwrap_err();

    match err {
        ContractError::Unauthorized {} => {}
        e => panic!("Expected Unauthorized, got: {:?}", e),
    }
}

#[test]
fn test_unauthorized_update_m014_phase() {
    let (mut deps, env) = setup_contract();
    let other = message_info(&Addr::unchecked("other"), &[]);

    let err = execute(
        deps.as_mut(),
        env,
        other,
        ExecuteMsg::UpdateM014Phase {
            phase: M014Phase::Active,
        },
    )
    .unwrap_err();

    match err {
        ContractError::Unauthorized {} => {}
        e => panic!("Expected Unauthorized, got: {:?}", e),
    }
}

// =========================================================================
// Instantiation validation tests
// =========================================================================
#[test]
fn test_instantiation_fails_zero_cap() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.hard_cap = Uint128::zero();

    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    match err {
        ContractError::ZeroCap {} => {}
        e => panic!("Expected ZeroCap, got: {:?}", e),
    }
}

#[test]
fn test_instantiation_fails_supply_exceeds_cap() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.initial_supply = Uint128::new(300_000_000_000_000); // > 221M cap

    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    match err {
        ContractError::SupplyExceedsCap { .. } => {}
        e => panic!("Expected SupplyExceedsCap, got: {:?}", e),
    }
}

#[test]
fn test_instantiation_fails_rate_too_high() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let mut msg = default_instantiate_msg();
    msg.base_regrowth_rate = Decimal::percent(15); // 15% > 10% max

    let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
    match err {
        ContractError::RegrowthRateExceedsBound { .. } => {}
        e => panic!("Expected RegrowthRateExceedsBound, got: {:?}", e),
    }
}

// =========================================================================
// Query: supply params
// =========================================================================
#[test]
fn test_query_supply_params() {
    let (deps, env) = setup_contract();

    let resp: SupplyParamsResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyParams {}).unwrap()).unwrap();

    assert_eq!(resp.admin, "admin");
    assert_eq!(resp.hard_cap, Uint128::new(221_000_000_000_000));
    assert_eq!(resp.base_regrowth_rate, Decimal::percent(2));
    assert!(!resp.ecological_multiplier_enabled);
    assert_eq!(resp.m014_phase, M014Phase::Inactive);
    assert_eq!(resp.equilibrium_periods_required, 12);
}

// =========================================================================
// Query: cap headroom
// =========================================================================
#[test]
fn test_query_cap_headroom() {
    let (deps, env) = setup_contract();

    let state: SupplyStateResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyState {}).unwrap()).unwrap();

    // 221M - 200M = 21M REGEN = 21_000_000_000_000 uregen
    assert_eq!(state.cap_headroom, Uint128::new(21_000_000_000_000));
}

// =========================================================================
// Multi-period simulation: supply converges toward cap
// =========================================================================
#[test]
fn test_supply_converges_toward_cap() {
    let (mut deps, env) = setup_contract();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let cap = Uint128::new(221_000_000_000_000);
    let mut prev_headroom = Uint128::new(21_000_000_000_000);

    // Run 10 periods with no burn -> supply should steadily approach cap
    for _ in 0..10 {
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::ExecutePeriod {
                burn_amount: Uint128::zero(),
                staked_amount: Uint128::zero(),
                stability_committed: Uint128::zero(),
                delta_co2: None,
            },
        )
        .unwrap();

        let state: SupplyStateResponse =
            from_json(query(deps.as_ref(), env.clone(), QueryMsg::SupplyState {}).unwrap())
                .unwrap();

        // Headroom should be shrinking
        assert!(state.cap_headroom < prev_headroom);
        // Supply must never exceed cap
        assert!(state.current_supply <= cap);
        prev_headroom = state.cap_headroom;
    }
}

// =========================================================================
// UpdateM014Phase and SetEcologicalMultiplier admin controls
// =========================================================================
#[test]
fn test_update_m014_phase() {
    let (mut deps, env) = setup_contract();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::UpdateM014Phase {
            phase: M014Phase::Active,
        },
    )
    .unwrap();

    let params: SupplyParamsResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyParams {}).unwrap()).unwrap();
    assert_eq!(params.m014_phase, M014Phase::Active);
}

#[test]
fn test_set_ecological_multiplier() {
    let (mut deps, env) = setup_contract();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    // Enable with updated reference value
    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::SetEcologicalMultiplier {
            enabled: true,
            reference_value: Some(Decimal::from_atomics(100u128, 0).unwrap()),
        },
    )
    .unwrap();

    let params: SupplyParamsResponse =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::SupplyParams {}).unwrap()).unwrap();
    assert!(params.ecological_multiplier_enabled);
    assert_eq!(
        params.ecological_reference_value,
        Decimal::from_atomics(100u128, 0).unwrap()
    );

    // Disable
    execute(
        deps.as_mut(),
        env.clone(),
        info,
        ExecuteMsg::SetEcologicalMultiplier {
            enabled: false,
            reference_value: None,
        },
    )
    .unwrap();

    let params: SupplyParamsResponse =
        from_json(query(deps.as_ref(), env, QueryMsg::SupplyParams {}).unwrap()).unwrap();
    assert!(!params.ecological_multiplier_enabled);
}

#[test]
fn test_set_ecological_multiplier_fails_zero_ref() {
    let (mut deps, env) = setup_contract();
    let info = message_info(&Addr::unchecked("admin"), &[]);

    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SetEcologicalMultiplier {
            enabled: true,
            reference_value: Some(Decimal::zero()),
        },
    )
    .unwrap_err();

    match err {
        ContractError::ZeroReferenceValue {} => {}
        e => panic!("Expected ZeroReferenceValue, got: {:?}", e),
    }
}
