# M013 Fee Router — Integration Test Specification

## Overview

**Purpose:** Validate the CosmWasm fee routing contract's behavior across edge cases, state transitions, and cross-contract interactions. This specification builds on PR #36's contract architecture, where Gregory made the design decision to implement M013 as a CosmWasm contract rather than a native Cosmos SDK module.

**Scope:** All execute messages (`ProcessFee`, `UpdateFeeSchedule`, `UpdatePoolDistribution`, `SetEnabled`), all query messages (`Config`, `SimulateFee`, `Stats`, `FeeEvents`), cross-contract interactions with `x/ecocredit`, and security invariant enforcement.

**Core fee formula under test:**

```
fee_amount = max(credit_value * rate_bps / 10000, min_fee_uregen)
```

**Pool distribution constraint:**

```
burn_bps + validator_fund_bps + community_pool_bps + agent_infra_bps = 10000
```

---

## Test Environment Setup

### cw-multi-test Harness

All integration tests run inside a `cw-multi-test` based harness. No live chain required for the core test suite.

```rust
// Pseudocode: test environment initialization
fn setup_test_env() -> (App, Addr, Addr, Addr, Addr) {
    let mut app = App::default();

    // Deploy fee router contract
    let fee_router_code_id = app.store_code(fee_router_contract());
    let fee_router_addr = app.instantiate_contract(
        fee_router_code_id,
        admin.clone(),
        &InstantiateMsg {
            admin: admin.to_string(),
            fee_schedule: default_fee_schedule(),
            pool_distribution: default_pool_distribution(),
            enabled: false, // starts disabled per lifecycle spec
        },
        &[],
        "fee-router",
        None,
    );

    // Deploy mock x/ecocredit contract
    let ecocredit_mock_addr = deploy_ecocredit_mock(&mut app);

    (app, fee_router_addr, admin, fee_payer, governance)
}
```

### Mock Contracts

| Mock Contract         | Purpose                                                      |
|-----------------------|--------------------------------------------------------------|
| `mock_ecocredit`      | Simulates `x/ecocredit` module responses (batch creation, transfers, retirements) |
| `mock_marketplace`    | Returns credit prices for value estimation                   |
| `mock_validator_fund` | Receives validator pool distributions                        |
| `mock_community_pool` | Receives community pool distributions                        |
| `mock_agent_infra`    | Receives agent infrastructure pool distributions             |

### Test Accounts

| Account       | Role                                        | Initial Balance     |
|---------------|---------------------------------------------|---------------------|
| `admin`       | Contract admin; authorized for all execute msgs | 1,000,000 uregen    |
| `fee_payer`   | Submits transactions that trigger fee collection | 10,000,000 uregen   |
| `governance`  | Authorized for `UpdateFeeSchedule` and `UpdatePoolDistribution` | 0 uregen            |
| `unauthorized`| No special permissions; used for access control tests | 1,000,000 uregen    |

---

## Test Vectors

### Fee Calculation Test Vectors

All values in uregen (1 REGEN = 1,000,000 uregen) unless noted otherwise.

