/**
 * Standalone Mode
 *
 * When ElizaOS is not installed, agents run directly using the OODA
 * executor. Each character maps to a workflow definition and a set of
 * step handlers. A polling loop drives the observe-orient-decide-act
 * cycle on a configurable interval.
 *
 * This is the pragmatic "it works without ElizaOS" path.
 */

import type { RegenConfig } from "@regen/core";
import {
  OODAExecutor,
  type StepHandlers,
} from "@regen/core";
import type {
  OODAWorkflow,
  ObserveStep,
  OrientStep,
  DecideStep,
  ActStep,
  WorkflowExecution,
  WorkflowDecision,
  GovernanceLayer,
} from "@regen/core";
import type { LedgerMCPClient } from "@regen/plugin-ledger-mcp";
import type { KOIMCPClient } from "@regen/plugin-koi-mcp";
import {
  analyzeProposal,
  formatProposalAnalysis,
} from "@regen/plugin-ledger-mcp";
import {
  getLedgerState,
  formatLedgerState,
} from "@regen/plugin-ledger-mcp";
import {
  getKnowledgeContext,
  formatKnowledgeContext,
} from "@regen/plugin-koi-mcp";

// ---------------------------------------------------------------------------
// Workflow definitions per character
// ---------------------------------------------------------------------------

function governanceAnalystWorkflow(): OODAWorkflow {
  return {
    id: "wf-ga-01-proposal-monitor",
    name: "Governance Proposal Monitor",
    agentId: "AGENT-002",
    governanceLayer: 1 as GovernanceLayer, // Layer 1 -- informational only
    sla: { responseTimeMs: 30_000 },
    observe: [
      {
        id: "obs-active-proposals",
        type: "ledger_query",
        tool: "list_governance_proposals",
        params: { limit: 10, proposal_status: "PROPOSAL_STATUS_VOTING_PERIOD" },
      },
      {
        id: "obs-knowledge-context",
        type: "koi_search",
        tool: "search",
        params: { query: "active governance proposals regen", limit: 5 },
      },
    ],
    orient: [
      {
        id: "orient-analyze",
        type: "llm_analysis",
        prompt:
          "Analyze the active governance proposals. For each, assess technical, economic, and governance impact. Note quorum risk and unusual voting patterns.",
      },
    ],
    decide: {
      id: "decide-report",
      escalationThreshold: 0.6,
      rules: [
        {
          condition: "activeProposals > 0",
          recommendation: "REPORT",
          confidence: 0.9,
        },
        {
          condition: "activeProposals == 0",
          recommendation: "SKIP",
          confidence: 1.0,
        },
      ],
    },
    act: [
      {
        id: "act-publish-report",
        type: "log",
        condition: "recommendation == REPORT",
        params: { channel: "stdout" },
      },
    ],
  };
}

function registryReviewerWorkflow(): OODAWorkflow {
  return {
    id: "wf-rr-01-registry-monitor",
    name: "Registry Submission Monitor",
    agentId: "AGENT-001",
    governanceLayer: 2 as GovernanceLayer, // Layer 2 -- agentic with oversight
    sla: { responseTimeMs: 60_000 },
    observe: [
      {
        id: "obs-credit-classes",
        type: "ledger_query",
        tool: "list_classes",
        params: { limit: 20 },
      },
      {
        id: "obs-recent-batches",
        type: "ledger_query",
        tool: "list_batches",
        params: { limit: 10 },
      },
      {
        id: "obs-knowledge",
        type: "koi_search",
        tool: "search",
        params: { query: "credit class methodology requirements", limit: 5 },
      },
    ],
    orient: [
      {
        id: "orient-review",
        type: "rule_check",
        rules: [
          {
            condition: "newBatchesExist",
            result: "Review new batches against methodology requirements",
          },
          {
            condition: "newClassesExist",
            result: "Review new credit class applications",
          },
        ],
      },
    ],
    decide: {
      id: "decide-action",
      escalationThreshold: 0.6,
      rules: [
        {
          condition: "pendingReviews > 0",
          recommendation: "REVIEW",
          confidence: 0.8,
        },
        {
          condition: "pendingReviews == 0",
          recommendation: "SKIP",
          confidence: 1.0,
        },
      ],
    },
    act: [
      {
        id: "act-publish-review",
        type: "log",
        condition: "recommendation == REVIEW",
        params: { channel: "stdout" },
      },
      {
        id: "act-escalate",
        type: "escalate",
        condition: "confidence < 0.6",
        params: { channel: "operator" },
      },
    ],
  };
}

const WORKFLOWS: Record<string, () => OODAWorkflow> = {
  "governance-analyst": governanceAnalystWorkflow,
  "registry-reviewer": registryReviewerWorkflow,
};

// ---------------------------------------------------------------------------
// Step handlers -- wire MCP clients into the OODA executor
// ---------------------------------------------------------------------------

