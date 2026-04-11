import { LedgerClient } from "../ledger.js";
import { config } from "../config.js";
import { describeDelegationFlows } from "../monitor.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type {
  DelegationEvent,
  DelegationFlow,
  DelegationFlowSummary,
  AlertLevel,
} from "../types.js";

/**
 * WF-VM-02: Delegation Flow Analysis
 *
 * Trigger: MsgDelegate / MsgUndelegate / MsgBeginRedelegate (real
 *          on-chain events via the LCD tx-search endpoint)
 * Layer: 1 (Fully Automated)
 *
 * OODA:
 *   Observe — Fetch recent delegation events from the LCD tx-search
 *             endpoint for all three staking message types.
 *   Orient  — Aggregate events per validator into DelegationFlow
 *             records. Compute inflow/outflow/net across the whole
 *             set. Tag whale-sized movements. Pick top inflow and
 *             outflow.
 *   Decide  — Generate flow summary via Claude.
 *   Act     — Persist, output, alert on whale activity.
 *
 * Previous versions of this workflow used a token-delta MVP proxy
 * — snapshotting `validator.tokens` per cycle and computing the
 * delta against the previous snapshot. That proxy worked but
 * could not distinguish delegate vs undelegate vs redelegate, did
 * not carry delegator identity, and missed all the intra-cycle
 * movements. The current implementation uses the real MsgDelegate
 * / MsgUndelegate / MsgBeginRedelegate tx stream, so every flow
 * in the summary is backed by a real on-chain transaction.
 */

interface Observations {
  events: DelegationEvent[];
}

interface Orientation {
  flows: DelegationFlow[];
  summary: DelegationFlowSummary;
}

interface Decision {
  summary: DelegationFlowSummary;
  report: string;
  alertLevel: AlertLevel;
}

interface Actions {
  saved: number;
  alertsSent: number;
}

export function absBig(x: bigint): bigint {
  return x < 0n ? -x : x;
}

/**
 * Aggregate a flat list of DelegationEvent records into
 * per-validator DelegationFlow records. Exported so unit tests
 * can feed it synthetic input without going through the observe
 * phase.
 *
 * Rules:
 *   - `delegate`   → inflow to `event.validator`
 *   - `undelegate` → outflow from `event.validator`
 *   - `redelegate` → outflow from `event.sourceValidator`,
 *                    inflow to `event.validator` (destination)
 */
export function aggregateEventsToFlows(
  events: DelegationEvent[],
  capturedAt: string = new Date().toISOString()
): DelegationFlow[] {
  // Per-validator running deltas in uregen, signed.
  const deltaByValidator = new Map<string, bigint>();

  const addDelta = (validator: string, delta: bigint) => {
    const prev = deltaByValidator.get(validator) ?? 0n;
    deltaByValidator.set(validator, prev + delta);
  };

  for (const ev of events) {
    let amt: bigint;
    try {
      amt = BigInt(ev.amountUregen);
    } catch {
      continue;
    }
    if (amt <= 0n) continue;

    if (ev.eventType === "delegate") {
      addDelta(ev.validator, amt);
    } else if (ev.eventType === "undelegate") {
      addDelta(ev.validator, -amt);
    } else if (ev.eventType === "redelegate") {
      if (ev.sourceValidator) addDelta(ev.sourceValidator, -amt);
      addDelta(ev.validator, amt);
    }
  }

  const whaleThreshold = BigInt(config.validator.whaleDelegationUregen);
  const flows: DelegationFlow[] = [];

  for (const [operatorAddress, delta] of deltaByValidator) {
    if (delta === 0n) continue;
    const deltaAbs = absBig(delta);
    flows.push({
      operatorAddress,
      // DelegationFlow carries a moniker for the narrative layer, but
      // the event stream only has validator operator addresses. The
      // orient phase inside the workflow resolves monikers by looking
      // up the validator set once per cycle; the aggregator here
      // leaves moniker as the operator address and expects the caller
      // to backfill. Callers that only care about the aggregate math
      // can use this field unchanged.
      moniker: operatorAddress,
      // Previous/current token fields are preserved for backward
      // compatibility with the DelegationFlow shape, but they no
      // longer correspond to validator.tokens snapshots — they
      // describe the delta window instead.
      previousTokens: "0",
      currentTokens: delta.toString(),
      deltaUregen: delta.toString(),
      deltaAbsUregen: deltaAbs.toString(),
      isWhale: deltaAbs >= whaleThreshold,
      flowDirection: delta > 0n ? "INFLOW" : "OUTFLOW",
      capturedAt,
    });
  }

  return flows;
}

