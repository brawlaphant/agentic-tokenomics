export const config = {
  // Regen LCD endpoint
  lcdUrl: process.env.REGEN_LCD_URL || "https://regen.api.chandrastation.com",

  // Anthropic
  anthropicApiKey: process.env.ANTHROPIC_API_KEY || "",
  model: process.env.ANTHROPIC_MODEL || "claude-sonnet-4-5-20250929",

  // Discord webhook (optional)
  discordWebhookUrl: process.env.DISCORD_WEBHOOK_URL || "",

  // Polling — market monitor runs faster than governance analyst
  // because trade/retirement events move quickly.
  pollIntervalMs: (parseInt(process.env.POLL_INTERVAL_SECONDS || "300", 10)) * 1000,

  // Market monitor thresholds. These mirror the character thresholds in
  // agents/packages/agents/src/characters/market-monitor.ts so downstream
  // tooling has a single source of truth. Keep them in sync if either
  // file changes.
  market: {
    /** z-score at or above which anomaly severity becomes CRITICAL */
    criticalZScore: 3.5,
    /** z-score at which anomaly is flagged for watchlist (WARNING) */
    warningZScore: 2.0,
    /** minimum sample size before computing a z-score */
    minSamples: 5,
    /** trailing window for class median (days) */
    classMedianWindowDays: 30,
    /** trailing window for batch median (days) */
    batchMedianWindowDays: 30,
    /** liquidity depth threshold (USD) below which health is DEGRADED */
    liquidityDepthFloor: 5_000,
    /** large trade volume (USD) that triggers an off-cycle liquidity report */
    largeTradeVolumeUsd: 10_000,
    /** retirement tx window (hours) for demand signal extraction */
    retirementWindowHours: 168, // 7 days
  },

  // Agent identity
  agentId: "AGENT-003",
  agentName: "RegenMarketMonitor",
  governanceLayer: 1 as const,
} as const;

export function validateConfig(): void {
  if (!config.anthropicApiKey) {
    throw new Error(
      "ANTHROPIC_API_KEY is required. Copy .env.example to .env and set it."
    );
  }
}
