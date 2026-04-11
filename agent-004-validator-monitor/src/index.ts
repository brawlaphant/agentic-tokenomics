#!/usr/bin/env node
import { config, validateConfig } from "./config.js";
import { LedgerClient } from "./ledger.js";
import { executeOODA } from "./ooda.js";
import { store } from "./store.js";
import { createPerformanceTrackingWorkflow } from "./workflows/performance-tracking.js";
import { createDelegationFlowAnalysisWorkflow } from "./workflows/delegation-flow-analysis.js";
import { createDecentralizationMonitorWorkflow } from "./workflows/decentralization-monitor.js";

// ── Banner ────────────────────────────────────────────────────

function banner() {
  console.log(`
  ╔══════════════════════════════════════════════════════════════╗
  ║             REGEN VALIDATOR MONITOR (AGENT-004)              ║
  ║                                                              ║
  ║  Layer 1 — Fully Automated, Informational Only               ║
  ║  Workflows: WF-VM-01, WF-VM-02, WF-VM-03                     ║
  ║                                                              ║
  ║  Regen Agentic Tokenomics Framework                          ║
  ╚══════════════════════════════════════════════════════════════╝
`);
}

// ── Main loop ─────────────────────────────────────────────────

async function runCycle(ledger: LedgerClient): Promise<void> {
  const ts = new Date().toISOString();
  console.log(`\n[${ts}] ═══ Starting validator monitor cycle ═══\n`);

  // WF-VM-01: Performance tracking + alerts
  const wf01 = createPerformanceTrackingWorkflow(ledger);
  await executeOODA(wf01);

  // WF-VM-02: Delegation flow analysis
  const wf02 = createDelegationFlowAnalysisWorkflow(ledger);
  await executeOODA(wf02);

  // WF-VM-03: Decentralization monitoring
  const wf03 = createDecentralizationMonitorWorkflow(ledger);
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
    // Recursive setTimeout rather than setInterval so a slow cycle
    // never overlaps with the next tick. If a cycle takes longer
    // than pollIntervalMs the next tick simply starts late — we
    // never have two runCycle invocations sharing the SQLite
    // connection in parallel.
    let timeoutId: NodeJS.Timeout | null = null;
    let stopping = false;

    const runNext = () => {
      runCycle(ledger)
        .catch((err) => console.error(`Cycle failed:`, err))
        .finally(() => {
          if (!stopping) {
            timeoutId = setTimeout(runNext, config.pollIntervalMs);
          }
        });
    };

    runNext();

    const shutdown = () => {
      console.log("\nShutting down gracefully...");
      stopping = true;
      if (timeoutId !== null) clearTimeout(timeoutId);
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