| test_id | tx_type    | credit_value (uregen) | rate_bps | min_fee (uregen) | expected_fee (uregen) | notes |
|---------|------------|-----------------------|----------|------------------|-----------------------|-------|
| TC-001  | issuance   | 100,000,000           | 200      | 1,000,000        | 2,000,000             | Standard 2% issuance fee: 100 REGEN * 200/10000 = 2 REGEN |
| TC-002  | transfer   | 500,000               | 10       | 1,000,000        | 1,000,000             | Calculated fee = 500 uregen, below min_fee floor of 1 REGEN; min_fee applied |
| TC-003  | retirement | 0                     | 200      | 1,000,000        | 1,000,000             | Zero-value credit transaction; min_fee floor applied |
| TC-004  | trade      | 10,000,000,000,000    | 100      | 1,000,000        | 100,000,000,000       | 10M REGEN at 1%; verifies large integer arithmetic (100K REGEN fee) |
| TC-005  | issuance   | 100,000,000           | 2000     | 1,000,000        | 1,000,000             | Rate 20% exceeds max_fee_rate_bps (10% = 1000 bps); capped to 1000 bps, fee = 1,000,000 uregen |
| TC-006a | issuance   | 50,000,000            | 150      | 500,000          | 750,000               | Issuance at 1.5%: 50 REGEN * 150/10000 = 0.75 REGEN |
| TC-006b | transfer   | 50,000,000            | 100      | 500,000          | 500,000               | Transfer at 1%: 50 REGEN * 100/10000 = 0.5 REGEN = 500,000 uregen |
| TC-006c | retirement | 50,000,000            | 250      | 500,000          | 1,250,000             | Retirement at 2.5% |
| TC-006d | trade      | 50,000,000            | 50       | 500,000          | 500,000               | Trade at 0.5%: calculated = 250,000 uregen, below min_fee; floor applied |

### Pool Distribution Test Vectors

All values in uregen. Pools: B = burn, V = validator_fund, C = community_pool, A = agent_infra.

| test_id | fee_amount | burn_bps | validator_bps | community_bps | agent_bps | expected_burn | expected_validator | expected_community | expected_agent | notes |
|---------|------------|----------|---------------|---------------|-----------|---------------|--------------------|--------------------|----------------|-------|
| TD-001  | 2,000,000  | 3000     | 4000          | 2500          | 500       | 600,000       | 800,000            | 500,000            | 100,000        | Standard split; sum = 2,000,000 |
| TD-002  | 1,000,003  | 3000     | 4000          | 2500          | 500       | 300,000       | 400,001            | 250,000            | 50,000         | Remainder of 2 uregen distributed to largest-share pool (validator); total = 1,000,001 -- see note below |
| TD-003  | 5,000,000  | 10000    | 0             | 0             | 0         | 5,000,000     | 0                  | 0                  | 0              | 100% burn edge case |
| TD-004  | 4,000,000  | 2500     | 2500          | 2500          | 2500      | 1,000,000     | 1,000,000          | 1,000,000          | 1,000,000      | Equal four-way split; no remainder |
| TD-005  | 1          | 3000     | 4000          | 2500          | 500       | 0             | 1                  | 0                  | 0              | 1 uregen fee; only largest-share pool receives; no overflow |

**TD-002 Remainder Handling Note:** With integer-only arithmetic, `1,000,003 * 3000 / 10000 = 300,000` (truncated), `1,000,003 * 4000 / 10000 = 400,001` (truncated), `1,000,003 * 2500 / 10000 = 250,000` (truncated), `1,000,003 * 500 / 10000 = 50,000` (truncated). Sum of truncated = 1,000,001. Remainder = 2 uregen. Remainder is distributed one uregen at a time to pools in descending share order (validator first, then burn). The exact remainder-handling strategy must match PR #36's implementation; this vector validates that **fee_amount == sum(distributions)** (the FEE_CONSERVATION invariant).

---

## Edge Case Tests

### Zero-Value Transactions

```yaml
test_id: EC-001
name: zero_value_credit_triggers_min_fee
setup:
  fee_schedule:
    issuance_rate_bps: 200
    min_fee_uregen: 1000000
  pool_distribution:
    burn_bps: 3000
    validator_fund_bps: 4000
    community_pool_bps: 2500
    agent_infra_bps: 500
  fee_payer_balance: 10000000
action:
  msg: ProcessFee
  params:
    tx_type: issuance
    credit_value: 0
    payer: fee_payer
expected:
  fee_collected: 1000000  # min_fee applied
  burn: 300000
  validator_fund: 400000
  community_pool: 250000
  agent_infra: 50000
  fee_payer_balance_after: 9000000
  events:
    - type: fee_collected
      attributes:
        fee_amount: "1000000"
        applied_min_fee: "true"
```

### Fee Cap Breach

