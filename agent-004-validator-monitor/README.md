# AGENT-004: Regen Validator Monitor

**Layer 1 (fully automated, read-only, informational) agent that watches the Regen Network validator set, scores per-validator performance against the M014 methodology, tracks delegation flows, and monitors network decentralization.**

Mirrors the AGENT-002 Governance Analyst / AGENT-003 Market Monitor structure: the same OODA executor, the same standalone Node.js process shape, the same SQLite-backed local state. Scope is validator infrastructure intelligence rather than governance or marketplace.

## What it does

| Workflow | Trigger | Output |
|----------|---------|--------|
| **WF-VM-01** Performance Tracking | Periodic (each poll cycle) | Per-validator scorecards, composite 0-1000, PoA eligibility, performance alerts |
| **WF-VM-02** Delegation Flow Analysis | Periodic (delta against previous snapshot) | Net inflow/outflow, whale detection, top movers |
| **WF-VM-03** Decentralization Monitoring | Periodic (each poll cycle) | Nakamoto coefficient, Gini index, top-N share, health tier, trend vs previous |

Each workflow is an **OODA loop** (Observe → Orient → Decide → Act). Numeric decisions (scoring, Nakamoto, Gini, health tier, whale detection) are computed **deterministically**; Claude is only used for the narrative layer.

## Architecture

```
Regen Ledger (LCD REST API)
    ↓ observe
AGENT-004 (OODA engine)
    ↓ orient (deterministic)
    ↓ decide (deterministic)
Local SQLite (state)
    ↓ act (narrative via Claude)
Console / Discord webhook
```

**No MCP dependency.** Talks directly to any Cosmos LCD endpoint. When Ledger MCP becomes available, the `LedgerClient` can be swapped behind the same interface.

**No ElizaOS dependency.** Standalone Node.js process. The ElizaOS character for AGENT-004 lives at `agents/packages/agents/src/characters/validator-monitor.ts`; this package shares the same system prompt text and the same threshold constants so downstream tooling has a single source of truth.

## Quick start

```bash
# 1. Install
cd agent-004-validator-monitor
npm install

# 2. Configure
cp .env.example .env
# Edit .env — at minimum set ANTHROPIC_API_KEY

# 3. Run (single cycle)
npm run analyze

# 4. Run (continuous polling)
npm start

# 5. Run (dev mode with auto-reload)
npm run dev
```

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ANTHROPIC_API_KEY` | Yes | — | Claude API key |
| `REGEN_LCD_URL` | No | `https://regen.api.chandrastation.com` | Cosmos LCD endpoint |
| `DISCORD_WEBHOOK_URL` | No | — | Discord webhook for posting reports |
| `POLL_INTERVAL_SECONDS` | No | `900` (15 min) | Polling interval (seconds) |
| `ANTHROPIC_MODEL` | No | `claude-sonnet-4-5-20250929` | Claude model to use |

Thresholds live in `src/config.ts` under `validator.*` and mirror the character definition at `agents/packages/agents/src/characters/validator-monitor.ts`.

## How it maps to the framework specs

| Framework Spec | Implementation |
|----------------|---------------|
| Phase 2.2 WF-VM-01 | `src/workflows/performance-tracking.ts` |
| Phase 2.2 WF-VM-02 | `src/workflows/delegation-flow-analysis.ts` |
| Phase 2.2 WF-VM-03 | `src/workflows/decentralization-monitor.ts` |
| Phase 2.4 OODA executor | `src/ooda.ts` |
| Phase 2.4 Agent character | `agents/packages/agents/src/characters/validator-monitor.ts` |
| Phase 2.5 Workflow executions table | `src/store.ts` (SQLite) |
| Phase 3.2 Ledger MCP client | `src/ledger.ts` (direct LCD) |
| M014 scoring methodology | `src/workflows/performance-tracking.ts` |

## Scoring methodology (M014)

Composite score is **0-1000**, computed deterministically in `performance-tracking.ts`:

| Component | Weight | Source |
|---|---|---|
| Uptime | 400 | `signed_blocks_window − missed_blocks_counter` from slashing signing info |
| Governance participation | 350 | Votes cast over recent finalized proposals (MVP keeps this at 0; see below) |
| Stability | 250 | 250 − (jailings × 100) − (commission changes in window × 40), floored at 0 |

**PoA eligibility:** composite >= `config.validator.poaEligibilityScore` (default 800).

## Design decisions

1. **Deterministic numbers, narrative-only LLM calls.** All scoring, Nakamoto and Gini computation, health classification, and whale detection happen locally in plain TypeScript. Claude is only invoked to write the report. Keeps the agent cheap, reproducible, and auditable.

2. **Real MsgDelegate / MsgUndelegate / MsgBeginRedelegate tx-stream.** WF-VM-02 reads recent staking events from the Cosmos LCD `tx-search` endpoint, filtered by message type URL, and parses them into `DelegationEvent` records. Each tx can carry multiple events (batched operations); the parser walks `logs[].events[]` and the flattened `tx.events[]` for cross-SDK compatibility. The aggregator groups events per-validator: delegate → inflow, undelegate → outflow, redelegate → source outflow + destination inflow. Earlier drafts used a token-delta proxy against `validator.tokens` snapshots; the current implementation produces events with delegator identity and bit-exact amounts rather than synthesized deltas.

3. **Real governance participation scoring via bech32 op→delegator conversion.** The observe phase decodes each validator's operator bech32 (`regenvaloper1...`), strips the `valoper` suffix from the HRP to produce the delegator bech32 (`regen1...`), and then queries `/proposals/{id}/votes/{voter}` once per validator per recent finalized proposal. The result is a per-validator vote count that maps directly into the governance component of the composite score. The operator→delegator conversion is a pure function (`operatorToAccountBech32`) that returns `null` on any invalid input — failure modes include non-bech32 strings, HRPs that don't end in `valoper`, and bad checksums. A null result means the validator gets a 0 governance score rather than crashing the cycle.

4. **Nakamoto coefficient uses the 33.4% halt-threshold convention.** Defined as the smallest number of validators whose cumulative voting power strictly exceeds 33.4% of total bonded stake — matches the standard Cosmos-era definition.

5. **Gini index uses the textbook formula** operating on ascending-sorted token amounts, normalized to uregen. 0 = perfect equality, 1 = perfect inequality.

6. **Standalone over ElizaOS.** Matches AGENT-002 and AGENT-003. ElizaOS plugin API may change; a standalone process proves the workflow logic works independently of any runtime framework.

## Governance layer

This agent operates at **Layer 1 only**:

- Read-only access to on-chain state
- Cannot delegate, undelegate, or redelegate
- Cannot submit proposals or votes
- Cannot execute transactions
- Informational output only

Matches the framework's principle of starting with the lowest-risk, highest-value capability. Raising the automation layer (e.g., automated delegation rebalancing to under-concentrated validators) is a separate governance decision.
