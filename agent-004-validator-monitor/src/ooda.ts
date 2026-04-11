import { config } from "./config.js";
import { store } from "./store.js";
import type { OODAExecution } from "./types.js";

/**
 * Generic OODA loop executor.
 *
 * Same shape as agent-002 and agent-003. Kept locally so this agent
 * can run as a standalone Node process. If the agents converge on a
 * common runtime later, this is a candidate to lift into a shared
 * package.
 */
export interface OODAWorkflow<TObserve, TOrient, TDecide, TAct> {
  id: string;
  name: string;
  observe: () => Promise<TObserve>;
  orient: (observations: TObserve) => Promise<TOrient>;
  decide: (orientation: TOrient) => Promise<TDecide>;
  act: (decision: TDecide) => Promise<TAct>;
}

export async function executeOODA<TObserve, TOrient, TDecide, TAct>(
  workflow: OODAWorkflow<TObserve, TOrient, TDecide, TAct>
): Promise<OODAExecution<TObserve, TOrient, TDecide, TAct>> {
  const executionId = crypto.randomUUID();
  const execution: OODAExecution<TObserve, TOrient, TDecide, TAct> = {
    executionId,
    workflowId: workflow.id,
    status: "running",
    observations: null as unknown as TObserve,
    orientation: null,
    decision: null,
    actions: null,
    startedAt: new Date(),
    completedAt: null,
    error: null,
  };

  const log = (phase: string, msg: string) =>
    console.log(
      `[${new Date().toISOString()}] [${workflow.id}] [${phase}] ${msg}`
    );

  try {
    log("OBSERVE", "Gathering data...");
    execution.observations = await workflow.observe();

    log("ORIENT", "Analyzing context...");
    execution.orientation = await workflow.orient(execution.observations);

    log("DECIDE", "Making decision...");
    execution.decision = await workflow.decide(execution.orientation);

    log("ACT", "Executing actions...");
    execution.actions = await workflow.act(execution.decision);

    execution.status = "completed";
    log("DONE", "Workflow completed successfully.");
  } catch (err) {
    execution.status = "failed";
    execution.error = err instanceof Error ? err.message : String(err);
    log("ERROR", execution.error);
  }

  execution.completedAt = new Date();

  store.logExecution({
    executionId: execution.executionId,
    workflowId: execution.workflowId,
    agentId: config.agentId,
    status: execution.status,
    startedAt: execution.startedAt.toISOString(),
    completedAt: execution.completedAt.toISOString(),
    result: JSON.stringify({
      orientation: execution.orientation,
      decision: execution.decision,
      actions: execution.actions,
      error: execution.error,
    }),
  });

  return execution;
}
