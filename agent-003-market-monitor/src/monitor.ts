import Anthropic from "@anthropic-ai/sdk";
import { config } from "./config.js";
import type {
  PriceAnomaly,
  LiquiditySnapshot,
  RetirementSummary,
} from "./types.js";

const client = new Anthropic({ apiKey: config.anthropicApiKey });

/**
 * System prompt mirrors the AGENT-003 character definition at
 * agents/packages/agents/src/characters/market-monitor.ts. Thresholds
 * are injected from config so a single source of truth drives both the
 * deterministic pipeline AND the narrative layer.
 */
const SYSTEM_PROMPT = `You are the Regen Market Monitor Agent (AGENT-003).

Your responsibilities:
1. Monitoring ecological credit prices across all credit classes
2. Tracking marketplace liquidity and order book health
3. Analyzing retirement patterns and demand signals
4. Detecting price anomalies and potential manipulation

Core Principles:
- Prioritize market integrity above all else
- Minimize false positives — verify anomalies against multiple data sources
- Present data with precision — include units, timeframes, and confidence intervals
- Never provide trading advice or price predictions
- Cite the deterministic numbers passed to you; do not invent values

Workflows:
- WF-MM-01 (Price Impact Alert): z-score analysis (CRITICAL >= ${config.market.criticalZScore}, WARNING >= ${config.market.warningZScore})
- WF-MM-02 (Liquidity Monitor): hourly liquidity health checks
- WF-MM-03 (Retirement Tracking): demand signals and impact quantification

Alert Severity Levels:
- INFO: Normal activity, logged for trend analysis
- WARNING: z-score between ${config.market.warningZScore} and ${config.market.criticalZScore}, added to watchlist
- CRITICAL: z-score >= ${config.market.criticalZScore}, escalate for investigation

Output Format:
- Use markdown with explicit tables
- Include severity level in the title of every alert
- Include units, timeframes, and sample sizes
- Quantify confidence when relevant`;

// ============================================================
// WF-MM-01: Price anomaly narrative
// ============================================================

export async function describePriceAnomaly(
  anomaly: PriceAnomaly
): Promise<string> {
  const prompt = `Generate a Price Impact Alert report for this anomaly detection.

## Deterministic Pipeline Output
- Trade ID: ${anomaly.tradeId}
- Credit Class: ${anomaly.classId}
- Batch: ${anomaly.batchDenom}
- Seller: ${anomaly.seller}
- Quantity: ${anomaly.quantity}
- Trade Price: $${anomaly.pricePerCredit.toFixed(4)}/credit
- Class Median (${config.market.classMedianWindowDays}d): $${anomaly.classMedian.toFixed(4)}/credit  (n=${anomaly.sampleSizeClass})
- Batch Median (${config.market.batchMedianWindowDays}d): $${anomaly.batchMedian.toFixed(4)}/credit  (n=${anomaly.sampleSizeBatch})
- Z-score vs class: ${anomaly.zScoreVsClass.toFixed(2)}
- Z-score vs batch: ${anomaly.zScoreVsBatch.toFixed(2)}
- Severity: ${anomaly.severity}
- Confidence: ${anomaly.confidence.toFixed(2)}
- Detected: ${anomaly.detectedAt}

Generate a structured Markdown report with:
1. A header "Price Impact Alert" followed by severity, class, batch, timestamp
2. An "Anomaly Details" table using ONLY the numbers above
3. A "Context" section (bullet points) — note the relative deviation, sample size adequacy, and any concerns about class vs batch z-score divergence
4. An "Action" section — what the agent has done (watchlist add / escalation) and what the next review is

Do not invent any numbers beyond what's provided. Do not recommend trading positions.`;

  const response = await client.messages.create({
    model: config.model,
    max_tokens: 1000,
    system: SYSTEM_PROMPT,
    messages: [{ role: "user", content: prompt }],
  });

  return extractText(response);
}

// ============================================================
// WF-MM-02: Liquidity narrative
// ============================================================

