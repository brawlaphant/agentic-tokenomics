# SDK v0.54 Migration Implementation Guide

> **STATUS: DRAFT — Pending SDK v0.54 Release**
>
> This guide is based on the Cosmos SDK v0.54 pre-release specification and x/poa module design documents as of March 2026. Key caveats:
>
> - **SDK v0.54 is not yet released.** Import paths, interface names, and module behavior may change before final release. All Go code blocks in this document are marked as `SPECULATIVE` where they reference unreleased APIs.
> - **x/poa is under active development** by Cosmos Labs. The module API documented here is based on PR #32's analysis and may diverge from the shipped version.
> - **Review checklist at bottom** (§ What Will Need Updating) lists every section that must be re-verified against the actual v0.54 release.
>
> Code blocks are marked with one of three labels:
> - `// CURRENT (v0.53)` — Code that works today against the existing Regen Ledger
> - `// TARGET (v0.54)` — Speculative code based on pre-release v0.54 design; verify before use
> - `// ARCHITECTURE` — Structural patterns that are SDK-version-independent

## Overview

**Purpose**: Provide concrete code migration patterns for Regen Ledger engineers, turning PR #35's checklist into implementation-ready guidance.

**Audience**: Regen Ledger core developers, CosmWasm contract developers, and integration engineers involved in the Economic Reboot implementation (M012-M015).

**Context**: This guide bridges three related PRs:
- **PR #35** — SDK v0.53 to v0.54 migration checklist (three-phase plan)
- **PR #32** — x/poa module integration analysis (gaps and recommendations)
- **PR #36** — M013 fee router as CosmWasm contract

### Current Stack
| Component | Current Version |
|-----------|----------------|
| Cosmos SDK | v0.53.4 |
| CometBFT | v0.38.19 |
| IBC-Go | v10.4.0 |
| wasmvm | v1.x |
| IAVL | v0 |

### Target Stack
| Component | Target Version |
|-----------|---------------|
| Cosmos SDK | v0.54.x |
| CometBFT | v0.39.x |
| IBC-Go | v11.x |
| wasmvm | v2.x |
| IAVL | v2 |

---

## Breaking Changes Inventory

| Area | Change | Impact on Regen | Migration Pattern |
|------|--------|-----------------|-------------------|
| **Module Manager** | `module.AppModule` replaced by `appmodule.AppModule` interface | All custom modules (x/ecocredit, x/data) must implement new interface | Implement `appmodule.AppModule`; use `appmodule.HasBeginBlocker` / `HasEndBlocker` traits |
| **Store** | IAVL v0 to IAVL v2 migration | State migration required for all modules; affects genesis export/import | Run IAVL migration tool during upgrade handler; schedule ~30min downtime |
| **Auth** | Account number sequence changes; `AccountKeeper` interface updated | x/ecocredit module accounts, fee collector accounts | Update all `AccountKeeper` references to new method signatures |
| **Staking** | x/staking deprecation path; new abstraction layer for validator set management | Direct replacement path via x/poa; existing staking queries change | Phase B swap: x/poa replaces x/staking as validator set provider |
| **Governance** | Updated proposal handling; `v1` proposals fully replace legacy `v1beta1` | All governance proposals (GOV-001 through GOV-005) must use v1 format | Remove `v1beta1` proposal registrations; update proposal submission code |
| **Bank** | `SendRestrictions` interface changes; new `WithSendRestriction()` pattern | x/ecocredit marketplace escrow, M013 fee collection | Refactor `SendRestrictions` to implement new callback signature |
| **CosmWasm** | wasmvm v2 compatibility layer; new capability negotiation | All deployed contracts (M008, M009, M011, M013) need recompilation | Recompile contracts against wasmvm v2; update `cosmwasm-std` to matching version |
| **Protobuf** | `cosmos.msg.v1.signer` annotation enforcement | All custom message types in x/ecocredit, x/data | Verify all `Msg` types have correct signer annotations |
| **ADR-028** | Address format changes in SDK modules | Module account addresses may change | Verify all hardcoded module account addresses; update genesis if needed |
| **Events** | Typed events fully replace legacy `sdk.Event` | All event emissions in custom modules | Replace `sdk.NewEvent()` calls with typed event structs |

---

## x/authority Module Migration

### Before (v0.53 Custom x/authority Concept)

The Phase 3 spec (3.1-smart-contract-specs.md) and M014 assume a custom `x/authority` module wrapping `x/staking`:

```go
// CURRENT (v0.53) — Phase 2.6 assumed architecture
// x/authority wraps x/staking to manage a curated validator set

package authority

import (
    stakingkeeper "github.com/cosmos/cosmos-sdk/x/staking/keeper"
)

type Keeper struct {
    stakingKeeper   stakingkeeper.Keeper   // delegates consensus to x/staking
    validators      collections.Map        // AuthorityValidator records
    applications    collections.Map        // ValidatorApplication records
    performance     collections.Map        // PerformanceRecord history
}

// BeginBlocker: check term expirations, enforce composition
func (k Keeper) BeginBlocker(ctx context.Context) error {
    // manually sync authority set -> staking active set
    // enforce category minimums (5 infra, 5 refi, 5 data stewards)
    // check term expirations
    return k.syncValidatorSet(ctx)
}

// Custom keeper managing the full validator lifecycle:
//   CANDIDATE -> APPROVED -> ACTIVE -> PROBATION -> REMOVED/TERM_EXPIRED
// Plus: composition enforcement, compensation splitting, performance tracking
```

**Problems with this approach**:
1. Maintaining a wrapper around x/staking is fragile across SDK upgrades
2. Dual-state management (x/authority records + x/staking state) creates consistency risks
3. All M014 logic (lifecycle, composition, compensation, performance) in one native module is high-complexity

### After (v0.54 with x/poa)

SDK v0.54 ships with native `x/poa` from Cosmos Labs. x/poa is a direct replacement for `x/staking` that provides token-free Proof of Authority consensus.