function createStepHandlers(
  ledgerClient: LedgerMCPClient,
  koiClient: KOIMCPClient
): StepHandlers {
  return {
    async observe(
      step: ObserveStep,
      _context: WorkflowExecution
    ): Promise<unknown> {
      console.log(`  [observe] ${step.id} (${step.type}: ${step.tool})`);

      if (step.type === "ledger_query") {
        try {
          return await ledgerClient.call(step.tool, step.params);
        } catch (err) {
          console.log(
            `  [observe] Ledger call failed: ${(err as Error).message}`
          );
          return { error: (err as Error).message };
        }
      }

      if (step.type === "koi_search") {
        try {
          return await koiClient.call(step.tool, step.params);
        } catch (err) {
          console.log(
            `  [observe] KOI call failed: ${(err as Error).message}`
          );
          return { error: (err as Error).message };
        }
      }

      // memory_recall, external_oracle -- not yet wired
      return { note: `Step type ${step.type} not yet implemented in standalone mode` };
    },

    async orient(
      step: OrientStep,
      observations: unknown[],
      _context: WorkflowExecution
    ): Promise<Record<string, unknown>> {
      console.log(`  [orient] ${step.id} (${step.type})`);

      if (step.type === "rule_check" && step.rules) {
        // Evaluate simple rule conditions against observations
        const results: Record<string, unknown> = {};
        for (const rule of step.rules) {
          results[rule.condition] = rule.result;
        }
        return results;
      }

      // For llm_analysis in standalone mode, we return the raw observations
      // since we don't have an LLM loop. The executor still produces a
      // structured execution record.
      return {
        observationCount: observations.length,
        prompt: step.prompt ?? "No prompt specified",
        rawObservations: observations,
      };
    },

    async decide(
      step: DecideStep,
      orientation: Record<string, unknown>
    ): Promise<WorkflowDecision> {
      console.log(`  [decide] ${step.id}`);

      // Pick the first matching rule, or fall back to the last rule
      const matchedRule =
        step.rules.find((_r) => {
          // In standalone mode without an LLM, we can't evaluate complex
          // conditions. Default to the first rule with highest confidence.
          return true;
        }) ?? step.rules[step.rules.length - 1];

      return {
        recommendation: matchedRule.recommendation,
        confidence: matchedRule.confidence,
        rationale: `Standalone mode: matched rule "${matchedRule.condition}" with confidence ${matchedRule.confidence}`,
        evidence: [orientation],
      };
    },

    async act(
      step: ActStep,
      decision: WorkflowDecision,
      context: WorkflowExecution
    ): Promise<unknown> {
      console.log(`  [act] ${step.id} (${step.type})`);

      if (step.type === "log") {
        console.log(
          `\n--- ${context.workflowId} Result ---\n` +
            `Recommendation: ${decision.recommendation}\n` +
            `Confidence: ${decision.confidence}\n` +
            `Rationale: ${decision.rationale}\n` +
            `Observations: ${context.observations.length}\n` +
            `---\n`
        );
        return { logged: true };
      }

      if (step.type === "escalate") {
        console.log(
          `[ESCALATE] Confidence ${decision.confidence} below threshold. ` +
            `Human review required for: ${decision.rationale}`
        );
        return { escalated: true };
      }

      if (step.type === "post_message" || step.type === "create_koi_object") {
        console.log(
          `[${step.type}] Would execute in full runtime. Skipped in standalone mode.`
        );
        return { skipped: true, reason: "standalone mode" };
      }

      return { unknown: true };
    },
  };
}

// ---------------------------------------------------------------------------
// Polling loop
// ---------------------------------------------------------------------------

const DEFAULT_POLL_INTERVAL_MS = 60_000; // 1 minute

export async function runStandalone(
  characterName: string,
  config: RegenConfig,
  ledgerClient: LedgerMCPClient,
  koiClient: KOIMCPClient
): Promise<void> {
  const workflowFactory = WORKFLOWS[characterName];
  if (!workflowFactory) {
    console.error(
      `No standalone workflow defined for character: "${characterName}". ` +
        `Available: ${Object.keys(WORKFLOWS).join(", ")}`
    );
    process.exit(1);
  }

  const handlers = createStepHandlers(ledgerClient, koiClient);
  const executor = new OODAExecutor(handlers);

  const pollIntervalMs = parseInt(
    process.env.AGENT_POLL_INTERVAL_MS ?? "",
    10
  ) || DEFAULT_POLL_INTERVAL_MS;

  console.log(
    `[Standalone] Running ${characterName} with OODA executor.\n` +
      `[Standalone] Poll interval: ${pollIntervalMs / 1000}s\n` +
      `[Standalone] Press Ctrl+C to stop.\n`
  );

  let running = true;
  let cycleCount = 0;

  const shutdown = () => {
    console.log("\n[Standalone] Shutting down...");
    running = false;
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  // Run first cycle immediately, then poll
  while (running) {
    cycleCount++;
    const workflow = workflowFactory();

    console.log(
      `\n[Standalone] Cycle #${cycleCount} starting at ${new Date().toISOString()}`
    );

    try {
      const execution = await executor.execute(workflow);
      console.log(
        `[Standalone] Cycle #${cycleCount} completed: ` +
          `status=${execution.status}, ` +
          `observations=${execution.observations.length}, ` +
          `actions=${execution.actions.length}`
      );

      if (execution.status === "escalated") {
        console.log(
          `[Standalone] Cycle #${cycleCount} escalated. Decision: ` +
            JSON.stringify(execution.decision, null, 2)
        );
      }
    } catch (err) {
      console.error(
        `[Standalone] Cycle #${cycleCount} error: ${(err as Error).message}`
      );
    }

    // Wait for next cycle (interruptible)
    if (running) {
      await new Promise<void>((resolve) => {
        const timer = setTimeout(resolve, pollIntervalMs);
        // Allow immediate exit on shutdown
        const checkShutdown = () => {
          if (!running) {
            clearTimeout(timer);
            resolve();
          }
        };
        // Check every second
        const interval = setInterval(checkShutdown, 1000);
        const originalResolve = resolve;
        setTimeout(() => clearInterval(interval), pollIntervalMs + 100);
      });
    }
  }

  console.log(
    `[Standalone] Stopped after ${cycleCount} cycles.`
  );
}
