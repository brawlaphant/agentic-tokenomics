import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { screenCreditClass } from "../reviewer.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type { CreditClass, ScreeningResult } from "../types.js";

/**
 * WF-RR-01: Credit Class Screening
 *
 * Trigger: New credit class registered on-chain
 * Layer: 1 (fully automated, informational only)
 * SLA: Screening within 2 hours of class creation
 *
 * OODA:
 *   Observe — Fetch all credit classes and their issuers from ledger
 *   Orient  — Filter to unscreened classes
 *   Decide  — Screen each via Claude (methodology, reputation, novelty, completeness)
 *   Act     — Persist results, output recommendations
 */

interface Observations {
  classes: CreditClass[];
  issuersMap: Map<string, string[]>;
  unscreenedIds: string[];
}

interface Orientation {
  classesToScreen: {
    classData: CreditClass;
    issuers: string[];
  }[];
  allClasses: CreditClass[];
}

interface Decision {
  screenings: {
    classId: string;
    className: string;
    result: ScreeningResult;
  }[];
}

interface Actions {
  saved: number;
  output: number;
}

export function createClassScreeningWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-RR-01",
    name: "Credit Class Screening",

    async observe(): Promise<Observations> {
      const classes = await ledger.getCreditClasses();

      // Fetch issuers for unscreened classes
      const unscreenedIds = classes
        .map((c) => c.id)
        .filter((id) => !store.hasClassScreening(id));

      const issuersMap = new Map<string, string[]>();
      await Promise.all(
        unscreenedIds.map(async (id) => {
          try {
            const issuers = await ledger.getClassIssuers(id);
            issuersMap.set(id, issuers);
          } catch {
            issuersMap.set(id, []);
          }
        })
      );

      return { classes, issuersMap, unscreenedIds };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const classesToScreen = obs.unscreenedIds
        .map((id) => {
          const classData = obs.classes.find((c) => c.id === id);
          if (!classData) return null;
          return {
            classData,
            issuers: obs.issuersMap.get(id) || [],
          };
        })
        .filter(Boolean) as Orientation["classesToScreen"];

      return { classesToScreen, allClasses: obs.classes };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const screenings: Decision["screenings"] = [];

      for (const { classData, issuers } of orientation.classesToScreen) {
        console.log(
          `  Screening credit class ${classData.id}: ${classData.credit_type?.name || "unknown"}`
        );

        const result = await screenCreditClass(
          classData,
          issuers,
          orientation.allClasses
        );

        screenings.push({
          classId: classData.id,
          className: classData.credit_type?.name || classData.id,
          result,
        });
      }

      return { screenings };
    },

    async act(decision: Decision): Promise<Actions> {
      let saved = 0;
      let outputCount = 0;

      for (const { classId, className, result } of decision.screenings) {
        // Persist
        store.saveClassScreening(classId, JSON.stringify(result));
        saved++;

        // Determine alert level based on recommendation
        const alertLevel =
          result.recommendation === "REJECT"
            ? "CRITICAL" as const
            : result.recommendation === "CONDITIONAL"
              ? "HIGH" as const
              : "NORMAL" as const;

        // Output
        await output({
          workflow: "WF-RR-01",
          entityId: classId,
          title: `Credit Class: ${className} — ${result.recommendation}`,
          content: formatScreeningOutput(result),
          alertLevel,
          timestamp: new Date(),
        });
        outputCount++;
      }

      if (decision.screenings.length === 0) {
        console.log("  No new credit classes to screen.");
      }

      return { saved, output: outputCount };
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
