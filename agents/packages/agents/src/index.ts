/**
 * Agent Entrypoint
 *
 * Bootstraps an ElizaOS agent runtime with the selected character
 * and Regen MCP plugins. Falls back to standalone OODA loop mode
 * when ElizaOS is not installed.
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

const CHARACTERS: Record<string, any> = {
  "governance-analyst": governanceAnalystCharacter,
  "registry-reviewer": registryReviewerCharacter,
  "market-monitor": marketMonitorCharacter,
  "validator-monitor": validatorMonitorCharacter,
};

async function verifyMcpConnections(
  ledgerClient: LedgerMCPClient,
  koiClient: KOIMCPClient
): Promise<{ ledger: boolean; koi: boolean }> {
  const result = { ledger: false, koi: false };

  try {
    await ledgerClient.listProposals({ limit: 1 });
    console.log("[Ledger MCP] Connected.");
    result.ledger = true;
  } catch (err) {
    console.log(
      `[Ledger MCP] Not available (${(err as Error).message}). ` +
        "This is expected if the MCP server isn't running locally."
    );
  }

  try {
    await koiClient.search({ query: "regen governance", limit: 1 });
    console.log("[KOI MCP] Connected.");
    result.koi = true;
  } catch (err) {
    console.log(
      `[KOI MCP] Not available (${(err as Error).message}). ` +
        "This is expected if the MCP server isn't running locally."
    );
  }

  return result;
}

async function bootstrapElizaOS(
  character: any,
  config: ReturnType<typeof loadConfig>,
  ledgerClient: LedgerMCPClient,
  koiClient: KOIMCPClient
): Promise<void> {
  // Dynamic import -- only resolves if @elizaos/core is installed.
  // Cast to any because @elizaos/core v1.7 has a moduleResolution quirk
  // where ./runtime re-export doesn't resolve under NodeNext. The class
  // exists at runtime; this cast is safe.
  const elizaCore: any = await import("@elizaos/core");
  const AgentRuntime = elizaCore.AgentRuntime as any;

  // Import plugin objects from our MCP packages
  const { ledgerMcpPlugin } = await import(
    "@regen/plugin-ledger-mcp"
  );
  const { koiMcpPlugin } = await import("@regen/plugin-koi-mcp");

  // Initialize the plugins with MCP client instances
  ledgerMcpPlugin._setClient(ledgerClient);
  koiMcpPlugin._setClient(koiClient);

  // ElizaOS v1.6.3 AgentRuntime constructor:
  //   { character?, plugins?, adapter?, settings?, agentId?, fetch? }
  // API key goes through character.settings.secrets or runtime settings.
  const runtime = new AgentRuntime({
    character: {
      ...character,
      settings: {
        ...character.settings,
        secrets: {
          ...character.settings?.secrets,
          ANTHROPIC_API_KEY: config.anthropicApiKey,
        },
      },
    },
    plugins: [ledgerMcpPlugin as any, koiMcpPlugin as any],
    settings: {
      ANTHROPIC_API_KEY: config.anthropicApiKey,
    } as any,
  });

  await runtime.initialize();

  console.log("[ElizaOS] Runtime initialized successfully.");
  console.log(
    `[ElizaOS] Plugins loaded: ${[ledgerMcpPlugin.name, koiMcpPlugin.name].join(", ")}`
  );

  // Graceful shutdown
  const shutdown = async () => {
    console.log("\n[ElizaOS] Shutting down...");
    try {
      if (typeof (runtime as any).close === "function") {
        await (runtime as any).close();
      }
    } catch {
      // best-effort cleanup
    }
    process.exit(0);
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  // Keep process alive -- the runtime manages its own event loop
  console.log("[ElizaOS] Runtime running. Press Ctrl+C to stop.");
}

async function bootstrapStandalone(
  characterName: string,
  config: ReturnType<typeof loadConfig>,
  ledgerClient: LedgerMCPClient,
  koiClient: KOIMCPClient
): Promise<void> {
  const { runStandalone } = await import("./standalone.js");
  await runStandalone(characterName, config, ledgerClient, koiClient);
}

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
\u2554\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2557
\u2551          Regen Agentic Tokenomics - Agent Runtime        \u2551
\u2560\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2563
\u2551  Character:   ${characterName.padEnd(41)}\u2551
\u2551  Ledger MCP:  ${config.ledgerMcp.baseUrl.padEnd(41)}\u2551
\u2551  KOI MCP:     ${config.koiMcp.baseUrl.padEnd(41)}\u2551
\u255a\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u255d
`);

  // Verify MCP connections before any runtime bootstrap
  console.log("Verifying MCP connections...\n");
  await verifyMcpConnections(ledgerClient, koiClient);
  console.log();

  // Attempt ElizaOS runtime; fall back to standalone OODA loop
  try {
    await bootstrapElizaOS(character, config, ledgerClient, koiClient);
  } catch (err) {
    const isModuleNotFound =
      err instanceof Error &&
      (err.message.includes("Cannot find module") ||
        err.message.includes("Cannot find package") ||
        err.message.includes("MODULE_NOT_FOUND") ||
        err.message.includes("ERR_MODULE_NOT_FOUND"));

    if (isModuleNotFound) {
      console.log(
        "[ElizaOS] @elizaos/core not installed. Falling back to standalone OODA mode.\n" +
          "  To enable ElizaOS runtime: npm install @elizaos/core\n"
      );
      await bootstrapStandalone(
        characterName,
        config,
        ledgerClient,
        koiClient
      );
    } else {
      throw err;
    }
  }
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
