import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { config } from "../config.js";
import { describeLiquidity } from "../monitor.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type {
  LiquiditySnapshot,
  SellOrder,
  CreditClass,
  AlertLevel,
} from "../types.js";
import { median, classIdFromBatchDenom } from "../utils.js";

/**
 * WF-MM-02: Liquidity Monitoring & Reporting
 *
 * Trigger: periodic (hourly) OR significant_trade (volume > $10k)
 * Layer: 1 (Fully Automated)
 *
 * OODA:
 *   Observe — Fetch all open sell orders, group by credit class
 *   Orient  — Compute liquidity metrics per class (listed value,
 *             bid-ask spread proxy, top-10 depth, health score)
 *   Decide  — Generate per-class liquidity reports. Flag classes
 *             whose depth falls below the configured floor.
 *   Act     — Persist snapshot, output report, alert on degradation.
 */

interface Observations {
  classes: CreditClass[];
  ordersByClass: Map<string, SellOrder[]>;
}

interface Orientation {
  snapshots: LiquiditySnapshot[];
}

interface Decision {
  reports: {
    snapshot: LiquiditySnapshot;
    previous: LiquiditySnapshot | null;
    report: string;
    alertLevel: AlertLevel;
  }[];
}

interface Actions {
  saved: number;
  alertsSent: number;
}

export function askUsd(order: SellOrder): number {
  const ask = Number(order.ask_amount);
  const qty = Number(order.quantity);
  if (!Number.isFinite(ask) || !Number.isFinite(qty) || qty <= 0) return 0;
  // MVP: treat ask as already-USD; price oracle integration is future work.
  return ask / qty;
}

export function scoreHealth(
  depthUsd: number,
  sellOrderCount: number
): { score: number; tier: LiquiditySnapshot["health"] } {
  const depthRatio = Math.min(1, depthUsd / (config.market.liquidityDepthFloor * 2));
  const countScore = Math.min(1, sellOrderCount / 20);
  const score = depthRatio * 0.7 + countScore * 0.3;

  let tier: LiquiditySnapshot["health"];
  if (depthUsd < config.market.liquidityDepthFloor * 0.5 || score < 0.3) {
    tier = "CRITICAL";
  } else if (depthUsd < config.market.liquidityDepthFloor || score < 0.6) {
    tier = "DEGRADED";
  } else {
    tier = "HEALTHY";
  }

  return { score, tier };
}

export function createLiquidityMonitorWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-MM-02",
    name: "Liquidity Monitoring & Reporting",

    async observe(): Promise<Observations> {
      const [classes, sellOrders] = await Promise.all([
        ledger.getCreditClasses(),
        ledger.getSellOrders(500),
      ]);

      const ordersByClass = new Map<string, SellOrder[]>();
      for (const order of sellOrders) {
        const classId = classIdFromBatchDenom(order.batch_denom);
        const bucket = ordersByClass.get(classId) || [];
        bucket.push(order);
        ordersByClass.set(classId, bucket);
      }

      return { classes, ordersByClass };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const snapshots: LiquiditySnapshot[] = [];
      const capturedAt = new Date().toISOString();

      // Snapshot every class that currently has any open sell orders.
      // Classes with zero open orders are intentionally skipped —
      // that's a "no market" state, distinct from a degraded market.
      for (const [classId, orders] of obs.ordersByClass) {
        if (orders.length === 0) continue;

        const prices = orders
          .map((o) => askUsd(o))
          .filter((p) => p > 0)
          .sort((a, b) => a - b);
        if (prices.length === 0) continue;

        const quantities = orders
          .map((o) => Number(o.quantity))
          .filter((q) => Number.isFinite(q) && q > 0);
        const totalListedQuantity = quantities.reduce((a, b) => a + b, 0);

        const listedValues = orders.map((o) => {
          const p = askUsd(o);
          const q = Number(o.quantity);
          return Number.isFinite(p) && Number.isFinite(q) ? p * q : 0;
        });
        const totalListedValueUsd = listedValues.reduce((a, b) => a + b, 0);

        // Pre-compute askUsd once per order so the comparator is a
        // plain number subtraction instead of recomputing the ratio on
        // every sort invocation (sort can call the comparator O(n log n)
        // times).
        const pricedOrders = orders
          .map((order) => ({ order, price: askUsd(order) }))
          .sort((a, b) => a.price - b.price);
        const top10 = pricedOrders.slice(0, 10);
        const depthUsd = top10.reduce((acc, { order, price }) => {
          const q = Number(order.quantity);
          return acc + (Number.isFinite(price) && Number.isFinite(q) ? price * q : 0);
        }, 0);

        const lowestAskUsd = prices[0]!;
        const highestAskUsd = prices[prices.length - 1]!;
        const medianAskUsd = median(prices);
        const meanAskUsd = prices.reduce((a, b) => a + b, 0) / prices.length;

        const { score, tier } = scoreHealth(depthUsd, orders.length);

        snapshots.push({
          classId,
          totalListedQuantity,
          totalListedValueUsd,
          sellOrderCount: orders.length,
          lowestAskUsd,
          highestAskUsd,
          medianAskUsd,
          meanAskUsd,
          depthUsd,
          healthScore: score,
          health: tier,
          capturedAt,
        });
      }

      return { snapshots };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const reports: Decision["reports"] = [];

      for (const snapshot of orientation.snapshots) {
        const prevRow = store.getLatestLiquiditySnapshot(snapshot.classId);
        let previous: LiquiditySnapshot | null = null;
        if (prevRow) {
          try {
            previous = JSON.parse(prevRow.snapshot) as LiquiditySnapshot;
          } catch {
            previous = null;
          }
        }

        const report = await describeLiquidity(snapshot, previous);

        const alertLevel: AlertLevel =
          snapshot.health === "CRITICAL"
            ? "CRITICAL"
            : snapshot.health === "DEGRADED"
              ? "HIGH"
              : "NORMAL";

        reports.push({ snapshot, previous, report, alertLevel });
      }

      return { reports };
    },

    async act(decision: Decision): Promise<Actions> {
      let saved = 0;
      let alertsSent = 0;

      for (const { snapshot, report, alertLevel } of decision.reports) {
        store.saveLiquiditySnapshot(
          snapshot.classId,
          JSON.stringify(snapshot),
          snapshot.health
        );
        saved++;

        await output({
          workflow: "WF-MM-02",
          subjectId: snapshot.classId,
          title: `Liquidity ${snapshot.health} (score ${snapshot.healthScore.toFixed(2)})`,
          content: report,
          alertLevel,
          timestamp: new Date(),
        });

        if (alertLevel !== "NORMAL") alertsSent++;
      }

      if (decision.reports.length === 0) {
        console.log("  No credit classes with open sell orders.");
      }

      return { saved, alertsSent };
    },
  };
}