```yaml
test_id: EC-002
name: fee_rate_capped_at_max
setup:
  fee_schedule:
    trade_rate_bps: 2500  # 25%, exceeds max
    max_fee_rate_bps: 1000  # 10% cap
    min_fee_uregen: 100000
  credit_value: 100000000  # 100 REGEN
action:
  msg: ProcessFee
  params:
    tx_type: trade
    credit_value: 100000000
    payer: fee_payer
expected:
  effective_rate_bps: 1000  # capped
  fee_collected: 10000000  # 100 REGEN * 10% = 10 REGEN
  events:
    - type: fee_collected
      attributes:
        rate_capped: "true"
        requested_rate_bps: "2500"
        effective_rate_bps: "1000"
```

### Multi-Hop Routing (IBC-Originating Transaction)

```yaml
test_id: EC-003
name: ibc_originating_fee_collection
setup:
  # Fee payer holds IBC-transferred tokens
  fee_payer_balance:
    uregen: 5000000
    ibc/ABC123: 10000000
  credit_value: 50000000  # denominated in uregen equivalent
action:
  msg: ProcessFee
  params:
    tx_type: trade
    credit_value: 50000000
    payer: fee_payer
    denom: uregen  # fee always collected in native denom
expected:
  fee_collected_denom: uregen
  fee_collected_amount: 500000  # at 1% rate
  note: "Fee is always collected in uregen regardless of transaction origin"
```

### Circuit Breaker

```yaml
test_id: EC-004a
name: circuit_breaker_disable_blocks_fees
setup:
  enabled: true
action:
  - msg: SetEnabled
    params:
      enabled: false
    sender: admin
  - msg: ProcessFee
    params:
      tx_type: issuance
      credit_value: 100000000
      payer: fee_payer
expected:
  SetEnabled_result: success
  ProcessFee_result: error
  error_contains: "contract is disabled"
  fee_payer_balance_unchanged: true
```

```yaml
test_id: EC-004b
name: circuit_breaker_re_enable_resumes_fees
setup:
  enabled: false  # previously disabled
action:
  - msg: SetEnabled
    params:
      enabled: true
    sender: admin
  - msg: ProcessFee
    params:
      tx_type: issuance
      credit_value: 100000000
      payer: fee_payer
expected:
  SetEnabled_result: success
  ProcessFee_result: success
  fee_collected: 2000000
```

### Insufficient Balance

```yaml
test_id: EC-005
name: insufficient_balance_reverts_cleanly
setup:
  fee_schedule:
    issuance_rate_bps: 200
    min_fee_uregen: 1000000
  fee_payer_balance: 500000  # less than min_fee
action:
  msg: ProcessFee
  params:
    tx_type: issuance
    credit_value: 10000000
    payer: fee_payer
expected:
  result: error
  error_contains: "insufficient funds"
  fee_payer_balance_after: 500000  # unchanged
  pool_balances_unchanged: true
  stats_unchanged: true
```

### Concurrent Fee Collection

```yaml
test_id: EC-006
name: multiple_fees_same_block
setup:
  pool_distribution:
    burn_bps: 2500
    validator_fund_bps: 2500
    community_pool_bps: 2500
    agent_infra_bps: 2500
  fee_payer_balance: 100000000
action:
  # Three ProcessFee calls executed in the same block
  - msg: ProcessFee
    params: { tx_type: trade, credit_value: 10000000, payer: fee_payer }
  - msg: ProcessFee
    params: { tx_type: transfer, credit_value: 20000000, payer: fee_payer }
  - msg: ProcessFee
    params: { tx_type: retirement, credit_value: 30000000, payer: fee_payer }
expected:
  all_succeed: true
  total_fees_collected: sum_of_individual_fees
  pool_balances: sum_of_individual_distributions
  stats_total_fees: incremented_by_total
  fee_events_count: 3
  note: "Each ProcessFee is independent; no cross-contamination between calls"
```

### Rounding Edge Cases

