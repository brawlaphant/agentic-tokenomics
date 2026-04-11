import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { config } from "../config.js";
import { describeRetirementSummary } from "../monitor.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type { Retirement, RetirementSummary } from "../types.js";

/**
 * WF-MM-03: Retirement Pattern Analysis
 *
 * Trigger: MsgRetire events (LCD tx-search)
 * Layer: 1 (Fully Automated)
 *
 * OODA:
 *   Observe — Fetch recent MsgRetire tx responses from the LCD
 *             tx-search endpoint and parse them into Retirement
 *             records. Each tx can emit multiple retirements if
 *             the user batched them.
 *   Orient  — Group retirements by credit class. Aggregate totals,
 *             unique retirees, jurisdiction-metadata share, top
 *             retiree, and derive the demand index.
 *   Decide  — Generate narrative summary for classes that moved.
 *   Act     — Persist, output, alert on large positive demand shifts.
 *
 * Previous versions of this workflow used the per-batch supply
 * query as an MVP proxy for retirement activity — comparing
 * `retired_amount` across cycles to derive a delta. That worked
 * but could not carry retiree identity, jurisdiction, or
 * per-event timestamp. The current implementation uses the real
 * MsgRetire tx stream via `ledger.getRecentRetirementTxs`, so all
 * five fields on the Retirement record are now populated from
 * ledger state rather than synthesized.
 */

interface Observations {
  retirements: Retirement[];
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

/**
 * Aggregate a list of Retirement records into per-class summaries.
 * Exported so unit tests can feed it synthetic input without going
 * through the observe phase.
 *
 * `classPriceUsd` provides a per-class USD reference price (dollars
 * per credit) that valueing the retirement volume relies on. In
 * production the prices come from the most recent WF-MM-02 median ask
 * snapshot; tests inject a Map directly. Classes missing from the
 * map fall back to 1:1 USD, which preserves the prior behavior and
 * documents the "no price oracle yet" state.
 */
export function aggregateRetirementsByClass(
  retirements: Retirement[],
  capturedAt: string = new Date().toISOString(),
  classPriceUsd: Map<string, number> = new Map()
): Map<string, RetirementSummary> {
  const byClass = new Map<string, Retirement[]>();
  for (const r of retirements) {
    const bucket = byClass.get(r.classId) || [];
    bucket.push(r);
    byClass.set(r.classId, bucket);
  }

  const summariesByClass = new Map<string, RetirementSummary>();

  for (const [classId, rows] of byClass) {
    if (rows.length === 0) continue;

    let totalQuantity = 0;
    const retireeSet = new Set<string>();
    const retireeQuantity = new Map<string, number>();
    let jurisdictionCount = 0;

    for (const r of rows) {
      totalQuantity += r.quantity;
      if (r.retiree) {
        retireeSet.add(r.retiree);
        retireeQuantity.set(
          r.retiree,
          (retireeQuantity.get(r.retiree) ?? 0) + r.quantity
        );
      }
      if (r.jurisdiction) jurisdictionCount++;
    }

    if (totalQuantity === 0) continue;

    let topRetiree: string | null = null;
    let topRetireeQuantity = 0;
    for (const [retiree, qty] of retireeQuantity) {
      if (qty > topRetireeQuantity) {
        topRetiree = retiree;
        topRetireeQuantity = qty;
      }
    }

    const retirementCount = rows.length;
    const uniqueRetirees = retireeSet.size;
    const pctWithJurisdiction =
      retirementCount > 0 ? (jurisdictionCount / retirementCount) * 100 : 0;

    // Value retirement volume using the most recent median ask
    // price from WF-MM-02 for this class when available. Falls back
    // to 1:1 USD when no snapshot exists yet — this keeps the field
    // populated without pretending there's a price we don't have.
    const unitPriceUsd = classPriceUsd.get(classId) ?? 1;
    const totalValueUsd = totalQuantity * unitPriceUsd;
    const demandIndex = computeDemandIndex(
      totalQuantity,
      retirementCount,
      uniqueRetirees
    );

    summariesByClass.set(classId, {
      classId,
      windowHours: config.market.retirementWindowHours,
      retirementCount,
      totalQuantity,
      totalValueUsd,
      uniqueRetirees,
      topRetiree,
      topRetireeQuantity,
      pctWithJurisdiction,
      demandIndex,
      capturedAt,
    });
  }

  return summariesByClass;
}

export function createRetirementTrackingWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-MM-03",
    name: "Retirement Pattern Analysis",

    async observe(): Promise<Observations> {
      // Pull the most recent retirement transactions from the LCD
      // tx-search endpoint. Each tx response can emit multiple
      // MsgRetire events (batched retirements) — the ledger client
      // flattens them into individual Retirement records.
      const retirements = await ledger.getRecentRetirementTxs(200);
      return { retirements };
    },

    async orient(obs: Observations): Promise<Orientation> {
      // Build the per-class price map from the latest liquidity
      // snapshots so aggregation can value retirement volume without
      // duplicating the ask-price parsing logic.
      const classPriceUsd = new Map<string, number>();
      const distinctClassIds = new Set(obs.retirements.map((r) => r.classId));
      for (const classId of distinctClassIds) {
        const price = store.getLatestMedianAsk(classId);
        if (price !== null) classPriceUsd.set(classId, price);
      }

      const summariesByClass = aggregateRetirementsByClass(
        obs.retirements,
        undefined,
        classPriceUsd
      );
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
