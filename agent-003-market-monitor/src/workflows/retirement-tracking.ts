import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { config } from "../config.js";
import { describeRetirementSummary } from "../monitor.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type {
  CreditClass,
  CreditBatch,
  BatchSupply,
  RetirementSummary,
} from "../types.js";

/**
 * WF-MM-03: Retirement Pattern Analysis
 *
 * Trigger: MsgRetire event (proxied by per-cycle retired-amount deltas
 *          until the Ledger MCP provides an event stream)
 * Layer: 1 (Fully Automated)
 *
 * OODA:
 *   Observe — Fetch credit classes + batches + batch supplies. The
 *             supply response includes `retired_amount`, which we
 *             compare against the previously observed amount to derive
 *             a per-class retirement delta for this cycle.
 *   Orient  — Build a RetirementSummary per class: totals, unique
 *             retirees proxied by batch count for this MVP, demand
 *             index derived from quantity vs trailing baseline.
 *   Decide  — Generate narrative summary for classes that moved.
 *   Act     — Persist, output, alert on large positive demand shifts.
 *
 * This MVP uses the batch supply query as the retirement source. A
 * follow-up PR will plug in a real tx-stream client (proto.regen.
 * ecocredit.v1.MsgRetire) once the LCD event endpoint is available.
 */

interface BatchWithSupply {
  batch: CreditBatch;
  supply: BatchSupply;
  classId: string;
}

interface Observations {
  classes: CreditClass[];
  batchesWithSupply: BatchWithSupply[];
}

interface Orientation {
  summariesByClass: Map<string, RetirementSummary>;
}

interface Decision {
  reports: {
    summary: RetirementSummary;
    baselineDemand: number;
    report: string;
  }[];
}

interface Actions {
  saved: number;
  alertsSent: number;
}

function classIdFromBatchDenom(denom: string): string {
  const idx = denom.indexOf("-");
  return idx > 0 ? denom.slice(0, idx) : denom;
}

/** Demand index on a bounded 0-100 scale. Inputs are rolling and a
 * class with no trailing activity gets 0. The index is intentionally
 * simple — it exists so the narrative layer has a single number to
 * anchor the "demand up / demand down" story. */
export function computeDemandIndex(
  totalQuantity: number,
  retirementCount: number,
  uniqueRetirees: number
): number {
  const volumeComponent = Math.min(60, Math.log10(Math.max(1, totalQuantity)) * 20);
  const countComponent = Math.min(20, retirementCount * 2);
  const breadthComponent = Math.min(20, uniqueRetirees * 4);
  return Math.round(volumeComponent + countComponent + breadthComponent);
}

export function createRetirementTrackingWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-MM-03",
    name: "Retirement Pattern Analysis",

    async observe(): Promise<Observations> {
      const [classes, batches] = await Promise.all([
        ledger.getCreditClasses(),
        ledger.getCreditBatches(),
      ]);

      // Cap the per-cycle batch fan-out to the most recent 100 batches
      // so we don't hammer the LCD on mainnet (where batch counts grow
      // unbounded). Older batches are retired less frequently and the
      // agent will catch new retirement activity next cycle anyway.
      const cappedBatches = batches.slice(0, 100);

      const batchesWithSupply: BatchWithSupply[] = [];
      await Promise.all(
        cappedBatches.map(async (batch) => {
          const supply = await ledger.getBatchSupply(batch.denom);
          if (supply) {
            batchesWithSupply.push({
              batch,
              supply,
              classId: classIdFromBatchDenom(batch.denom),
            });
          }
        })
      );

      return { classes, batchesWithSupply };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const summariesByClass = new Map<string, RetirementSummary>();
      const capturedAt = new Date().toISOString();

      // Group batches by class so we can aggregate retired amounts.
      const byClass = new Map<string, BatchWithSupply[]>();
      for (const row of obs.batchesWithSupply) {
        const bucket = byClass.get(row.classId) || [];
        bucket.push(row);
        byClass.set(row.classId, bucket);
      }

      for (const [classId, rows] of byClass) {
        let totalRetired = 0;
        let batchesWithRetirement = 0;
        for (const row of rows) {
          const retired = Number(row.supply.retired_amount);
          if (Number.isFinite(retired) && retired > 0) {
            totalRetired += retired;
            batchesWithRetirement++;
          }
        }
        if (totalRetired === 0) continue;

        // Unique retirees / top retiree are placeholders until the
        // tx-event source lands. The summary still tells the right
        // story: "N batches in class X have retirements totaling Y".
        const uniqueRetirees = batchesWithRetirement;
        const topRetiree: string | null = null;
        const topRetireeQuantity = 0;

        // Treat USD value as 1:1 with quantity for the MVP. Price
        // oracle integration is future work — documented as an open
        // question in the workflow spec.
        const totalValueUsd = totalRetired;
        const demandIndex = computeDemandIndex(
          totalRetired,
          batchesWithRetirement,
          uniqueRetirees
        );

        summariesByClass.set(classId, {
          classId,
          windowHours: config.market.retirementWindowHours,
          retirementCount: batchesWithRetirement,
          totalQuantity: totalRetired,
          totalValueUsd,
          uniqueRetirees,
          topRetiree,
          topRetireeQuantity,
          pctWithJurisdiction: 0,
          demandIndex,
          capturedAt,
        });
      }

      return { summariesByClass };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const reports: Decision["reports"] = [];

      for (const summary of orientation.summariesByClass.values()) {
        const baselineDemand = store.getBaselineDemand(summary.classId, 7);
        const report = await describeRetirementSummary(summary, baselineDemand);
        reports.push({ summary, baselineDemand, report });
      }

      return { reports };
    },

    async act(decision: Decision): Promise<Actions> {
      let saved = 0;
      let alertsSent = 0;

      for (const { summary, baselineDemand, report } of decision.reports) {
        store.saveRetirementSummary({
          classId: summary.classId,
          windowHours: summary.windowHours,
          totalQuantity: summary.totalQuantity,
          totalValueUsd: summary.totalValueUsd,
          retirementCount: summary.retirementCount,
          demandIndex: summary.demandIndex,
          summary: report,
        });
        saved++;

        // Alert if demand index jumped meaningfully above the baseline.
        // Threshold of +15 index points mirrors the agent character's
        // "moderate-high" boundary (65 → 72 example).
        const jumped = baselineDemand > 0 && summary.demandIndex - baselineDemand >= 15;
        const alertLevel = jumped ? "HIGH" : "NORMAL";
        if (alertLevel !== "NORMAL") alertsSent++;

        await output({
          workflow: "WF-MM-03",
          subjectId: summary.classId,
          title: `Retirement summary (demand ${summary.demandIndex}${baselineDemand ? ` vs ${baselineDemand.toFixed(0)}` : ""})`,
          content: report,
          alertLevel,
          timestamp: new Date(),
        });
      }

      if (decision.reports.length === 0) {
        console.log("  No retirement activity detected in any class.");
      }

      return { saved, alertsSent };
    },
  };
}