```yaml
test_id: EC-007a
name: one_uregen_four_pools
setup:
  pool_distribution:
    burn_bps: 2500
    validator_fund_bps: 2500
    community_pool_bps: 2500
    agent_infra_bps: 2500
action:
  msg: ProcessFee
  params:
    tx_type: issuance
    credit_value: 1  # triggers min_fee
    payer: fee_payer
  # Assuming min_fee is set to 1 uregen for this test
  override_min_fee: 1
expected:
  fee_collected: 1
  # 1 * 2500 / 10000 = 0 for each pool (truncated)
  # Remainder = 1 uregen, goes to first pool in priority order
  distributions_sum: 1
  no_arithmetic_overflow: true
  note: "Exactly one pool receives 1 uregen; others receive 0"
```

```yaml
test_id: EC-007b
name: three_uregen_four_pools
setup:
  pool_distribution:
    burn_bps: 2500
    validator_fund_bps: 2500
    community_pool_bps: 2500
    agent_infra_bps: 2500
  override_min_fee: 3
action:
  msg: ProcessFee
  params:
    tx_type: issuance
    credit_value: 1
    payer: fee_payer
expected:
  fee_collected: 3
  # 3 * 2500 / 10000 = 0 for each pool (truncated)
  # Remainder = 3 uregen, distributed one each to first 3 pools
  distributions_sum: 3
  note: "Three pools receive 1 uregen each; fourth receives 0"
```

### Governance Parameter Updates

```yaml
test_id: EC-008a
name: fee_schedule_update_applies_to_subsequent_calls
setup:
  fee_schedule:
    issuance_rate_bps: 200
    min_fee_uregen: 1000000
action:
  - msg: ProcessFee
    params: { tx_type: issuance, credit_value: 100000000, payer: fee_payer }
    expected_fee: 2000000  # 2% of 100 REGEN
  - msg: UpdateFeeSchedule
    params:
      issuance_rate_bps: 500  # raise to 5%
    sender: governance
  - msg: ProcessFee
    params: { tx_type: issuance, credit_value: 100000000, payer: fee_payer }
    expected_fee: 5000000  # 5% of 100 REGEN (new rate)
expected:
  first_fee: 2000000
  second_fee: 5000000
  note: "Updated rate applies immediately to subsequent ProcessFee calls"
```

```yaml
test_id: EC-008b
name: invalid_pool_distribution_rejected
action:
  msg: UpdatePoolDistribution
  params:
    burn_bps: 3000
    validator_fund_bps: 4000
    community_pool_bps: 2500
    agent_infra_bps: 600  # sum = 10100, not 10000
  sender: governance
expected:
  result: error
  error_contains: "pool distribution must sum to 10000"
  pool_distribution_unchanged: true
```

```yaml
test_id: EC-008c
name: unauthorized_update_rejected
action:
  msg: UpdateFeeSchedule
  params:
    issuance_rate_bps: 500
  sender: unauthorized  # not admin or governance
expected:
  result: error
  error_contains: "unauthorized"
  fee_schedule_unchanged: true
```

---

## State Transition Tests

### DISABLED -> ENABLED -> DISABLED Lifecycle

