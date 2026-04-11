/**
 * Agent Entrypoint
 *
 * Bootstraps an ElizaOS agent runtime with the selected character
 * and Regen MCP plugins. Selected via AGENT_CHARACTER env var.
 *
 * Usage:
 *   AGENT_CHARACTER=governance-analyst tsx packages/agents/src/index.ts
 *   AGENT_CHARACTER=registry-reviewer tsx packages/agents/src/index.ts
 *   AGENT_CHARACTER=market-monitor    tsx packages/agents/src/index.ts
 *   AGENT_CHARACTER=validator-monitor  tsx packages/agents/src/index.ts
 */

import { loadConfig } from "@regen/core";
import { LedgerMCPClient } from "@regen/plugin-ledger-mcp";
import { KOIMCPClient } from "@regen/plugin-koi-mcp";
import { governanceAnalystCharacter } from "./characters/governance-analyst.js";
import { registryReviewerCharacter } from "./characters/registry-reviewer.js";
import { marketMonitorCharacter } from "./characters/market-monitor.js";
import { validatorMonitorCharacter } from "./characters/validator-monitor.js";

const CHARACTERS: Record<string, unknown> = {
  "governance-analyst": governanceAnalystCharacter,
  "registry-reviewer": registryReviewerCharacter,
  "market-monitor": marketMonitorCharacter,
  "validator-monitor": validatorMonitorCharacter,
};

async function main() {
  const config = loadConfig();

  const characterName = config.agentCharacter;
  const character = CHARACTERS[characterName];
  if (!character) {
    console.error(
      `Unknown character: "${characterName}". Available: ${Object.keys(CHARACTERS).join(", ")}`
    );
    process.exit(1);
  }

  // Initialize MCP clients
  const ledgerClient = new LedgerMCPClient({
    baseUrl: config.ledgerMcp.baseUrl,
    apiKey: config.ledgerMcp.apiKey,
  });

  const koiClient = new KOIMCPClient({
    baseUrl: config.koiMcp.baseUrl,
    apiKey: config.koiMcp.apiKey,
  });

  console.log(`
╔══════════════════════════════════════════════════════════╗
║          Regen Agentic Tokenomics - Agent Runtime        ║
╠══════════════════════════════════════════════════════════╣
║  Character:   ${characterName.padEnd(41)}║
║  Ledger MCP:  ${config.ledgerMcp.baseUrl.padEnd(41)}║
║  KOI MCP:     ${config.koiMcp.baseUrl.padEnd(41)}║
╚══════════════════════════════════════════════════════════╝
`);

  // ---------------------------------------------------------------
  // ElizaOS Runtime Bootstrap
  //
  // This is where you'd wire up the full ElizaOS runtime:
  //
  //   import { AgentRuntime } from "@elizaos/core";
  //   import { bootstrapPlugin } from "@elizaos/plugin-bootstrap";
  //
  //   const runtime = new AgentRuntime({
  //     character,
  //     token: config.anthropicApiKey,
  //     modelProvider: "anthropic",
  //     plugins: [bootstrapPlugin, ledgerPlugin, koiPlugin],
  //     databaseAdapter: db,
  //     cacheManager: cache,
  //   });
  //   await runtime.initialize();
  //
  // For now, we verify the MCP connections and demonstrate the
  // agent's core capability: analyzing a governance proposal.
  // ---------------------------------------------------------------

  console.log("Verifying MCP connections...\n");

  // Test Ledger MCP
  try {
    const proposals = await ledgerClient.listProposals({ limit: 1 });
    console.log(
      "[Ledger MCP] Connected. Sample response:",
      JSON.stringify(proposals, null, 2).substring(0, 200)
    );
  } catch (err) {
    console.log(
      `[Ledger MCP] Not available (${(err as Error).message}). ` +
        "This is expected if the MCP server isn't running locally."
    );
  }

  // Test KOI MCP
  try {
    const results = await koiClient.search({
      query: "regen governance",
      limit: 1,
    });
    console.log(
      "[KOI MCP] Connected. Sample response:",
      JSON.stringify(results, null, 2).substring(0, 200)
    );
  } catch (err) {
    console.log(
      `[KOI MCP] Not available (${(err as Error).message}). ` +
        "This is expected if the MCP server isn't running locally."
    );
  }

  console.log("\nAgent scaffold initialized successfully.");
  console.log(
    "To run with full ElizaOS runtime, install @elizaos/core and uncomment the bootstrap section above."
  );
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
