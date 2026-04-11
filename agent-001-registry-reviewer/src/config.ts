export const config = {
  // Regen LCD endpoint
  lcdUrl: process.env.REGEN_LCD_URL || "https://regen.api.chandrastation.com",

  // Anthropic
  anthropicApiKey: process.env.ANTHROPIC_API_KEY || "",
  model: process.env.ANTHROPIC_MODEL || "claude-sonnet-4-5-20250929",

  // Discord webhook (optional)
  discordWebhookUrl: process.env.DISCORD_WEBHOOK_URL || "",

  // KOI MCP endpoint (optional)
  koiMcpUrl: process.env.KOI_MCP_URL || "",

  // Polling
  pollIntervalMs:
    parseInt(process.env.POLL_INTERVAL_SECONDS || "300", 10) * 1000,

  // Screening thresholds
  screening: {
    approveThreshold: 700,     // score >= 700 → APPROVE
    rejectThreshold: 300,      // score < 300 → REJECT
    // Between 300–699 → CONDITIONAL
  },

  // Factor weights (must sum to 1.0)
  weights: {
    methodology_quality: 0.40,
    reputation: 0.30,
    novelty: 0.20,
    completeness: 0.10,
  },

  // Agent identity
  agentId: "AGENT-001",
  agentName: "RegenRegistryReviewer",
  governanceLayer: 1 as const,
} as const;

export function validateConfig(): void {
  if (!config.anthropicApiKey) {
    throw new Error(
      "ANTHROPIC_API_KEY is required. Copy .env.example to .env and set it."
    );
  }
}