```yaml
test_suite: ST-001
name: full_contract_lifecycle
steps:
  - step: 1
    name: contract_starts_disabled
    action:
      query: Config
    expected:
      enabled: false

  - step: 2
    name: process_fee_while_disabled
    action:
      msg: ProcessFee
      params: { tx_type: issuance, credit_value: 100000000, payer: fee_payer }
    expected:
      result: error
      error_contains: "contract is disabled"

  - step: 3
    name: queries_work_while_disabled
    action:
      query: SimulateFee
      params: { tx_type: issuance, credit_value: 100000000 }
    expected:
      result: success
      simulated_fee: 2000000
      note: "Read-only queries are always available"

  - step: 4
    name: admin_enables_contract
    action:
      msg: SetEnabled
      params: { enabled: true }
      sender: admin
    expected:
      result: success
      events:
        - type: contract_enabled

  - step: 5
    name: process_fee_succeeds
    action:
      msg: ProcessFee
      params: { tx_type: issuance, credit_value: 100000000, payer: fee_payer }
    expected:
      result: success
      fee_collected: 2000000

  - step: 6
    name: process_multiple_fees
    action:
      - msg: ProcessFee
        params: { tx_type: trade, credit_value: 50000000, payer: fee_payer }
      - msg: ProcessFee
        params: { tx_type: retirement, credit_value: 75000000, payer: fee_payer }
    expected:
      both_succeed: true

  - step: 7
    name: verify_stats_accumulated
    action:
      query: Stats
    expected:
      total_fees_collected: sum_of_all_fees
      total_transactions: 3
      fees_by_type:
        issuance: 1
        trade: 1
        retirement: 1

  - step: 8
    name: admin_disables_circuit_breaker
    action:
      msg: SetEnabled
      params: { enabled: false }
      sender: admin
    expected:
      result: success
      events:
        - type: contract_disabled

  - step: 9
    name: process_fee_blocked_again
    action:
      msg: ProcessFee
      params: { tx_type: issuance, credit_value: 100000000, payer: fee_payer }
    expected:
      result: error
      error_contains: "contract is disabled"

  - step: 10
    name: stats_still_queryable_while_disabled
    action:
      query: Stats
    expected:
      result: success
      total_transactions: 3
      note: "Historical data preserved and accessible even when contract is disabled"

  - step: 11
    name: fee_events_queryable_while_disabled
    action:
      query: FeeEvents
      params: { limit: 10 }
    expected:
      result: success
      events_count: 3
```

---

## Cross-Contract Integration Tests

### Fee Collection via x/ecocredit MsgCreateBatch

```yaml
test_id: XC-001
name: ecocredit_batch_creation_triggers_fee
description: >
  When a new credit batch is created via x/ecocredit MsgCreateBatch,
  the fee router collects an issuance fee. Integration path depends on
  PR #36 Option A (native hook) or Option B (wrapper contract).
setup:
  fee_schedule:
    issuance_rate_bps: 200
    min_fee_uregen: 1000000
  mock_ecocredit:
    batch_value: 500000000  # 500 REGEN equivalent
action:
  # Option A: hook fires automatically after MsgCreateBatch
  # Option B: wrapper contract calls ProcessFee then MsgCreateBatch
  trigger: MsgCreateBatch
  params:
    issuer: fee_payer
    class_id: "C01"
    metadata: "test-batch"
    credits: 1000
expected:
  fee_collected: 10000000  # 500 REGEN * 2% = 10 REGEN
  batch_created: true
  note: >
    Test must validate that batch creation succeeds only after fee is collected.
    If fee collection fails, batch creation must also fail (atomic).
  architecture_note: >
    PR #36 specifies two integration architectures (Option A: native hook,
    Option B: wrapper contract). This test case is architecture-agnostic —
    the expected fee amount and atomicity guarantee apply to both options.
    However, gas costs differ: Option A incurs lower gas (~50K less per tx)
    due to native execution. Performance benchmarks (PB-001 through PB-003)
    should be run separately for each option when the architecture decision
    is finalized.
```

### Fee Collection via Marketplace MsgBuyDirect

```yaml
test_id: XC-002
name: marketplace_buy_triggers_trade_fee
setup:
  fee_schedule:
    trade_rate_bps: 100  # 1%
    min_fee_uregen: 500000
  mock_marketplace:
    sell_order:
      id: 1
      price_per_credit: 5000000  # 5 REGEN per credit
      quantity: 100
action:
  trigger: MsgBuyDirect
  params:
    buyer: fee_payer
    sell_order_id: 1
    quantity: 100
    # Total value: 100 * 5 REGEN = 500 REGEN = 500,000,000 uregen
expected:
  credit_value_estimated: 500000000
  fee_collected: 5000000  # 500 REGEN * 1% = 5 REGEN
  trade_executed: true
```

