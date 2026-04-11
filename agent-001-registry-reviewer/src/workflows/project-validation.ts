import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { screenProject } from "../reviewer.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type { Project, CreditClass, ScreeningResult } from "../types.js";

/**
 * WF-RR-02: Project Validation
 *
 * Trigger: New project registered on-chain
 * Layer: 1 (fully automated, informational only)
 * SLA: Validation within 2 hours of project registration
 *
 * OODA:
 *   Observe — Fetch all projects and their parent credit classes from ledger
 *   Orient  — Filter to unscreened projects, resolve parent class data
 *   Decide  — Screen each via Claude (class alignment, jurisdiction, metadata)
 *   Act     — Persist results, output recommendations
 */

interface Observations {
  projects: Project[];
  classMap: Map<string, CreditClass>;
  unscreenedIds: string[];
}

interface Orientation {
  projectsToScreen: {
    projectData: Project;
    classData: CreditClass | null;
  }[];
}

interface Decision {
  screenings: {
    projectId: string;
    projectName: string;
    result: ScreeningResult;
  }[];
}

interface Actions {
  saved: number;
  output: number;
}

export function createProjectValidationWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-RR-02",
    name: "Project Validation",

    async observe(): Promise<Observations> {
      const projects = await ledger.getProjects();

      const unscreenedIds = projects
        .map((p) => p.id)
        .filter((id) => !store.hasProjectScreening(id));

      // Resolve parent classes for unscreened projects
      const classIdsNeeded = new Set(
        projects
          .filter((p) => unscreenedIds.includes(p.id))
          .map((p) => p.class_id)
      );

      const classMap = new Map<string, CreditClass>();
      await Promise.all(
        [...classIdsNeeded].map(async (classId) => {
          try {
            const cls = await ledger.getCreditClass(classId);
            if (cls) classMap.set(classId, cls);
          } catch {
            // Class not found — will screen without it
          }
        })
      );

      return { projects, classMap, unscreenedIds };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const projectsToScreen = obs.unscreenedIds
        .map((id) => {
          const projectData = obs.projects.find((p) => p.id === id);
          if (!projectData) return null;
          return {
            projectData,
            classData: obs.classMap.get(projectData.class_id) || null,
          };
        })
        .filter(Boolean) as Orientation["projectsToScreen"];

      return { projectsToScreen };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const screenings: Decision["screenings"] = [];

      for (const { projectData, classData } of orientation.projectsToScreen) {
        console.log(
          `  Screening project ${projectData.id} (class: ${projectData.class_id})`
        );

        const result = await screenProject(projectData, classData);

        screenings.push({
          projectId: projectData.id,
          projectName: projectData.id,
          result,
        });
      }

      return { screenings };
    },

    async act(decision: Decision): Promise<Actions> {
      let saved = 0;
      let outputCount = 0;

      for (const { projectId, projectName, result } of decision.screenings) {
        // Persist
        store.saveProjectScreening(projectId, JSON.stringify(result));
        saved++;

        // Determine alert level
        const alertLevel =
          result.recommendation === "REJECT"
            ? "CRITICAL" as const
            : result.recommendation === "CONDITIONAL"
              ? "HIGH" as const
              : "NORMAL" as const;

        // Output
        await output({
          workflow: "WF-RR-02",
          entityId: projectId,
          title: `Project: ${projectName} — ${result.recommendation}`,
          content: formatScreeningOutput(result),
          alertLevel,
          timestamp: new Date(),
        });
        outputCount++;
      }

      if (decision.screenings.length === 0) {
        console.log("  No new projects to screen.");
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
