import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { config } from "../config.js";
import { describeDelegationFlows } from "../monitor.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type {
  Validator,
  DelegationFlow,
  DelegationFlowSummary,
  AlertLevel,
} from "../types.js";

/**
 * WF-VM-02: Delegation Flow Analysis
 *
 * Trigger: MsgDelegate / MsgUndelegate / MsgRedelegate (proxied by
 * per-cycle `validator.tokens` deltas until a real tx-stream lands)
 * Layer: 1 (Fully Automated)
 *
 * OODA:
 *   Observe — Snapshot current `tokens` per validator; pull the prior
 *             snapshot from SQLite and compute per-validator deltas.
 *   Orient  — Aggregate inflow / outflow / net. Tag whale-sized
 *             movements. Pick top inflow and outflow.
 *   Decide  — Generate flow summary via Claude.
 *   Act     — Persist, output, alert on whale activity.
 */

interface Observations {
  validators: Validator[];
  previousTokensByOperator: Map<string, string>;
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

export function createDelegationFlowAnalysisWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-VM-02",
    name: "Delegation Flow Analysis",

    async observe(): Promise<Observations> {
      const validators = await ledger.getValidators();

      const previousTokensByOperator = new Map<string, string>();
      for (const v of validators) {
        const prev = store.getPreviousTokenSnapshot(v.operator_address, false);
        if (prev) previousTokensByOperator.set(v.operator_address, prev.tokens);
      }

      // Write the new snapshots AFTER reading previous — we want the
      // delta to be "previous cycle vs this cycle".
      for (const v of validators) {
        store.recordTokenSnapshot({
          operatorAddress: v.operator_address,
          moniker: v.description.moniker,
          tokens: v.tokens,
          commissionRate: v.commission.commission_rates.rate,
          jailed: v.jailed,
        });
      }

      return { validators, previousTokensByOperator };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const capturedAt = new Date().toISOString();
      const flows: DelegationFlow[] = [];
      const whaleThreshold = BigInt(config.validator.whaleDelegationUregen);

      for (const v of obs.validators) {
        const previousStr = obs.previousTokensByOperator.get(v.operator_address);
        if (!previousStr) continue; // First time we see this validator; nothing to diff.

        const previous = BigInt(previousStr);
        const current = BigInt(v.tokens || "0");
        const delta = current - previous;
        const deltaAbs = absBig(delta);
        if (delta === 0n) continue;

        flows.push({
          operatorAddress: v.operator_address,
          moniker: v.description.moniker,
          previousTokens: previousStr,
          currentTokens: v.tokens,
          deltaUregen: delta.toString(),
          deltaAbsUregen: deltaAbs.toString(),
          isWhale: deltaAbs >= whaleThreshold,
          flowDirection: delta > 0n ? "INFLOW" : delta < 0n ? "OUTFLOW" : "FLAT",
          capturedAt,
        });
      }

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

      const summary: DelegationFlowSummary = {
        windowLabel: "since previous cycle",
        totalInflowUregen: totalInflow.toString(),
        totalOutflowUregen: totalOutflow.toString(),
        netFlowUregen: (totalInflow - totalOutflow).toString(),
        validatorsWithFlow: flows.length,
        whaleFlowCount,
        topInflow,
        topOutflow,
        capturedAt,
      };

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