### Credit Value Estimation Fallback

```yaml
test_id: XC-003
name: value_estimation_fallback_to_governance_default
description: >
  When no marketplace price is available for a credit class (e.g., new issuance
  or transfer of a class with no trade history), the contract falls back to a
  governance-configured default price.
setup:
  fee_schedule:
    issuance_rate_bps: 200
    min_fee_uregen: 1000000
  mock_marketplace:
    price_available: false  # no trade history
  governance_defaults:
    default_credit_value_uregen: 10000000  # 10 REGEN per credit
  credits_in_batch: 50
action:
  msg: ProcessFee
  params:
    tx_type: issuance
    credit_class: "C02"  # no market price
    credit_amount: 50
    payer: fee_payer
expected:
  estimated_value: 500000000  # 50 credits * 10 REGEN = 500 REGEN
  fee_collected: 10000000  # 500 REGEN * 2% = 10 REGEN
  estimation_method: "governance_default"
  events:
    - type: fee_collected
      attributes:
        value_source: "governance_default"
```

### Burn Event Consumption by M012 Supply Module

```yaml
test_id: XC-004
name: burn_events_formatted_for_m012
description: >
  When the fee router burns tokens via BankMsg::Burn, the emitted events
  must be in the format that M012 (supply tracking module) expects to read.
  This validates the event schema compatibility between M013 and M012.
setup:
  pool_distribution:
    burn_bps: 5000  # 50% to burn
    validator_fund_bps: 2000
    community_pool_bps: 2000
    agent_infra_bps: 1000
  fee_amount: 10000000  # 10 REGEN fee
action:
  msg: ProcessFee
  params:
    tx_type: retirement
    credit_value: 200000000
    payer: fee_payer
expected:
  burn_amount: 5000000  # 50% of 10 REGEN = 5 REGEN
  burn_event:
    type: "wasm-token_burn"
    attributes:
      contract_addr: fee_router_addr
      amount: "5000000"
      denom: "uregen"
      burn_source: "fee_router"
      block_height: current_height
  m012_compatibility:
    event_parseable: true
    note: >
      M012 supply module must be able to parse this event to update
      total_burned counters. Event type and attribute keys must match
      M012's expected schema exactly.
```

---

## Invariant Tests

These invariants must hold after **every** `ProcessFee`, `UpdateFeeSchedule`, and `UpdatePoolDistribution` operation. Test harness should run invariant checks as post-conditions on every test.

### FEE_CONSERVATION

```yaml
invariant_id: INV-001
name: fee_conservation
description: >
  The sum of all pool distributions must exactly equal the total fee collected.
  No uregen is created or destroyed during distribution (burns are a valid distribution target).
check: |
  for every ProcessFee execution:
    assert(
      burn_amount + validator_amount + community_amount + agent_amount
      == fee_collected
    )
enforcement: post-condition on every test that calls ProcessFee
failure_action: test fails immediately; log all distribution amounts
```

### SHARE_SUM_UNITY

```yaml
invariant_id: INV-002
name: share_sum_unity
description: >
  Pool distribution basis points must always sum to exactly 10000.
  Validated on every UpdatePoolDistribution and during contract instantiation.
check: |
  assert(
    burn_bps + validator_fund_bps + community_pool_bps + agent_infra_bps
    == 10000
  )
enforcement: post-condition on every UpdatePoolDistribution call
failure_action: >
  UpdatePoolDistribution must reject the transaction before state change.
  Test verifies that on-chain state is unchanged after a rejected update.
```

### RATE_BOUNDS

