/**
 * Environment configuration loader.
 *
 * Reads from process.env with sensible defaults for local development.
 * Follows the feasibility review recommendation to separate config from code.
 */

export interface RegenConfig {
  anthropicApiKey: string;
  ledgerMcp: {
    baseUrl: string;
    apiKey: string;
  };
  koiMcp: {
    baseUrl: string;
    apiKey: string;
  };
  databaseUrl: string;
  redisUrl: string;
  agentCharacter: string;
}

export function loadConfig(): RegenConfig {
  return {
    anthropicApiKey: requireEnv("ANTHROPIC_API_KEY"),
    ledgerMcp: {
      baseUrl: process.env.LEDGER_MCP_URL ?? "http://localhost:3001",
      apiKey: process.env.LEDGER_MCP_API_KEY ?? "",
    },
    koiMcp: {
      baseUrl: process.env.KOI_MCP_URL ?? "http://localhost:3002",
      apiKey: process.env.KOI_MCP_API_KEY ?? "",
    },
    databaseUrl:
      process.env.DATABASE_URL ??
      "postgresql://postgres:postgres@localhost:5432/regen_agents",
    redisUrl: process.env.REDIS_URL ?? "redis://localhost:6379",
    agentCharacter: process.env.AGENT_CHARACTER ?? "governance-analyst",
  };
}

export function requireEnv(key: string): string {
  const value = process.env[key];
  if (!value) {
    throw new Error(
      `Missing required environment variable: ${key}. See .env.example.`
    );
  }
  return value;
}