**What x/poa handles natively** (from PR #32 analysis):
- `REGISTERED -> ACTIVE -> REMOVED` validator lifecycle
- Authority-weighted governance (equal vote weight per validator)
- Checkpoint-based fee distribution to active validators
- CometBFT v0.39 validator set updates

**What x/poa does NOT handle** (gaps identified in PR #32):
- No category enforcement (infrastructure / ReFi / data steward minimums)
- No term tracking (12-month terms, re-application, expiry)
- No performance scoring (uptime, governance participation, ecosystem contribution)
- No composition ratio enforcement (min 5 per category)
- No probation state or graduated response

**Recommended architecture**: x/poa for consensus + CosmWasm contracts for application logic + AGENT-004 monitoring.

```
                     ┌───────────────────────────────────┐
                     │          x/poa (native)           │
                     │  - Validator set management       │
                     │  - Block production authority      │
                     │  - Fee distribution (base share)   │
                     │  - CometBFT v0.39 integration      │
                     └──────────────┬────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    │                               │
       ┌────────────▼────────────┐    ┌─────────────▼──────────────┐
       │  authority-registry.wasm │    │  AGENT-004 (off-chain)     │
       │  (CosmWasm contract)    │    │  - Performance monitoring   │
       │  - Category tracking     │    │  - Uptime tracking          │
       │  - Term management       │    │  - Ecosystem contribution   │
       │  - Composition checks    │    │  - Alert generation         │
       │  - Performance scores    │    │  - Probation recommendations│
       │  - Compensation splitting│    └────────────────────────────┘
       │    (base + bonus)        │
       └──────────────────────────┘
```

#### Refactored Code Pattern

```go
// TARGET (v0.54) — Verify import paths against actual release
// Before: custom keeper managing full validator set
type AuthorityKeeper struct {
    stakingKeeper  stakingkeeper.Keeper
    validators     collections.Map
    // ... all lifecycle, composition, compensation logic
}

// After: x/poa manages consensus set; CosmWasm manages application logic
// The native Go code is minimal — just the x/poa configuration:

type App struct {
    // ...
    POAKeeper     poakeeper.Keeper      // replaces StakingKeeper
    WasmKeeper    wasmkeeper.Keeper     // hosts authority-registry contract
    // ...
}

// x/poa handles the consensus-critical path:
//   - Who produces blocks
//   - How fees are distributed to validators
//   - CometBFT validator set updates

// The CosmWasm authority-registry contract handles application logic:
//   - Which validators are eligible (category, term, performance)
//   - When to propose additions/removals to x/poa
//   - Compensation splitting (base + performance bonus from M013 validator fund)
//   - Composition enforcement (min per category)

// The contract calls x/poa via governance proposals or authorized messages:
//   MsgSetPower{} — update a validator's power (add/remove from active set)
```

### Module Registration

Update `app/app.go` for SDK v0.54 with x/poa replacing x/staking:

```go
// TARGET (v0.54) — Verify import paths against actual release
// app/app.go — SDK v0.54 module registration changes

import (
    // Remove:
    // "github.com/cosmos/cosmos-sdk/x/staking"
    // stakingkeeper "github.com/cosmos/cosmos-sdk/x/staking/keeper"
    // stakingtypes "github.com/cosmos/cosmos-sdk/x/staking/types"

    // Add:
    "github.com/cosmos/cosmos-sdk/x/poa"
    poakeeper "github.com/cosmos/cosmos-sdk/x/poa/keeper"
    poatypes "github.com/cosmos/cosmos-sdk/x/poa/types"

    // SDK v0.54 module interface
    "cosmossdk.io/core/appmodule"
)

// App struct changes
type App struct {
    // ...

    // Remove:
    // StakingKeeper   stakingkeeper.Keeper

    // Add:
    POAKeeper       poakeeper.Keeper

    // Unchanged:
    AccountKeeper   authkeeper.AccountKeeper
    BankKeeper      bankkeeper.Keeper
    GovKeeper       govkeeper.Keeper
    WasmKeeper      wasmkeeper.Keeper
    EcocreditKeeper ecocreditkeeper.Keeper
    // ...
}

func NewApp(/* ... */) *App {
    // ...

    // x/poa keeper initialization
    // The admin address controls who can add/remove validators.
    // Set to the governance module address so that only governance
    // proposals can modify the validator set.
    app.POAKeeper = poakeeper.NewKeeper(
        appCodec,
        runtime.NewKVStoreService(keys[poatypes.StoreKey]),
        app.AccountKeeper,
        app.BankKeeper,
        authtypes.NewModuleAddress(govtypes.ModuleName).String(), // admin = gov
    )

    // Module manager — use new appmodule interface
    app.ModuleManager = module.NewManager(
        // ... other modules ...
        poa.NewAppModule(appCodec, app.POAKeeper),
        // Remove: staking.NewAppModule(appCodec, app.StakingKeeper, ...),
        wasm.NewAppModule(appCodec, &app.WasmKeeper, ...),
    )

    // BeginBlocker / EndBlocker ordering
    // x/poa replaces x/staking in the block lifecycle
    app.ModuleManager.SetOrderBeginBlockers(
        // ...
        poatypes.ModuleName,  // was: stakingtypes.ModuleName
        // ...
    )

    app.ModuleManager.SetOrderEndBlockers(
        // ...
        poatypes.ModuleName,  // was: stakingtypes.ModuleName
        // ...
    )

    // Genesis order
    app.ModuleManager.SetOrderInitGenesis(
        // ...
        poatypes.ModuleName,  // was: stakingtypes.ModuleName
        // ...
    )

    return app
}
```

### Genesis Migration

Migrate genesis state from x/staking to x/poa format during the upgrade handler:

```go
// TARGET (v0.54) — Genesis migration handler; verify x/poa state format against release
// app/upgrades/v054/upgrade.go

package v054

import (
    "context"

    upgradetypes "cosmossdk.io/x/upgrade/types"
    sdk "github.com/cosmos/cosmos-sdk/types"
    "github.com/cosmos/cosmos-sdk/types/module"
    poatypes "github.com/cosmos/cosmos-sdk/x/poa/types"
    stakingtypes "github.com/cosmos/cosmos-sdk/x/staking/types"
)

const UpgradeName = "v054-poa-migration"

func CreateUpgradeHandler(
    mm *module.Manager,
    configurator module.Configurator,
    poaKeeper poakeeper.Keeper,
    stakingKeeper stakingkeeper.Keeper,
) upgradetypes.UpgradeHandler {
    return func(ctx context.Context, plan upgradetypes.Plan, fromVM module.VersionMap) (module.VersionMap, error) {
        sdkCtx := sdk.UnwrapSDKContext(ctx)
        logger := sdkCtx.Logger()

        logger.Info("Starting v0.54 PoA migration")

        // Step 1: Read current active validator set from x/staking
        activeVals, err := stakingKeeper.GetBondedValidatorsByPower(ctx)
        if err != nil {
            return nil, err
        }

        // Step 2: Filter to approved authority validators
        // (pre-approved via governance before upgrade)
        approvedSet := filterApprovedAuthorities(activeVals)

        // Step 3: Initialize x/poa genesis with authority validators
        poaGenesis := &poatypes.GenesisState{
            Params: poatypes.DefaultParams(),
            Validators: make([]poatypes.Validator, 0, len(approvedSet)),
        }

        for _, val := range approvedSet {
            poaGenesis.Validators = append(poaGenesis.Validators, poatypes.Validator{
                Address:       val.OperatorAddress,
                ConsensusPubkey: val.ConsensusPubkey,
                Power:         1, // equal power in PoA
                Status:        poatypes.ValidatorStatus_ACTIVE,
            })
        }

        // Step 4: Initialize x/poa state
        if err := poaKeeper.InitGenesis(ctx, poaGenesis); err != nil {
            return nil, err
        }

        // Step 5: Run all module migrations
        newVM, err := mm.RunMigrations(ctx, configurator, fromVM)
        if err != nil {
            return nil, err
        }

        // Step 6: Disable inflation (M012 prerequisite)
        // Inflation will be replaced by M012 dynamic supply
        logger.Info("PoA migration complete",
            "authority_validators", len(approvedSet),
        )

        return newVM, nil
    }
}

// filterApprovedAuthorities returns only validators pre-approved
// for the PoA authority set. The approved list is maintained in the
// authority-registry CosmWasm contract, queried here.
func filterApprovedAuthorities(
    validators []stakingtypes.Validator,
) []stakingtypes.Validator {
    // In practice, query the authority-registry contract
    // or use a hardcoded approved set from the upgrade proposal
    approved := make([]stakingtypes.Validator, 0)
    for _, val := range validators {
        if isApprovedAuthority(val.OperatorAddress) {
            approved = append(approved, val)
        }
    }
    return approved
}
```

#### Genesis JSON Migration (export/import path)

For testnet migration via genesis export:

```json
{
  "app_state": {
    "poa": {
      "params": {
        "admin": "regen1...gov_module_address..."
      },
      "validators": [
        {
          "address": "regenvaloper1...",
          "consensus_pubkey": { "@type": "/cosmos.crypto.ed25519.PubKey", "key": "..." },
          "power": "1",
          "status": "ACTIVE"
        }
      ]
    }
  }
}
```

Remove the `staking` key from genesis and replace with `poa`. The x/staking state (delegations, unbonding entries, redelegations) is not migrated -- delegators must unbond during Phase B transition window.

---

## x/feerouter Module Migration

M013 (Value-Based Fee Routing) has two possible implementation paths. PR #36 specifies the CosmWasm contract approach; the Phase 3 spec originally assumed a native module. Both are documented below.

### CosmWasm Contract Approach (PR #36 / Recommended)

PR #36 specifies M013 as a CosmWasm contract (`fee-router.wasm`). This is the recommended approach because:
1. Upgradeable without chain upgrades (governance proposal to migrate contract)
2. Fee schedule changes via contract execution, not parameter change proposals
3. Simpler to audit and test independently

#### Contract Integration with x/ecocredit Hooks

In SDK v0.54, x/ecocredit message handlers emit hooks that the fee-router contract consumes:

```rust
// contracts/fee-router/src/contract.rs

use cosmwasm_std::{
    entry_point, DepsMut, Env, MessageInfo, Response, Uint128,
    BankMsg, CosmosMsg, SubMsg, WasmMsg,
};

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Called by x/ecocredit hooks when a credit transaction occurs
        ExecuteMsg::CollectFee {
            tx_type,
            credit_value,
            payer,
        } => execute_collect_fee(deps, env, info, tx_type, credit_value, payer),

        // Distribute collected fees to pools
        ExecuteMsg::DistributeFees {} => execute_distribute_fees(deps, env, info),

        // Governance: update fee schedule
        ExecuteMsg::UpdateFeeSchedule { schedule } => {
            execute_update_schedule(deps, env, info, schedule)
        }
    }
}

fn execute_collect_fee(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tx_type: TransactionType,
    credit_value: Uint128,
    payer: String,
) -> Result<Response, ContractError> {
    // Only authorized callers (x/ecocredit hook or governance)
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.ecocredit_hook_address
        && info.sender != config.governance_address
    {
        return Err(ContractError::Unauthorized {});
    }

    // Calculate fee based on transaction type
    let fee_rate = match tx_type {
        TransactionType::Issuance => config.issuance_rate,   // 1-3%
        TransactionType::Transfer => config.transfer_rate,   // 0.1%
        TransactionType::Retirement => config.retirement_rate, // 0.5%
        TransactionType::Trade => config.trade_rate,         // 1%
    };

    let fee_amount = credit_value * fee_rate / Uint128::new(10_000); // basis points
    let fee_amount = fee_amount.max(config.min_fee); // floor at min_fee

    // Distribute to pools per M013 spec
    let burn_amount = fee_amount * config.burn_share / Uint128::new(10_000);
    let validator_amount = fee_amount * config.validator_share / Uint128::new(10_000);
    let community_amount = fee_amount * config.community_share / Uint128::new(10_000);
    let agent_amount = fee_amount - burn_amount - validator_amount - community_amount;

    // Build distribution messages
    let mut msgs: Vec<CosmosMsg> = vec![];

    // Burn pool -> send to burn address (M012)
    if !burn_amount.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Burn {
            amount: vec![coin(burn_amount.u128(), &config.fee_denom)],
        }));
    }

    // Validator fund -> send to x/poa fee distribution
    if !validator_amount.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.validator_fund_address.clone(),
            amount: vec![coin(validator_amount.u128(), &config.fee_denom)],
        }));
    }

    // Community pool -> send to x/distribution community pool
    if !community_amount.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.community_pool_address.clone(),
            amount: vec![coin(community_amount.u128(), &config.fee_denom)],
        }));
    }

    // Agent infra fund
    if !agent_amount.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.agent_fund_address.clone(),
            amount: vec![coin(agent_amount.u128(), &config.fee_denom)],
        }));
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "collect_fee")
        .add_attribute("tx_type", tx_type.to_string())
        .add_attribute("fee_amount", fee_amount.to_string())
        .add_attribute("payer", payer))
}
```

#### Registering the Fee Router as an Authorized Hook

In SDK v0.54, x/ecocredit exposes a hook registration mechanism. The fee-router contract must be registered as an authorized post-execution hook:

```go
// app/app.go — register fee-router contract as ecocredit hook

func (app *App) registerFeeRouterHook(ctx context.Context) error {
    // The fee-router contract address is set via governance proposal
    // after initial deployment. This is called in the upgrade handler.

    feeRouterAddr := app.getParamOrDefault(
        "fee_router_contract",
        "", // empty until deployed
    )

    if feeRouterAddr == "" {
        return nil // fee router not yet deployed
    }

    // Register the contract as a post-execution hook
    // for credit transactions
    app.EcocreditKeeper.SetPostExecutionHook(
        ctx,
        ecocredittypes.HookConfig{
            ContractAddress: feeRouterAddr,
            MessageTypes: []string{
                "/regen.ecocredit.v1.MsgCreateBatch",
                "/regen.ecocredit.v1.MsgSend",
                "/regen.ecocredit.v1.MsgRetire",
                "/regen.ecocredit.marketplace.v1.MsgBuyDirect",
            },
        },
    )

    return nil
}
```

#### BeginBlocker/EndBlocker Hook Patterns for Fee Collection

For periodic fee distribution (rather than per-transaction), use a CosmWasm Sudo entry point triggered by the chain's EndBlocker:

```go
// app/app.go — EndBlocker calls fee-router for periodic distribution

func (app *App) EndBlocker(ctx context.Context) error {
    sdkCtx := sdk.UnwrapSDKContext(ctx)

    // Trigger fee distribution every N blocks (e.g., every epoch)
    if sdkCtx.BlockHeight() % app.FeeDistributionInterval == 0 {
        feeRouterAddr := app.getFeeRouterAddress(ctx)
        if feeRouterAddr != "" {
            sudoMsg := []byte(`{"distribute_fees":{}}`)
            _, err := app.WasmKeeper.Sudo(ctx, feeRouterAddr, sudoMsg)
            if err != nil {
                sdkCtx.Logger().Error("fee distribution failed", "err", err)
                // Non-fatal: fees accumulate until next successful distribution
            }
        }
    }

    return nil
}
```

### Native Module Approach (Phase 3 Spec Fallback)

If keeping M013 as a native `x/feerouter` module, the following SDK v0.54 migration patterns apply:

#### Message Routing Changes

```go
// x/feerouter/module.go — SDK v0.54 appmodule interface

package feerouter

import (
    "cosmossdk.io/core/appmodule"
    "github.com/cosmos/cosmos-sdk/codec"
    "github.com/cosmos/cosmos-sdk/types/module"
)

// Verify interface compliance at compile time
var (
    _ appmodule.AppModule       = AppModule{}
    _ appmodule.HasBeginBlocker = AppModule{}
    _ appmodule.HasEndBlocker   = AppModule{}
    _ module.HasGenesis         = AppModule{}
)

type AppModule struct {
    cdc    codec.Codec
    keeper keeper.Keeper
}

// SDK v0.54: implement appmodule.HasEndBlocker instead of module.EndBlockModule
func (am AppModule) EndBlock(ctx context.Context) error {
    return am.keeper.EndBlocker(ctx)
}

// SDK v0.54: implement appmodule.HasBeginBlocker
func (am AppModule) BeginBlock(ctx context.Context) error {
    return am.keeper.BeginBlocker(ctx)
}

// IsOnePerModuleType and IsAppModule are required by SDK v0.54
func (AppModule) IsOnePerModuleType() {}
func (AppModule) IsAppModule()        {}
```

#### Keeper Interface Updates

```go
// x/feerouter/keeper/keeper.go — SDK v0.54 keeper pattern

package keeper

import (
    "context"

    "cosmossdk.io/collections"
    storetypes "cosmossdk.io/store/types"
    "github.com/cosmos/cosmos-sdk/codec"
    sdk "github.com/cosmos/cosmos-sdk/types"
)

type Keeper struct {
    cdc           codec.Codec
    storeService  storetypes.KVStoreService  // SDK v0.54: KVStoreService replaces StoreKey

    // Collections API (SDK v0.54 preferred over raw KV)
    FeeConfig     collections.Item[types.FeeConfig]
    FeeRecords    collections.Map[uint64, types.FeeRecord]
    PoolBalances  collections.Map[string, sdk.Coins]

    // Dependent keepers
    bankKeeper    types.BankKeeper
    accountKeeper types.AccountKeeper
    poaKeeper     types.POAKeeper   // was: stakingKeeper
}

func NewKeeper(
    cdc codec.Codec,
    storeService storetypes.KVStoreService,  // not StoreKey
    bankKeeper types.BankKeeper,
    accountKeeper types.AccountKeeper,
    poaKeeper types.POAKeeper,
) Keeper {
    sb := collections.NewSchemaBuilder(storeService)

    return Keeper{
        cdc:          cdc,
        storeService: storeService,
        FeeConfig:    collections.NewItem(sb, types.FeeConfigKey, "fee_config", codec.CollValue[types.FeeConfig](cdc)),
        FeeRecords:   collections.NewMap(sb, types.FeeRecordsPrefix, "fee_records", collections.Uint64Key, codec.CollValue[types.FeeRecord](cdc)),
        PoolBalances: collections.NewMap(sb, types.PoolBalancesPrefix, "pool_balances", collections.StringKey, codec.CollValue[sdk.Coins](cdc)),
        bankKeeper:   bankKeeper,
        accountKeeper: accountKeeper,
        poaKeeper:    poaKeeper,
    }
}
```

#### Module Account Registration

```go
// app/app.go — module account registration for x/feerouter

// Module accounts that can hold tokens
maccPerms := map[string][]string{
    // ... existing module accounts ...
    feeroutertypes.ModuleName:         {authtypes.Burner, authtypes.Minter},
    feeroutertypes.BurnPoolName:       {authtypes.Burner},
    feeroutertypes.ValidatorFundName:  nil,
    feeroutertypes.CommunityPoolName:  nil,
    feeroutertypes.AgentFundName:      nil,
}
```

---

## Test Patterns

### Unit Test Migration

#### Before (SDK v0.53)

```go
// Old pattern: github.com/cosmos/cosmos-sdk/testutil
package keeper_test

import (
    "testing"

    "github.com/cosmos/cosmos-sdk/testutil"
    sdk "github.com/cosmos/cosmos-sdk/types"
    "github.com/stretchr/testify/require"
)

func TestFeeCollection(t *testing.T) {
    ctx, keeper := setupTestKeeper(t) // custom test setup

    // Old pattern: manual context creation
    ctx = ctx.WithBlockHeight(100)

    err := keeper.CollectFee(ctx, /* ... */)
    require.NoError(t, err)
}
```

#### After (SDK v0.54)

```go
// New pattern: testutil/sims and simapp builder
package keeper_test

import (
    "testing"

    "cosmossdk.io/log"
    storetypes "cosmossdk.io/store/types"
    "github.com/cosmos/cosmos-sdk/testutil"
    simtestutil "github.com/cosmos/cosmos-sdk/testutil/sims"
    sdk "github.com/cosmos/cosmos-sdk/types"
    "github.com/stretchr/testify/require"
)

func TestFeeCollection(t *testing.T) {
    // SDK v0.54: use integration test builder
    key := storetypes.NewKVStoreKey("feerouter")
    testCtx := testutil.DefaultContextWithDB(t, key, storetypes.NewTransientStoreKey("transient_test"))
    ctx := testCtx.Ctx

    // Use mock keepers from simtestutil
    encCfg := simtestutil.MakeTestEncodingConfig()

    // Create keeper with mock dependencies
    keeper := feerouterkeeper.NewKeeper(
        encCfg.Codec,
        runtime.NewKVStoreService(key),
        mockBankKeeper{},
        mockAccountKeeper{},
        mockPOAKeeper{},    // was: mockStakingKeeper
    )

    err := keeper.CollectFee(ctx, /* ... */)
    require.NoError(t, err)
}
```

### Integration Test Patterns

#### Using simapp.New() with x/poa

```go
// integration_test.go — full app test with x/poa

package app_test

import (
    "testing"

    "cosmossdk.io/log"
    dbm "github.com/cosmos/cosmos-db"
    simtestutil "github.com/cosmos/cosmos-sdk/testutil/sims"
    "github.com/stretchr/testify/suite"
)

type IntegrationTestSuite struct {
    suite.Suite
    app *app.RegenApp
    ctx sdk.Context
}

func (s *IntegrationTestSuite) SetupTest() {
    // Create test app with x/poa instead of x/staking
    s.app = app.NewRegenApp(
        log.NewNopLogger(),
        dbm.NewMemDB(),
        nil,
        true,
        simtestutil.NewAppOptionsWithFlagHome(s.T().TempDir()),
    )

    // Initialize with PoA genesis (not staking genesis)
    poaGenesis := poatypes.GenesisState{
        Params: poatypes.DefaultParams(),
        Validators: []poatypes.Validator{
            {
                Address:         "regenvaloper1test1...",
                ConsensusPubkey: testPubKey1,
                Power:           1,
                Status:          poatypes.ValidatorStatus_ACTIVE,
            },
            {
                Address:         "regenvaloper1test2...",
                ConsensusPubkey: testPubKey2,
                Power:           1,
                Status:          poatypes.ValidatorStatus_ACTIVE,
            },
        },
    }

    s.ctx = s.app.BaseApp.NewContext(false)
}

func (s *IntegrationTestSuite) TestAuthorityValidatorAddition() {
    // Simulate governance proposal to add a validator via x/poa
    msg := &poatypes.MsgSetPower{
        Sender:           govModuleAddr,
        ValidatorAddress: "regenvaloper1new...",
        Power:            1,
        Unsafe:           false,
    }

    _, err := s.app.POAKeeper.SetPower(s.ctx, msg)
    s.Require().NoError(err)

    // Verify validator is in active set
    val, err := s.app.POAKeeper.GetValidator(s.ctx, "regenvaloper1new...")
    s.Require().NoError(err)
    s.Require().Equal(poatypes.ValidatorStatus_ACTIVE, val.Status)
}

func TestIntegration(t *testing.T) {
    suite.Run(t, new(IntegrationTestSuite))
}
```

#### Mock Validator Set for Testing x/authority Application Logic

```go
// testutil/mock_poa.go — mock x/poa keeper for authority-registry contract tests

type MockPOAKeeper struct {
    validators map[string]poatypes.Validator
}

func NewMockPOAKeeper() *MockPOAKeeper {
    return &MockPOAKeeper{
        validators: make(map[string]poatypes.Validator),
    }
}

func (m *MockPOAKeeper) GetValidator(ctx context.Context, addr string) (poatypes.Validator, error) {
    val, ok := m.validators[addr]
    if !ok {
        return poatypes.Validator{}, poatypes.ErrValidatorNotFound
    }
    return val, nil
}

func (m *MockPOAKeeper) GetActiveValidators(ctx context.Context) ([]poatypes.Validator, error) {
    var active []poatypes.Validator
    for _, v := range m.validators {
        if v.Status == poatypes.ValidatorStatus_ACTIVE {
            active = append(active, v)
        }
    }
    return active, nil
}

func (m *MockPOAKeeper) SetTestValidators(vals ...poatypes.Validator) {
    for _, v := range vals {
        m.validators[v.Address] = v
    }
}
```

### E2E Test Patterns

#### Updated Chain Initialization for x/poa

```go
// e2e/chain_setup_test.go

func setupChain(t *testing.T) *network.Network {
    cfg := network.DefaultConfig()

    // Override genesis with x/poa instead of x/staking
    poaGenState := poatypes.DefaultGenesisState()
    poaGenState.Validators = testAuthorities()  // 3 test validators

    cfg.GenesisState[poatypes.ModuleName] = cfg.Codec.MustMarshalJSON(poaGenState)

    // Remove staking genesis (x/poa replaces it)
    delete(cfg.GenesisState, stakingtypes.ModuleName)

    // Adjust validator count for test network
    cfg.NumValidators = 3

    return network.New(t, cfg)
}
```

#### IBC Testing with CometBFT v0.39

```go
// e2e/ibc_test.go

func TestIBCTransferAfterMigration(t *testing.T) {
    // CometBFT v0.39 test node configuration
    // The ibctesting package is updated for IBC-Go v11

    coordinator := ibctesting.NewCoordinator(t, 2)

    // Chain A: Regen with x/poa
    chainA := coordinator.GetChain(ibctesting.GetChainID(1))

    // Chain B: Standard Cosmos chain (counterparty)
    chainB := coordinator.GetChain(ibctesting.GetChainID(2))

    // Create IBC path
    path := ibctesting.NewPath(chainA, chainB)
    path.EndpointA.ChannelConfig.PortID = ibctesting.TransferPort
    path.EndpointB.ChannelConfig.PortID = ibctesting.TransferPort
    coordinator.Setup(path)

    // Verify IBC transfer works with PoA consensus
    transferMsg := transfertypes.NewMsgTransfer(
        path.EndpointA.ChannelConfig.PortID,
        path.EndpointA.ChannelID,
        sdk.NewCoin("uregen", sdkmath.NewInt(1000000)),
        chainA.SenderAccount.GetAddress().String(),
        chainB.SenderAccount.GetAddress().String(),
        clienttypes.NewHeight(1, 110),
        0,
        "",
    )

    _, err := chainA.SendMsgs(transferMsg)
    require.NoError(t, err)

    // Relay packets
    coordinator.RelayAndAckPendingPackets(path)

    // Verify receipt on chain B
    // ...
}
```

---

## Migration Execution Plan

### Phase A: SDK Upgrade (Estimated: 4-6 weeks)

SDK v0.53.4 to v0.54 core upgrade. No consensus model changes yet.

#### Step 1: Update go.mod Dependencies

```bash
# go.mod changes
go get github.com/cosmos/cosmos-sdk@v0.54.0
go get github.com/cometbft/cometbft@v0.39.0
go get github.com/cosmos/ibc-go/v11@latest
go get github.com/CosmWasm/wasmd@v0.54.0   # matching SDK version
go get cosmossdk.io/store@v2.0.0            # IAVL v2
```

```
// go.mod target state
require (
    github.com/cosmos/cosmos-sdk    v0.54.0
    github.com/cometbft/cometbft    v0.39.0
    github.com/cosmos/ibc-go/v11    v11.0.0
    github.com/CosmWasm/wasmd       v0.54.0
    cosmossdk.io/store              v2.0.0
    cosmossdk.io/core               v1.0.0
)
```

#### Step 2: Resolve Breaking API Changes

Address each item in the Breaking Changes Inventory above. Priority order:

1. **Module interface migration** — Update all `module.AppModule` implementations to `appmodule.AppModule`
2. **Store migration** — Update all direct store access to use `KVStoreService`
3. **Protobuf annotations** — Verify `cosmos.msg.v1.signer` on all message types
4. **Event migration** — Replace `sdk.NewEvent()` with typed events
5. **Keeper signatures** — Update all keeper constructors to accept `KVStoreService`
6. **Bank SendRestrictions** — Refactor to new callback pattern

```bash
# Useful command to find all occurrences needing migration
# Run from regen-ledger repo root:
grep -rn "module.AppModule" x/ --include="*.go"
grep -rn "sdk.StoreKey" x/ --include="*.go"
grep -rn "sdk.NewEvent" x/ --include="*.go"
```

#### Step 3: Update Module Registration

Update `app/app.go` as shown in the Module Registration section above. Keep x/staking during Phase A (replaced in Phase B).

#### Step 4: Run Existing Test Suite, Fix Failures

```bash
# Run unit tests first
go test ./x/ecocredit/... -count=1 -v
go test ./x/data/... -count=1 -v

# Then integration tests
go test ./app/... -count=1 -v

# Then simulation tests (long-running)
go test ./app/... -run TestFullAppSimulation -count=1 -v -timeout 30m
```

Expected failure categories:
- Import path changes (mechanical fixes)
- Interface compliance errors (add new required methods)
- Store access pattern changes (switch to `KVStoreService`)
- Test helper changes (`simapp` builder pattern)

#### Step 5: Testnet Deployment

1. Deploy to internal devnet with current validator set (still x/staking)
2. Run full regression: IBC transfers, credit issuance, marketplace trades
3. Verify x/ecocredit and x/data modules function correctly
4. Load test CometBFT v0.39 block production
5. Confirm wasmvm v2 contract execution

### Phase B: Consensus Migration (Estimated: 6-8 weeks)

Transition from x/staking (PoS) to x/poa (PoA).

#### Step 1: Deploy x/poa Alongside x/staking

During the transition window, both modules coexist. x/poa is initialized but not yet controlling the validator set.

```go
// Transition: both modules present, x/staking still active
app.ModuleManager = module.NewManager(
    staking.NewAppModule(appCodec, app.StakingKeeper, ...),  // still active
    poa.NewAppModule(appCodec, app.POAKeeper),                // initialized, not yet active
    // ...
)
```

Deploy the `authority-registry.wasm` CosmWasm contract with the initial approved validator list.

#### Step 2: Governance Proposal to Activate x/poa

Submit a software upgrade proposal that:
1. Runs the genesis migration handler (see Genesis Migration section)
2. Transfers validator set control from x/staking to x/poa
3. Sets x/poa admin to the governance module address
4. Begins the mandatory unbonding communication period

```bash
# Submit upgrade proposal
regen tx gov submit-proposal software-upgrade v054-poa-migration \
    --title "Activate PoA Consensus (M014)" \
    --summary "Migrate validator set management from x/staking to x/poa. See PR #35 for full migration plan." \
    --upgrade-height <target_height> \
    --deposit 10000000uregen \
    --from <proposer> \
    --chain-id regen-1
```

#### Step 3: Graceful x/staking Wind-Down

After x/poa activation:
1. Disable new delegations to x/staking
2. Begin mandatory 21-day unbonding period for all delegators
3. Communicate unbonding timeline via governance forums and validator channels
4. Monitor unbonding progress via AGENT-004

```
Timeline:
  Day 0:   PoA activated; new delegations disabled
  Day 1-7: Communication period; delegators begin unbonding
  Day 7-28: Unbonding period (21-day Cosmos standard)
  Day 28:  All delegations unbonded; x/staking can be removed
  Day 35:  Verify no remaining staking state; clean removal
```

#### Step 4: IBC Channel Verification

After consensus migration, verify all IBC channels remain operational:

```bash
# Verify IBC channels
regen q ibc channel channels --output json | jq '.channels[] | {channel_id, state, counterparty}'

# Test IBC transfer on each active channel
for channel in $(regen q ibc channel channels --output json | jq -r '.channels[].channel_id'); do
    echo "Testing channel: $channel"
    regen tx ibc-transfer transfer transfer $channel \
        <counterparty_addr> 1uregen \
        --from test-account \
        --chain-id regen-1 \
        --packet-timeout-height 0-0 \
        --packet-timeout-timestamp 600000000000
done
```

Verify client updates propagate correctly with CometBFT v0.39 light client proofs.

### Phase C: Economic Integration (Estimated: 8-12 weeks)

Deploy the Economic Reboot mechanisms (M012-M015) on top of the PoA foundation.

#### Step 1: Deploy M013 CosmWasm Contract (Fee Router)

```bash
# Store the fee-router contract
regen tx wasm store fee_router.wasm \
    --from deployer \
    --gas auto \
    --chain-id regen-1

# Instantiate with initial configuration
regen tx wasm instantiate <code_id> '{
    "admin": "regen1...gov_module...",
    "fee_denom": "uregen",
    "issuance_rate": 200,
    "transfer_rate": 10,
    "retirement_rate": 50,
    "trade_rate": 100,
    "burn_share": 3000,
    "validator_share": 4000,
    "community_share": 2500,
    "agent_share": 500,
    "min_fee": "1000000",
    "validator_fund_address": "regen1...poa_fee_collector...",
    "community_pool_address": "regen1...distribution...",
    "agent_fund_address": "regen1...agent_fund...",
    "ecocredit_hook_address": "regen1...ecocredit_module..."
}' \
    --label "fee-router-v1" \
    --admin "regen1...gov_module..." \
    --from deployer \
    --chain-id regen-1
```

#### Step 2: Activate Fee Routing

Register the fee-router contract as an x/ecocredit hook via governance:

```bash
regen tx gov submit-proposal execute-contract <fee_router_addr> '{
    "register_ecocredit_hook": {}
}' \
    --title "Activate M013 Fee Routing" \
    --summary "Register fee-router contract as post-execution hook for credit transactions." \
    --deposit 10000000uregen \
    --from proposer \
    --chain-id regen-1
```

#### Step 3: Deploy M012 Supply Module Changes

M012 (Fixed Cap Dynamic Supply) requires a native module upgrade to replace x/mint:

```bash
# Software upgrade proposal for x/supply module
regen tx gov submit-proposal software-upgrade v054-dynamic-supply \
    --title "Activate M012 Dynamic Supply" \
    --summary "Replace inflationary x/mint with M012 fixed-cap dynamic supply. Hard cap: 221M REGEN." \
    --upgrade-height <target_height> \
    --deposit 10000000uregen \
    --from proposer \
    --chain-id regen-1
```

#### Step 4: Activate M015 Rewards

Deploy the contribution-weighted rewards contract and connect it to the community pool:

```bash
# Store and instantiate rewards contract
regen tx wasm store contribution_rewards.wasm --from deployer --gas auto --chain-id regen-1

regen tx wasm instantiate <code_id> '{
    "admin": "regen1...gov_module...",
    "community_pool_source": "regen1...distribution...",
    "activity_weights": {
        "credit_purchase": 300,
        "credit_retirement": 200,
        "platform_facilitation": 300,
        "governance_participation": 200
    },
    "distribution_period": 604800
}' \
    --label "contribution-rewards-v1" \
    --admin "regen1...gov_module..." \
    --from deployer \
    --chain-id regen-1
```

---

## Compatibility Matrix

| Component | v0.53 Version | v0.54 Target | Breaking Changes | Status |
|-----------|--------------|-------------|------------------|--------|
| Cosmos SDK | v0.53.4 | v0.54.x | Module interface, store, events | Phase A |
| CometBFT | v0.38.19 | v0.39.x | Light client proofs, ABCI++ updates | Phase A |
| IBC-Go | v10.4.0 | v11.x | Channel upgrade protocol, client changes | Phase A |
| x/staking | v0.53.4 (active) | Removed | Replaced by x/poa | Phase B |
| x/poa | N/A | v0.54.x (native) | New module | Phase B |
| wasmvm | v1.x | v2.x | Contract recompilation required | Phase A |
| IAVL | v0 | v2 | State migration required | Phase A |
| x/ecocredit | v3.0 | v3.1 (hooks) | Post-execution hook API | Phase C |
| x/mint | Active | Removed | Replaced by M012 x/supply | Phase C |
| x/distribution | Active | Modified | Community pool fed by M013 | Phase C |
| x/gov | v1beta1 + v1 | v1 only | Legacy proposal removal | Phase A |
| authority-registry | N/A | CosmWasm v1 | New contract | Phase B |
| fee-router | N/A | CosmWasm v1 | New contract (M013) | Phase C |
| contribution-rewards | N/A | CosmWasm v1 | New contract (M015) | Phase C |

---

## Dependency Graph

Migration phases must respect these dependency ordering constraints:

```
Phase A (SDK Upgrade)
  ├── go.mod dependency updates
  ├── Module interface migration (appmodule.AppModule)
  ├── Store migration (IAVL v2)
  ├── wasmvm v2 recompilation
  ├── IBC-Go v11 compatibility
  └── Testnet validation
        │
        ▼
Phase B (Consensus Migration)
  ├── x/poa deployment (depends on: Phase A complete)
  ├── authority-registry.wasm deployment (depends on: wasmvm v2)
  ├── Governance proposal: activate x/poa
  ├── x/staking wind-down (21-day unbonding)
  ├── IBC channel verification
  └── Testnet validation
        │
        ▼
Phase C (Economic Integration)
  ├── M013 fee-router.wasm deployment (depends on: x/poa active)
  │     └── Requires: x/ecocredit hook registration
  ├── M012 x/supply activation (depends on: M013 active, x/mint disabled)
  │     └── Requires: fee revenue flowing to burn pool
  ├── M015 contribution-rewards.wasm (depends on: M013 community pool inflow)
  │     └── Requires: community pool funded by fee routing
  └── Mainnet validation
```

---

## Summary

This guide translates PR #35's three-phase migration checklist into implementation-ready patterns for Regen Ledger engineers. The key architectural decisions are:

1. **x/poa replaces x/staking** for consensus-level validator management, eliminating the need for a custom x/authority native module wrapper.

2. **Application logic moves to CosmWasm** -- the authority-registry contract handles category enforcement, term tracking, performance scoring, and compensation splitting that x/poa does not provide natively (gaps identified in PR #32).

3. **M013 fee routing is a CosmWasm contract** (per PR #36), integrated via x/ecocredit post-execution hooks rather than a native x/feerouter module.

4. **Migration is strictly ordered**: Phase A (SDK upgrade) must complete before Phase B (consensus change), which must complete before Phase C (economic mechanisms). Within each phase, the dependency graph above specifies the required ordering.

5. **AGENT-004 monitoring** bridges the gap between on-chain x/poa state and the off-chain performance tracking needed for the full M014 validator governance model.

### Related Documents
- [Economic Reboot Mechanism Specifications](../phase-2/2.6-economic-reboot-mechanisms.md) — M012, M013, M014, M015 protocol specs
- [Smart Contract Development Specifications](../phase-3/3.1-smart-contract-specs.md) — Contract architecture and protobuf definitions
- [Tokenomic Mechanisms Inventory](../phase-1/1.2-tokenomic-mechanisms.md) — Full mechanism catalog with PoA status annotations
- [Token Economics Synthesis](economics/token-economics-synthesis.md) — Economic model and parameter analysis

---

## What Will Need Updating When SDK v0.54 Ships

> **Action required**: When Cosmos SDK v0.54 reaches release candidate, an engineer must walk this checklist and verify each section against the actual release.

| Section | What to Verify | Likely Change Risk |
|---------|---------------|-------------------|
| **Breaking Changes Inventory** (§2) | Confirm all 10 breaking changes still apply; check for additional ones in the v0.54 CHANGELOG | Medium — there will be late-breaking changes |
| **x/poa import paths** (§3.2, §3.3) | Verify `github.com/cosmos/cosmos-sdk/x/poa` is the actual import path; may be `cosmossdk.io/x/poa` | High — module location may differ |
| **Module registration** (§3.3) | Verify `appmodule.AppModule` interface matches actual shipped interface | High — the interface changed between SDK v0.50 and v0.52; may change again |
| **x/poa MsgSetPower** (§3.2) | Verify the message name and fields; x/poa is pre-release | High — API not stable |
| **Genesis migration handler** (§3.4) | Verify x/staking state structure matches v0.53 export format | Medium — may need field-level adjustments |
| **CosmWasm wasmvm v2** (§4) | Verify contract compatibility; test recompilation against actual wasmvm v2 | Low — CosmWasm maintains backward compat |
| **Test utilities** (§5) | Verify `testutil` package paths and function signatures | Medium — test utils change frequently |
| **CLI commands** (§6) | Verify all `regen tx` and `regen query` commands still work | Low — CLI is relatively stable |
| **IAVL migration tool** (§2) | Verify tool exists at `cosmossdk.io/store/v2` and document expected downtime | Medium — tool may have different CLI flags |

**Recommended process**: When v0.54 RC1 drops, create a branch `test/sdk-v054-verification`, attempt to build regen-ledger against it, and update this document with findings. Track verification status per row.