```yaml
invariant_id: INV-003
name: rate_bounds
description: >
  Effective fee rate must never exceed max_fee_rate_bps after any UpdateFeeSchedule.
  ProcessFee must apply the cap even if stored rate somehow exceeds max.
check: |
  for each tx_type in [issuance, transfer, retirement, trade]:
    assert(effective_rate_bps(tx_type) <= max_fee_rate_bps)
enforcement: post-condition on every UpdateFeeSchedule and ProcessFee call
failure_action: >
  If rate exceeds max, ProcessFee must cap it. If UpdateFeeSchedule allows
  storage of a rate above max, that is a bug (test should flag it).
```

### NON_NEGATIVE

```yaml
invariant_id: INV-004
name: non_negative_balances
description: >
  All pool balances, fee amounts, and distribution amounts must be >= 0
  after every operation. Integer-only arithmetic with unsigned types should
  enforce this, but edge cases (e.g., subtraction underflow) must be tested.
check: |
  assert(burn_amount >= 0)
  assert(validator_amount >= 0)
  assert(community_amount >= 0)
  assert(agent_amount >= 0)
  assert(fee_collected >= 0)
  assert(fee_payer_balance_after >= 0)
enforcement: post-condition on every state-modifying operation
failure_action: test fails; potential integer underflow vulnerability
```

### Invariant Stress Test

```yaml
test_id: INV-STRESS-001
name: invariants_hold_under_random_inputs
description: >
  Property-based / fuzz-style test that generates random valid inputs
  and verifies all four invariants hold after each operation.
parameters:
  iterations: 10000
  random_ranges:
    credit_value: [0, 10_000_000_000_000]  # 0 to 10M REGEN
    rate_bps: [0, 10000]
    min_fee_uregen: [0, 100_000_000]
    pool_bps: random_valid_partition_of_10000
checks_per_iteration:
  - FEE_CONSERVATION
  - SHARE_SUM_UNITY
  - RATE_BOUNDS
  - NON_NEGATIVE
```

---

## Performance Benchmarks

### ProcessFee Gas Cost

| benchmark_id | scenario                          | target_gas | max_acceptable_gas | notes |
|--------------|-----------------------------------|------------|-------------------|-------|
| PB-001       | ProcessFee (issuance, standard)   | < 150,000  | 250,000           | Single fee with 4-pool distribution |
| PB-002       | ProcessFee (trade, with price lookup) | < 200,000  | 350,000           | Includes cross-contract query for price |
| PB-003       | ProcessFee (fallback to governance default) | < 180,000  | 300,000        | No marketplace price; uses default |
| PB-004       | SimulateFee query                 | < 50,000   | 100,000           | Read-only, no state changes |

### Batch Processing

```yaml
test_id: PB-005
name: batch_100_fees_single_block
description: >
  Process 100 sequential ProcessFee calls within a single block.
  Measures total gas and verifies no state corruption.
parameters:
  fee_count: 100
  credit_value_each: 10000000  # 10 REGEN
  tx_type: trade
assertions:
  all_succeed: true
  total_gas: measured
  avg_gas_per_fee: measured
  invariants_hold: true
  stats_total_transactions: 100
```

### Stats Query Scalability

| benchmark_id | fee_record_count | query              | target_response_time | notes |
|--------------|------------------|--------------------|---------------------|-------|
| PB-006a      | 10,000           | Stats              | < 50ms              | Aggregate stats query |
| PB-006b      | 100,000          | Stats              | < 100ms             | Should not degrade linearly |
| PB-006c      | 1,000,000        | Stats              | < 200ms             | Requires efficient storage indexing |
| PB-007a      | 10,000           | FeeEvents (last 100) | < 30ms            | Paginated query |
| PB-007b      | 1,000,000        | FeeEvents (last 100) | < 50ms            | Pagination must avoid full scan |

**Note:** If Stats is computed incrementally (updated on each ProcessFee), query cost should be O(1) regardless of history size. If computed from event log, benchmarks PB-006b/c are critical.

---

## Summary

