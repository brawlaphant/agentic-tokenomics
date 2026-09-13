# AGENT-003: Regen Market Monitor

**Layer 1 (fully automated, read-only, informational) agent that watches the Regen ecocredit marketplace, detects price anomalies, tracks liquidity health, and summarizes retirement demand.**

Mirrors the AGENT-002 Governance Analyst structure: the same OODA executor, the same standalone Node.js process shape, the same SQLite-backed local state. The scope is marketplace intelligence rather than governance.

## What it does

| Workflow | Trigger | Output |
|----------|---------|--------|
| **WF-MM-01** Price Anomaly Detection | New sell orders (SellOrderCreated / SellOrderFilled) | z-score anomaly alerts per severity, deduped |
| **WF-MM-02** Liquidity Monitoring | Periodic (every poll cycle) | Per-class liquidity health snapshot + trend vs previous |
| **WF-MM-03** Retirement Pattern Analysis | Periodic (every poll cycle) | Retirement volume, demand index, compliance metadata |

Each workflow is an **OODA loop** (Observe → Orient → Decide → Act) and persists both executions and domain state to SQLite. Numeric decisions (median, z-score, severity classification, health tier) are computed **deterministically**; Claude is only used for the narrative layer.

## Architecture

```
Regen Ledger (LCD REST API)
    ↓ observe
AGENT-003 (OODA engine)
    ↓ orient (deterministic)
    ↓ decide (deterministic)
Local SQLite (state)
    ↓ act (narrative via Claude)
Console / Discord webhook
```

**No MCP dependency.** Talks directly to any Cosmos LCD endpoint. When Ledger MCP becomes available, the `LedgerClient` can be swapped behind the same interface.

**No ElizaOS dependency.** Standalone Node.js process. The ElizaOS character for AGENT-003 lives at `agents/packages/agents/src/characters/market-monitor.ts`; this package shares the same system prompt text and the same threshold constants so downstream tooling has a single source of truth.

## Quick start

```bash
# 1. Install
cd agent-003-market-monitor
npm install

# 2. Configure
cp .env.example .env
# Edit .env — at minimum set ANTHROPIC_API_KEY

# 3. Run (single analysis pass)
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
| `POLL_INTERVAL_SECONDS` | No | `300` | Polling interval (seconds) |
| `ANTHROPIC_MODEL` | No | `claude-sonnet-4-5-20250929` | Claude model to use |

Thresholds live in `src/config.ts` under `market.*` and mirror the character definition at `agents/packages/agents/src/characters/market-monitor.ts`.

## How it maps to the framework specs

| Framework Spec | Implementation |
|----------------|---------------|
| Phase 2.2 WF-MM-01 | `src/workflows/price-anomaly-detection.ts` |
| Phase 2.2 WF-MM-02 | `src/workflows/liquidity-monitor.ts` |
| Phase 2.2 WF-MM-03 | `src/workflows/retirement-tracking.ts` |
| Phase 2.4 OODA executor | `src/ooda.ts` |
| Phase 2.4 Agent character | `agents/packages/agents/src/characters/market-monitor.ts` |
| Phase 2.5 Workflow executions table | `src/store.ts` (SQLite) |
| Phase 3.2 Ledger MCP client | `src/ledger.ts` (direct LCD) |

## Design decisions

1. **Deterministic numbers, narrative-only LLM calls.** Anomaly classification, liquidity health scoring, and demand index computation are all deterministic. Claude is only invoked to write the narrative report from those numbers. This keeps the agent cheap, reproducible, and auditable.

2. **Sell-order-as-trade proxy (MVP).** Until Ledger MCP exposes filled-trade events, the agent treats open sell orders as trade observations for the z-score baseline. An unusually high or low ask price is still a market structure signal worth surfacing, and the code path swaps cleanly when real trade events become available.

3. **MsgRetire tx-search as retirement source.** WF-MM-03 reads
   recent retirement transactions from the Cosmos LCD `tx-search`
   endpoint filtered on `message.action='/regen.ecocredit.v1.MsgRetire'`.
   Each tx response is parsed into zero or more Retirement records
   by harvesting `EventRetire` attributes (owner, batch_denom, amount,
   jurisdiction, reason) from either `logs[].events[]` or the
   flattened top-level `events[]` — the parser accepts both shapes
   for cross-SDK compatibility. Earlier drafts used a batch-supply
   delta as an MVP proxy; the current implementation produces
   richer Retirement records with retiree identity and jurisdiction
   metadata that the supply-delta proxy could not carry.

4. **Dedupe by trade + severity.** WF-MM-01 only alerts once per `(trade_id, severity)` tuple. A trade that later escalates from WARNING to CRITICAL still fires a new alert; a trade that stays at the same severity does not.

5. **Standalone over ElizaOS.** ElizaOS plugin API may change. A standalone process proves the workflow logic works independently of any runtime framework, matching AGENT-002's approach.

## Governance layer

This agent operates at **Layer 1 only**:

- Read-only access to on-chain state
- Cannot submit proposals
- Cannot create, modify, or cancel sell orders
- Cannot execute transactions
- Informational output only

This matches the framework's principle of starting with the lowest-risk, highest-value capability. Raising the automation layer is a separate governance decision.
