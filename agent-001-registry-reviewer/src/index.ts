#!/usr/bin/env node
import { config, validateConfig } from "./config.js";
import { LedgerClient } from "./ledger.js";
import { executeOODA } from "./ooda.js";
import { store } from "./store.js";
import { createClassScreeningWorkflow } from "./workflows/class-screening.js";
import { createProjectValidationWorkflow } from "./workflows/project-validation.js";
import { createBatchReviewWorkflow } from "./workflows/batch-review.js";

// ── Banner ────────────────────────────────────────────────────

function banner() {
  console.log(`
  ╔══════════════════════════════════════════════════════════════╗
  ║            REGEN REGISTRY REVIEWER (AGENT-001)               ║
  ║                                                              ║
  ║  Layer 1 — Fully Automated, Informational Only               ║
  ║  Workflows: WF-RR-01, WF-RR-02, WF-RR-03                    ║
  ║                                                              ║
  ║  Regen Agentic Tokenomics Framework                          ║
  ╚══════════════════════════════════════════════════════════════╝
`);
}

// ── Main loop ─────────────────────────────────────────────────

async function runCycle(ledger: LedgerClient): Promise<void> {
  const ts = new Date().toISOString();
  console.log(`\n[${ts}] ═══ Starting registry review cycle ═══\n`);

  // WF-RR-01: Screen new credit classes
  const wf01 = createClassScreeningWorkflow(ledger);
  await executeOODA(wf01);

  // WF-RR-02: Validate new projects
  const wf02 = createProjectValidationWorkflow(ledger);
  await executeOODA(wf02);

  // WF-RR-03: Review new credit batches
  const wf03 = createBatchReviewWorkflow(ledger);
  await executeOODA(wf03);

  const execCount = store.getExecutionCount();
  console.log(
    `[${new Date().toISOString()}] ═══ Cycle complete (${execCount} total executions logged) ═══\n`
  );
}

async function main() {
  banner();
  validateConfig();

  const runOnce = process.argv.includes("--once");
  const ledger = new LedgerClient();

  console.log(`Configuration:`);
  console.log(`  LCD endpoint: ${config.lcdUrl}`);
  console.log(`  LLM model:    ${config.model}`);
  console.log(`  Discord:      ${config.discordWebhookUrl ? "configured" : "not configured"}`);
  console.log(`  KOI MCP:      ${config.koiMcpUrl ? "configured" : "not configured"}`);
  console.log(`  Mode:         ${runOnce ? "single run" : `polling every ${config.pollIntervalMs / 1000}s`}`);
  console.log();

  // Verify LCD connectivity
  try {
    const { blockHeight } = await ledger.checkConnection();
    console.log(
      `Connected to Regen Ledger. Latest block: ${blockHeight}\n`
    );
  } catch (err) {
    console.error(
      `Failed to connect to Regen Ledger at ${config.lcdUrl}:`,
      err
    );
    process.exit(1);
  }

  if (runOnce) {
    // Single run mode (e.g., `npm run analyze`)
    await runCycle(ledger);
  } else {
    // Polling mode
    await runCycle(ledger);

    const interval = setInterval(() => {
      runCycle(ledger).catch((err) =>
        console.error(`Cycle failed:`, err)
      );
    }, config.pollIntervalMs);

    // Graceful shutdown
    const shutdown = () => {
      console.log("\nShutting down gracefully...");
      clearInterval(interval);
      store.close();
      process.exit(0);
    };

    process.on("SIGINT", shutdown);
    process.on("SIGTERM", shutdown);

    console.log("Agent running. Press Ctrl+C to stop.\n");
  }
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
