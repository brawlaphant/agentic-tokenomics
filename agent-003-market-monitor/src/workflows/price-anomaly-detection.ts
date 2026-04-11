import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { config } from "../config.js";
import { describePriceAnomaly } from "../monitor.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type {
  Trade,
  PriceAnomaly,
  AnomalySeverity,
  SellOrder,
} from "../types.js";
import {
  median,
  stddev,
  classIdFromBatchDenom,
  isUsdStableDenom,
} from "../utils.js";

// Re-exported so the existing unit-test suite can import helpers from
// this workflow file (the tests pre-date the utils.ts extraction).
export { median, stddev, classIdFromBatchDenom, isUsdStableDenom };

/**
 * WF-MM-01: Price Anomaly Detection
 *
 * Trigger: SellOrderCreated OR SellOrderFilled
 * Layer: 1 (alerts) / Layer 2 (if action needed)
 * SLA: Alert within one polling cycle of the trade landing on-chain
 *
 * OODA:
 *   Observe — Fetch current sell orders + record as synthetic trade
 *             observations. When Ledger MCP provides real fill data,
 *             the observe phase swaps in real trades.
 *   Orient  — For each new observation, compute z-score vs class
 *             median and batch median using the trailing window.
 *   Decide  — Classify severity (INFO / WARNING / CRITICAL) per the
 *             thresholds in config.market. Only surface WARNING or
 *             CRITICAL. Dedupe against previous alerts for this trade.
 *   Act     — Persist, generate narrative via Claude, output to channels.
 *
 * Determinism: all numeric decisions (median, z-score, severity) are
 * computed locally. Claude is only used for the narrative report.
 */

interface Observations {
  newTrades: Trade[];
}

interface Orientation {
  anomalies: PriceAnomaly[];
}

interface Decision {
  alertsToPublish: PriceAnomaly[];
}

interface Actions {
  alerted: number;
  skipped: number;
}

/**
 * Convert a sell order into a synthetic "trade observation" for this
 * agent. Until Ledger MCP provides filled-trade events, the agent uses
 * the ask price as a proxy signal — an unusually high/low ask price
 * still indicates a market structure anomaly worth surfacing.
 */
function sellOrderToTrade(order: SellOrder, classId: string): Trade | null {
  const qty = Number(order.quantity);
  const ask = Number(order.ask_amount);
  if (!Number.isFinite(qty) || !Number.isFinite(ask) || qty <= 0) return null;

  // Only USD stablecoin asks are priced — anything else would require
  // an oracle we don't have yet, and letting non-USD orders into the
  // baseline would pollute the class/batch medians with 1:1-priced
  // garbage. Downstream code can plug in a real price oracle.
  if (!isUsdStableDenom(order.ask_denom)) return null;
  const pricePerCredit = ask / qty;

  return {
    id: order.id,
    batchDenom: order.batch_denom,
    classId,
    seller: order.seller,
    buyer: "",
    quantity: qty,
    pricePerCredit,
    askDenom: order.ask_denom,
    executedAt: new Date().toISOString(),
  };
}

export function classifyAnomaly(
  zClass: number,
  zBatch: number
): AnomalySeverity {
  const maxZ = Math.max(Math.abs(zClass), Math.abs(zBatch));
  if (maxZ >= config.market.criticalZScore) return "CRITICAL";
  if (maxZ >= config.market.warningZScore) return "WARNING";
  return "INFO";
}

export function createPriceAnomalyDetectionWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-MM-01",
    name: "Price Anomaly Detection",

    async observe(): Promise<Observations> {
      const sellOrders = await ledger.getSellOrders(200);

      const newTrades: Trade[] = [];
      for (const order of sellOrders) {
        const classId = classIdFromBatchDenom(order.batch_denom);
        const trade = sellOrderToTrade(order, classId);
        if (trade) {
          newTrades.push(trade);
          store.recordTrade({
            tradeId: trade.id,
            classId: trade.classId,
            batchDenom: trade.batchDenom,
            pricePerCredit: trade.pricePerCredit,
            quantity: trade.quantity,
            executedAt: trade.executedAt,
          });
        }
      }

      return { newTrades };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const anomalies: PriceAnomaly[] = [];

      for (const trade of obs.newTrades) {
        const classSamples = store.getTradesByClass(
          trade.classId,
          config.market.classMedianWindowDays
        );
        const batchSamples = store.getTradesByBatch(
          trade.batchDenom,
          config.market.batchMedianWindowDays
        );

        if (classSamples.length < config.market.minSamples) continue;

        const classMedian = median(classSamples);
        const classMean = classSamples.reduce((a, b) => a + b, 0) / classSamples.length;
        const classStd = stddev(classSamples, classMean);
        // Z-score is (value - mean) / stddev; using median in the
        // numerator while dividing by stddev (computed off the mean)
        // would be statistically inconsistent. Median is still
        // published in the alert payload as a context signal.
        const zClass = classStd > 0 ? (trade.pricePerCredit - classMean) / classStd : 0;

        const batchMedian = batchSamples.length > 0 ? median(batchSamples) : classMedian;
        const batchMean =
          batchSamples.length > 0
            ? batchSamples.reduce((a, b) => a + b, 0) / batchSamples.length
            : classMean;
        const batchStd = batchSamples.length > 1 ? stddev(batchSamples, batchMean) : classStd;
        const zBatch = batchStd > 0 ? (trade.pricePerCredit - batchMean) / batchStd : 0;

        const severity = classifyAnomaly(zClass, zBatch);
        if (severity === "INFO") continue;

        // Confidence rises with sample size and drops when class and
        // batch z-scores disagree (which often indicates the batch is
        // just thinly traded rather than a real anomaly).
        const sampleConfidence = Math.min(1, classSamples.length / 30);
        const agreement =
          Math.sign(zClass) === Math.sign(zBatch) && Math.abs(zBatch) > 0
            ? 1
            : 0.6;
        const confidence = Math.max(0, Math.min(1, sampleConfidence * agreement));

        anomalies.push({
          tradeId: trade.id,
          batchDenom: trade.batchDenom,
          classId: trade.classId,
          seller: trade.seller,
          quantity: trade.quantity,
          pricePerCredit: trade.pricePerCredit,
          classMedian,
          batchMedian,
          zScoreVsClass: zClass,
          zScoreVsBatch: zBatch,
          sampleSizeClass: classSamples.length,
          sampleSizeBatch: batchSamples.length,
          severity,
          detectedAt: new Date().toISOString(),
          confidence,
        });
      }

      return { anomalies };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const alertsToPublish: PriceAnomaly[] = [];

      for (const anomaly of orientation.anomalies) {
        const existing = store.getAnomalyAlertSeverity(anomaly.tradeId);
        // Dedupe: don't re-alert at the same or lower severity.
        if (existing === "CRITICAL") continue;
        if (existing === "WARNING" && anomaly.severity !== "CRITICAL") continue;

        alertsToPublish.push(anomaly);
      }

      return { alertsToPublish };
    },

    async act(decision: Decision): Promise<Actions> {
      let alerted = 0;
      let skipped = 0;

      for (const anomaly of decision.alertsToPublish) {
        const report = await describePriceAnomaly(anomaly);

        store.saveAnomalyAlert({
          tradeId: anomaly.tradeId,
          severity: anomaly.severity,
          zClass: anomaly.zScoreVsClass,
          zBatch: anomaly.zScoreVsBatch,
          report,
        });

        await output({
          workflow: "WF-MM-01",
          subjectId: anomaly.tradeId,
          title: `${anomaly.severity} — ${anomaly.classId} / ${anomaly.batchDenom}`,
          content: report,
          alertLevel: anomaly.severity === "CRITICAL" ? "CRITICAL" : "HIGH",
          timestamp: new Date(),
        });
        alerted++;
      }

      if (decision.alertsToPublish.length === 0) {
        console.log("  No new price anomalies above threshold.");
      }

      return { alerted, skipped };
    },
  };
}