| test_id        | category                  | description                                          | priority |
|----------------|---------------------------|------------------------------------------------------|----------|
| TC-001         | Fee Calculation           | Standard issuance fee (2%)                           | P0       |
| TC-002         | Fee Calculation           | Minimum fee floor enforcement                        | P0       |
| TC-003         | Fee Calculation           | Zero-value transaction triggers min fee              | P0       |
| TC-004         | Fee Calculation           | Large value integer arithmetic                       | P1       |
| TC-005         | Fee Calculation           | Rate cap enforcement                                 | P0       |
| TC-006a-d      | Fee Calculation           | All transaction types at standard rates              | P0       |
| TD-001         | Pool Distribution         | Standard four-pool split                             | P0       |
| TD-002         | Pool Distribution         | Remainder / dust handling                            | P0       |
| TD-003         | Pool Distribution         | 100% to single pool                                  | P1       |
| TD-004         | Pool Distribution         | Equal four-way split                                 | P1       |
| TD-005         | Pool Distribution         | Minimum 1 uregen distribution                        | P0       |
| EC-001         | Edge Case                 | Zero-value credit triggers min fee with distribution | P0       |
| EC-002         | Edge Case                 | Fee rate capped at max, event emitted                | P0       |
| EC-003         | Edge Case                 | IBC-originating transaction fee collection           | P2       |
| EC-004a        | Edge Case / Circuit Breaker | Disable blocks ProcessFee                          | P0       |
| EC-004b        | Edge Case / Circuit Breaker | Re-enable resumes ProcessFee                       | P0       |
| EC-005         | Edge Case                 | Insufficient balance reverts cleanly                 | P0       |
| EC-006         | Edge Case                 | Multiple fees in same block                          | P1       |
| EC-007a        | Edge Case / Rounding      | 1 uregen split across 4 pools                        | P0       |
| EC-007b        | Edge Case / Rounding      | 3 uregen split across 4 pools                        | P1       |
| EC-008a        | Governance                | Fee schedule update applies to subsequent calls      | P0       |
| EC-008b        | Governance                | Invalid pool distribution rejected                   | P0       |
| EC-008c        | Governance                | Unauthorized update rejected                         | P0       |
| ST-001         | State Transition          | Full DISABLED -> ENABLED -> DISABLED lifecycle       | P0       |
| XC-001         | Cross-Contract            | ecocredit batch creation triggers issuance fee       | P0       |
| XC-002         | Cross-Contract            | Marketplace buy triggers trade fee                   | P0       |
| XC-003         | Cross-Contract            | Value estimation fallback to governance default      | P1       |
| XC-004         | Cross-Contract            | Burn events formatted for M012 consumption           | P1       |
| INV-001        | Invariant                 | Fee conservation (sum distributions == fee)          | P0       |
| INV-002        | Invariant                 | Share sum unity (bps sum == 10000)                   | P0       |
| INV-003        | Invariant                 | Rate bounds (effective <= max)                       | P0       |
| INV-004        | Invariant                 | Non-negative balances                                | P0       |
| INV-STRESS-001 | Invariant                 | Fuzz test: invariants under 10K random inputs        | P1       |
| PB-001         | Performance               | ProcessFee gas (issuance)                            | P2       |
| PB-002         | Performance               | ProcessFee gas (trade with price lookup)             | P2       |
| PB-003         | Performance               | ProcessFee gas (fallback pricing)                    | P2       |
| PB-004         | Performance               | SimulateFee query gas                                | P2       |
| PB-005         | Performance               | Batch 100 fees in single block                       | P2       |
| PB-006a-c      | Performance               | Stats query scalability (10K-1M records)             | P2       |
| PB-007a-b      | Performance               | FeeEvents pagination scalability                     | P2       |

**Priority Legend:**
- **P0** — Must pass before any testnet deployment. Blocks merge.
- **P1** — Must pass before mainnet. Can be deferred from initial PR.
- **P2** — Performance and scalability. Tracked but not blocking.

**Total test cases:** 42 (including sub-cases)
**P0 (critical):** 24
**P1 (required for mainnet):** 10
**P2 (performance tracking):** 8
