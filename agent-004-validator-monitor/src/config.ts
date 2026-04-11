export const config = {
  // Regen LCD endpoint
  lcdUrl: process.env.REGEN_LCD_URL || "https://regen.api.chandrastation.com",

  // Anthropic
  anthropicApiKey: process.env.ANTHROPIC_API_KEY || "",
  model: process.env.ANTHROPIC_MODEL || "claude-sonnet-4-5-20250929",

  // Discord webhook (optional)
  discordWebhookUrl: process.env.DISCORD_WEBHOOK_URL || "",

  // Polling — validator state moves slowly; 15 min is plenty and
  // keeps the agent off the LCD's rate-limit radar.
  pollIntervalMs: (parseInt(process.env.POLL_INTERVAL_SECONDS || "900", 10)) * 1000,

  // Validator monitor thresholds. These mirror the character
  // thresholds in
  // agents/packages/agents/src/characters/validator-monitor.ts so
  // downstream tooling has a single source of truth. Keep them in
  // sync if either file changes.
  validator: {
    /** Concentration % (of a single validator) that triggers CRITICAL */
    criticalConcentrationPct: 33,
    /** Concentration % that triggers WARNING */
    warningConcentrationPct: 20,
    /** Minimum composite score (0-1000) for M014 PoA eligibility */
    poaEligibilityScore: 800,
    /** Uptime component weight (of 1000) */
    scoreWeightUptime: 400,
    /** Governance participation component weight (of 1000) */
    scoreWeightGovernance: 350,
    /** Stability component weight (of 1000) */
    scoreWeightStability: 250,
    /** Trailing window for uptime scoring (days) — used for the narrative, not the math */
    uptimeTrailingDays: 30,
    /** Per-commission-change stability penalty */
    stabilityPenaltyCommissionChange: 40,
    /** Per-jailing stability penalty */
    stabilityPenaltyJailing: 100,
    /** Whale movement threshold (REGEN, in uregen) — flags delegation deltas */
    whaleDelegationUregen: "100000000000", // 100,000 REGEN
    /** Nakamoto floor below which WF-VM-03 warns */
    nakamotoWarningFloor: 8,
    /** Nakamoto floor below which WF-VM-03 escalates */
    nakamotoCriticalFloor: 5,
    /** Gini ceiling above which WF-VM-03 warns */
    giniWarningCeiling: 0.65,
  },

  // Agent identity
  agentId: "AGENT-004",
  agentName: "RegenValidatorMonitor",
  governanceLayer: 1 as const,
} as const;

export function validateConfig(): void {
  if (!config.anthropicApiKey) {
    throw new Error(
      "ANTHROPIC_API_KEY is required. Copy .env.example to .env and set it."
    );
  }
}