export async function describeLiquidity(
  snapshot: LiquiditySnapshot,
  previousSnapshot: LiquiditySnapshot | null
): Promise<string> {
  const prev = previousSnapshot
    ? `
## Previous Snapshot (${previousSnapshot.capturedAt})
- Listed value: $${previousSnapshot.totalListedValueUsd.toFixed(2)}
- Median ask: $${previousSnapshot.medianAskUsd.toFixed(4)}
- Depth (top-10): $${previousSnapshot.depthUsd.toFixed(2)}
- Health: ${previousSnapshot.health} (${previousSnapshot.healthScore.toFixed(2)})
`
    : "## Previous Snapshot\nNone — this is the first snapshot for this class.";

  const prompt = `Generate a Liquidity Report for the Regen ecocredit marketplace.

## Deterministic Pipeline Output
- Class: ${snapshot.classId}
- Sell orders: ${snapshot.sellOrderCount}
- Listed quantity: ${snapshot.totalListedQuantity}
- Listed value: $${snapshot.totalListedValueUsd.toFixed(2)}
- Lowest ask: $${snapshot.lowestAskUsd.toFixed(4)}
- Highest ask: $${snapshot.highestAskUsd.toFixed(4)}
- Median ask: $${snapshot.medianAskUsd.toFixed(4)}
- Mean ask: $${snapshot.meanAskUsd.toFixed(4)}
- Depth (top-10 sum): $${snapshot.depthUsd.toFixed(2)}
- Health score: ${snapshot.healthScore.toFixed(2)}
- Health: ${snapshot.health}
- Captured: ${snapshot.capturedAt}

${prev}

Generate a structured Markdown report with:
1. Header "Liquidity Report — ${snapshot.classId}" with health badge
2. A "Current Snapshot" table with the numbers above
3. A "Trend" section comparing to the previous snapshot (or noting this is the first)
4. A "Health Assessment" section — interpret the depth floor vs the configured liquidity_depth_floor
5. A one-line "Alert" line if health is DEGRADED or CRITICAL

Use ONLY the numbers provided. Do not invent trade volumes, slippage estimates, or predictions.`;

  const response = await client.messages.create({
    model: config.model,
    max_tokens: 1000,
    system: SYSTEM_PROMPT,
    messages: [{ role: "user", content: prompt }],
  });

  return extractText(response);
}

// ============================================================
// WF-MM-03: Retirement narrative
// ============================================================

export async function describeRetirementSummary(
  summary: RetirementSummary,
  baselineDemand: number
): Promise<string> {
  const trend =
    baselineDemand === 0
      ? "no prior baseline"
      : summary.demandIndex > baselineDemand
        ? `up from ${baselineDemand.toFixed(0)}`
        : summary.demandIndex < baselineDemand
          ? `down from ${baselineDemand.toFixed(0)}`
          : `flat vs ${baselineDemand.toFixed(0)}`;

  const prompt = `Generate a Retirement Pattern Summary for the Regen ecocredit marketplace.

## Deterministic Pipeline Output
- Class: ${summary.classId}
- Window: trailing ${summary.windowHours} hours
- Retirements (count): ${summary.retirementCount}
- Total quantity: ${summary.totalQuantity}
- Total value: $${summary.totalValueUsd.toFixed(2)}
- Unique retirees: ${summary.uniqueRetirees}
- Top retiree: ${summary.topRetiree ?? "(none)"}
- Top retiree quantity: ${summary.topRetireeQuantity}
- % with jurisdiction metadata: ${summary.pctWithJurisdiction.toFixed(1)}%
- Demand index (0-100): ${summary.demandIndex.toFixed(0)}  (${trend})
- Captured: ${summary.capturedAt}

Generate a structured Markdown report with:
1. Header "Retirement Summary — ${summary.classId}"
2. "Volume" table with counts, quantity, value
3. "Demand Signals" section — interpret the demand index and trend
4. "Concentration" section — note if one retiree accounts for an unusual share
5. "Metadata Compliance" note — if % with jurisdiction is high, that usually implies compliance-driven demand

Use ONLY the numbers provided. Do not extrapolate future demand.`;

  const response = await client.messages.create({
    model: config.model,
    max_tokens: 1000,
    system: SYSTEM_PROMPT,
    messages: [{ role: "user", content: prompt }],
  });

  return extractText(response);
}

// ============================================================
// Helpers
// ============================================================

function extractText(response: Anthropic.Message): string {
  return response.content
    .filter((b): b is Anthropic.TextBlock => b.type === "text")
    .map((b) => b.text)
    .join("\n");
}
