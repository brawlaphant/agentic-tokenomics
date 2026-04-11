import Anthropic from "@anthropic-ai/sdk";
import { config } from "./config.js";
import type {
  ValidatorScorecard,
  DelegationFlowSummary,
  DecentralizationSnapshot,
  PerformanceAlert,
} from "./types.js";

const client = new Anthropic({ apiKey: config.anthropicApiKey });

/**
 * System prompt mirrors the AGENT-004 character definition at
 * agents/packages/agents/src/characters/validator-monitor.ts. All
 * thresholds are injected from config so the deterministic pipeline
 * and the narrative layer cannot drift.
 */
const SYSTEM_PROMPT = `You are the Regen Validator Monitor Agent (AGENT-004).

Your responsibilities:
1. Tracking validator uptime and block production performance
2. Monitoring governance voting participation
3. Analyzing delegation flows, whale movements, and concentration
4. Computing M014 performance scores
5. Assessing network decentralization and PoA transition readiness

Core Principles:
- Prioritize network security and decentralization above all else
- Present performance data objectively
- Track trends over time, not just point-in-time snapshots
- Never recommend delegation choices; surface data for delegators to decide

Scoring Methodology (M014):
- Uptime weight: ${config.validator.scoreWeightUptime} / 1000
- Governance weight: ${config.validator.scoreWeightGovernance} / 1000
- Stability weight: ${config.validator.scoreWeightStability} / 1000
- PoA eligibility: composite >= ${config.validator.poaEligibilityScore}

Alert Levels:
- NORMAL: Metrics within healthy bounds
- WARNING / HIGH: Degradation detected
- CRITICAL: Immediate risk to network health (e.g., single validator > ${config.validator.criticalConcentrationPct}% of stake)

Output Format:
- Use markdown tables with units
- Include timeframes for all metrics
- Cite the deterministic numbers passed to you; do not invent values`;

// ============================================================
// WF-VM-01: Performance narrative
// ============================================================

export async function describePerformanceReport(
  scorecards: ValidatorScorecard[],
  alerts: PerformanceAlert[],
  bondedUregen: string
): Promise<string> {
  const poaEligible = scorecards.filter((s) => s.poaEligible).length;
  const top = scorecards.slice(0, 10);

  const prompt = `Generate a Validator Performance Report for the Regen Network active validator set.

## Deterministic Pipeline Output
- Active set: ${scorecards.length} validators
- Bonded: ${bondedUregen} uregen
- PoA eligible (composite >= ${config.validator.poaEligibilityScore}): ${poaEligible}/${scorecards.length}

## Top 10 Validators by Composite Score
${top
  .map(
    (s, i) =>
      `${i + 1}. ${s.moniker} (${s.operatorAddress.slice(0, 14)}…) — composite ${s.compositeScore}  (uptime ${s.uptimeScore}, gov ${s.governanceScore}, stability ${s.stabilityScore}) — stake ${s.stakePct.toFixed(2)}%`
  )
  .join("\n")}

## Alerts (${alerts.length})
${
  alerts.length === 0
    ? "- None"
    : alerts.map((a) => `- ${a.severity}: ${a.moniker} — ${a.reason}`).join("\n")
}

Generate a structured Markdown report with:
1. Header "Validator Performance Report" with active set size and PoA eligibility ratio
2. "Top 10 Validators" table with: rank, moniker, uptime ratio, gov participation, stability, composite
3. "Scoring Methodology (M014)" block using ONLY the weights passed in the prompt
4. "Alerts This Period" bullet list using ONLY the alerts passed in
5. "PoA Transition Readiness" section — ratio of validators meeting the threshold, no speculation about when PoA will activate

Use ONLY the numbers provided. Do not recommend delegation choices.`;

  const response = await client.messages.create({
    model: config.model,
    max_tokens: 1500,
    system: SYSTEM_PROMPT,
    messages: [{ role: "user", content: prompt }],
  });

  return extractText(response);
}

