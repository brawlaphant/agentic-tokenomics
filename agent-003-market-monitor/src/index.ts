#!/usr/bin/env node
import { config, validateConfig } from "./config.js";
import { LedgerClient } from "./ledger.js";
import { executeOODA } from "./ooda.js";
import { store } from "./store.js";
import { createPriceAnomalyDetectionWorkflow } from "./workflows/price-anomaly-detection.js";
import { createLiquidityMonitorWorkflow } from "./workflows/liquidity-monitor.js";
import { createRetirementTrackingWorkflow } from "./workflows/retirement-tracking.js";

// ── Banner ────────────────────────────────────────────────────

function banner() {
  console.log(`
  ╔══════════════════════════════════════════════════════════════╗
  ║              REGEN MARKET MONITOR (AGENT-003)                ║
  ║                                                              ║
  ║  Layer 1 — Fully Automated, Informational Only               ║
  ║  Workflows: WF-MM-01, WF-MM-02, WF-MM-03                     ║
  ║                                                              ║
  ║  Regen Agentic Tokenomics Framework                          ║
  ╚══════════════════════════════════════════════════════════════╝
`);
}

// ── Main loop ─────────────────────────────────────────────────

async function runCycle(ledger: LedgerClient): Promise<void> {
  const ts = new Date().toISOString();
  console.log(`\n[${ts}] ═══ Starting market monitor cycle ═══\n`);

  // WF-MM-01: Detect price anomalies in open sell orders
  const wf01 = createPriceAnomalyDetectionWorkflow(ledger);
  await executeOODA(wf01);

  // WF-MM-02: Snapshot liquidity health per credit class
  const wf02 = createLiquidityMonitorWorkflow(ledger);
  await executeOODA(wf02);

  // WF-MM-03: Summarize retirement activity + demand signal
  const wf03 = createRetirementTrackingWorkflow(ledger);
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
  console.log(`  Mode:         ${runOnce ? "single run" : `polling every ${config.pollIntervalMs / 1000}s`}`);
  console.log();

  try {
    const { blockHeight } = await ledger.checkConnection();
    console.log(`Connected to Regen Ledger at block ${blockHeight}\n`);
  } catch (err) {
    console.error(
      `Failed to connect to Regen Ledger at ${config.lcdUrl}:`,
      err
    );
    process.exit(1);
  }

  if (runOnce) {
    await runCycle(ledger);
  } else {
    await runCycle(ledger);

    const interval = setInterval(() => {
      runCycle(ledger).catch((err) =>
        console.error(`Cycle failed:`, err)
      );
    }, config.pollIntervalMs);

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
