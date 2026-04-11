import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { screenBatch } from "../reviewer.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type { CreditBatch, Project, ScreeningResult } from "../types.js";

/**
 * WF-RR-03: Batch Review
 *
 * Trigger: New credit batch issued on-chain
 * Layer: 1 (fully automated, informational only)
 * SLA: Review within 2 hours of batch issuance
 *
 * OODA:
 *   Observe — Fetch recent credit batches from ledger
 *   Orient  — Filter to unreviewed batches, resolve parent projects
 *   Decide  — Review each via Claude (amount, dates, project alignment, anomalies)
 *   Act     — Persist results, flag anomalies
 */

interface Observations {
  batches: CreditBatch[];
  projectMap: Map<string, Project>;
  unreviewedDenoms: string[];
}

interface Orientation {
  batchesToReview: {
    batchData: CreditBatch;
    projectData: Project | null;
  }[];
}

interface Decision {
  reviews: {
    batchDenom: string;
    batchLabel: string;
    result: ScreeningResult;
  }[];
}

interface Actions {
  saved: number;
  output: number;
  anomaliesFlagged: number;
}

export function createBatchReviewWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-RR-03",
    name: "Batch Review",

    async observe(): Promise<Observations> {
      const batches = await ledger.getCreditBatches();

      const unreviewedDenoms = batches
        .map((b) => b.denom)
        .filter((denom) => !store.hasBatchScreening(denom));

      // Resolve parent projects for unreviewed batches
      const projectIdsNeeded = new Set(
        batches
          .filter((b) => unreviewedDenoms.includes(b.denom))
          .map((b) => b.project_id)
      );

      const projectMap = new Map<string, Project>();
      await Promise.all(
        [...projectIdsNeeded].map(async (projectId) => {
          try {
            const proj = await ledger.getProject(projectId);
            if (proj) projectMap.set(projectId, proj);
          } catch {
            // Project not found — will review without it
          }
        })
      );

      return { batches, projectMap, unreviewedDenoms };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const batchesToReview = obs.unreviewedDenoms
        .map((denom) => {
          const batchData = obs.batches.find((b) => b.denom === denom);
          if (!batchData) return null;
          return {
            batchData,
            projectData: obs.projectMap.get(batchData.project_id) || null,
          };
        })
        .filter(Boolean) as Orientation["batchesToReview"];

      return { batchesToReview };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const reviews: Decision["reviews"] = [];

      for (const { batchData, projectData } of orientation.batchesToReview) {
        console.log(
          `  Reviewing batch ${batchData.denom} (project: ${batchData.project_id}, amount: ${batchData.total_amount})`
        );

        const result = await screenBatch(batchData, projectData);

        reviews.push({
          batchDenom: batchData.denom,
          batchLabel: `${batchData.denom} (${batchData.total_amount} credits)`,
          result,
        });
      }

      return { reviews };
    },

    async act(decision: Decision): Promise<Actions> {
      let saved = 0;
      let outputCount = 0;
      let anomaliesFlagged = 0;

      for (const { batchDenom, batchLabel, result } of decision.reviews) {
        // Persist
        store.saveBatchScreening(batchDenom, JSON.stringify(result));
        saved++;

        // Determine alert level — batches get CRITICAL for REJECT
        // because issuance of bad credits is an immediate concern
        const alertLevel =
          result.recommendation === "REJECT"
            ? "CRITICAL" as const
            : result.recommendation === "CONDITIONAL"
              ? "HIGH" as const
              : "NORMAL" as const;

        if (alertLevel !== "NORMAL") {
          anomaliesFlagged++;
        }

        // Output
        await output({
          workflow: "WF-RR-03",
          entityId: batchDenom,
          title: `Batch: ${batchLabel} — ${result.recommendation}`,
          content: formatScreeningOutput(result),
          alertLevel,
          timestamp: new Date(),
        });
        outputCount++;
      }

      if (decision.reviews.length === 0) {
        console.log("  No new batches to review.");
      }

      return { saved, output: outputCount, anomaliesFlagged };
    },
  };
}

function formatScreeningOutput(result: ScreeningResult): string {
  return `## Screening Result

**Score:** ${result.score}/1000 | **Confidence:** ${result.confidence}/1000 | **Recommendation:** ${result.recommendation}

### Factor Breakdown
| Factor | Score | Weight |
|--------|-------|--------|
| Methodology Quality | ${result.factors.methodology_quality}/1000 | 40% |
| Reputation | ${result.factors.reputation}/1000 | 30% |
| Novelty | ${result.factors.novelty}/1000 | 20% |
| Completeness | ${result.factors.completeness}/1000 | 10% |

### Rationale
${result.rationale}`;
}