/**
 * Summarize a list of DelegationFlow records into a
 * DelegationFlowSummary. Exported so unit tests can pin the
 * summary math independently of the aggregator.
 */
export function summarizeFlows(
  flows: DelegationFlow[],
  capturedAt: string = new Date().toISOString()
): DelegationFlowSummary {
  let totalInflow = 0n;
  let totalOutflow = 0n;
  let whaleFlowCount = 0;
  let topInflow: DelegationFlow | null = null;
  let topOutflow: DelegationFlow | null = null;

  for (const f of flows) {
    const delta = BigInt(f.deltaUregen);
    if (delta > 0n) {
      totalInflow += delta;
      if (!topInflow || delta > BigInt(topInflow.deltaUregen)) {
        topInflow = f;
      }
    } else if (delta < 0n) {
      totalOutflow += -delta;
      if (!topOutflow || -delta > BigInt(topOutflow.deltaAbsUregen)) {
        topOutflow = f;
      }
    }
    if (f.isWhale) whaleFlowCount++;
  }

  return {
    windowLabel: "recent tx-search window",
    totalInflowUregen: totalInflow.toString(),
    totalOutflowUregen: totalOutflow.toString(),
    netFlowUregen: (totalInflow - totalOutflow).toString(),
    validatorsWithFlow: flows.length,
    whaleFlowCount,
    topInflow,
    topOutflow,
    capturedAt,
  };
}

export function createDelegationFlowAnalysisWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-VM-02",
    name: "Delegation Flow Analysis",

    async observe(): Promise<Observations> {
      // Pull recent delegation tx events via the LCD tx-search
      // endpoint. The ledger client queries each of the three
      // staking message types separately and flattens the results
      // into a single DelegationEvent list.
      const events = await ledger.getRecentDelegationTxs(200);
      return { events };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const capturedAt = new Date().toISOString();
      const flows = aggregateEventsToFlows(obs.events, capturedAt);

      // Backfill monikers from the current validator set. This is a
      // single extra LCD call per cycle — cheap — and it lets the
      // narrative layer show operator names instead of opaque
      // bech32 addresses.
      const validators = await ledger.getValidators();
      const monikers = new Map<string, string>();
      for (const v of validators) {
        monikers.set(v.operator_address, v.description.moniker);
      }
      for (const f of flows) {
        const moniker = monikers.get(f.operatorAddress);
        if (moniker) f.moniker = moniker;
      }

      const summary = summarizeFlows(flows, capturedAt);
      return { flows, summary };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const report = await describeDelegationFlows(orientation.summary);

      const alertLevel: AlertLevel =
        orientation.summary.whaleFlowCount > 0 ? "HIGH" : "NORMAL";

      return {
        summary: orientation.summary,
        report,
        alertLevel,
      };
    },

    async act(decision: Decision): Promise<Actions> {
      let saved = 0;
      let alertsSent = 0;

      await output({
        workflow: "WF-VM-02",
        subjectId: "delegation-flows",
        title: `Net ${decision.summary.netFlowUregen} uregen (${decision.summary.whaleFlowCount} whale)`,
        content: decision.report,
        alertLevel: decision.alertLevel,
        timestamp: new Date(),
      });
      saved++;
      if (decision.alertLevel !== "NORMAL") alertsSent++;

      return { saved, alertsSent };
    },
  };
}