// ============================================================
// WF-VM-02: Delegation flow narrative
// ============================================================

export async function describeDelegationFlows(
  summary: DelegationFlowSummary
): Promise<string> {
  const prompt = `Generate a Delegation Flow Report for the Regen staking set.

## Deterministic Pipeline Output
- Window: ${summary.windowLabel}
- Validators with any flow: ${summary.validatorsWithFlow}
- Total inflow: ${summary.totalInflowUregen} uregen
- Total outflow: ${summary.totalOutflowUregen} uregen
- Net flow (signed): ${summary.netFlowUregen} uregen
- Whale-sized flows (>= ${config.validator.whaleDelegationUregen} uregen): ${summary.whaleFlowCount}
- Top inflow validator: ${summary.topInflow ? `${summary.topInflow.moniker} (+${summary.topInflow.deltaAbsUregen} uregen)` : "(none)"}
- Top outflow validator: ${summary.topOutflow ? `${summary.topOutflow.moniker} (-${summary.topOutflow.deltaAbsUregen} uregen)` : "(none)"}
- Captured: ${summary.capturedAt}

Generate a structured Markdown report with:
1. Header "Delegation Flow Report" with severity chosen from the whale count and net flow sign
2. "Flow Summary" table with inflow / outflow / net / whale count
3. "Notable Movements" section listing top inflow and top outflow (or a note if none)
4. "Assessment" section — one or two sentences noting whether the net flow direction warrants continued monitoring

Use ONLY the numbers provided. Do not predict future delegation behavior.`;

  const response = await client.messages.create({
    model: config.model,
    max_tokens: 1000,
    system: SYSTEM_PROMPT,
    messages: [{ role: "user", content: prompt }],
  });

  return extractText(response);
}

// ============================================================
// WF-VM-03: Decentralization narrative
// ============================================================

export async function describeDecentralizationSnapshot(
  snapshot: DecentralizationSnapshot,
  previous: DecentralizationSnapshot | null
): Promise<string> {
  const prev = previous
    ? `
## Previous Snapshot (${previous.capturedAt})
- Validator count: ${previous.validatorCount}
- Nakamoto: ${previous.nakamotoCoefficient}
- Gini: ${previous.giniIndex.toFixed(3)}
- Top-10 share: ${previous.top10SharePct.toFixed(2)}%
- Health: ${previous.health}
`
    : "## Previous Snapshot\nNone — this is the first decentralization snapshot.";

  const prompt = `Generate a Network Decentralization Report for Regen Network.

## Deterministic Pipeline Output
- Active validator count: ${snapshot.validatorCount}
- Bonded: ${snapshot.bondedUregen} uregen
- Nakamoto coefficient: ${snapshot.nakamotoCoefficient}
- Gini index: ${snapshot.giniIndex.toFixed(3)}
- Top-10 share: ${snapshot.top10SharePct.toFixed(2)}%
- Top-20 share: ${snapshot.top20SharePct.toFixed(2)}%
- Largest validator: ${snapshot.largestShareValidator} (${snapshot.largestSharePct.toFixed(2)}%)
- Health: ${snapshot.health}
- Captured: ${snapshot.capturedAt}

${prev}

Generate a structured Markdown report with:
1. Header "Network Decentralization Report" with health badge
2. "Metrics" table with current snapshot values
3. "Trend" section comparing to the previous snapshot (or noting this is the first)
4. "Assessment" — one paragraph interpreting the Nakamoto coefficient and Gini. If health is WARNING or CRITICAL, explain what triggered it using the thresholds in config (nakamoto < ${config.validator.nakamotoWarningFloor}, gini > ${config.validator.giniWarningCeiling}, single validator > ${config.validator.criticalConcentrationPct}%)

Use ONLY the numbers provided. Do not speculate about future decentralization.`;

  const response = await client.messages.create({
    model: config.model,
    max_tokens: 1200,
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
