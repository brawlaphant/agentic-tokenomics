/**
 * OODA Loop Workflow Executor
 *
 * Implements the Observe-Orient-Decide-Act pattern from
 * phase-2/2.2-agentic-workflows.md. Each agent workflow is expressed
 * as an OODAWorkflow and executed by this engine.
 *
 * Design notes:
 * - Stateless executor: all state lives in WorkflowExecution
 * - Step handlers are injected via StepHandlers interface (testable)
 * - Escalation is a first-class outcome, not an error
 */

import type {
  OODAWorkflow,
  ObserveStep,
  OrientStep,
  DecideStep,
  ActStep,
  WorkflowExecution,
  WorkflowDecision,
  WorkflowStatus,
} from "./types.js";

/** Injectable handlers for each OODA phase. */
export interface StepHandlers {
  observe(step: ObserveStep, context: WorkflowExecution): Promise<unknown>;
  orient(
    step: OrientStep,
    observations: unknown[],
    context: WorkflowExecution
  ): Promise<Record<string, unknown>>;
  decide(
    step: DecideStep,
    orientation: Record<string, unknown>
  ): Promise<WorkflowDecision>;
  act(
    step: ActStep,
    decision: WorkflowDecision,
    context: WorkflowExecution
  ): Promise<unknown>;
}

export class OODAExecutor {
  constructor(private handlers: StepHandlers) {}

  async execute(workflow: OODAWorkflow): Promise<WorkflowExecution> {
    const execution: WorkflowExecution = {
      executionId: crypto.randomUUID(),
      workflowId: workflow.id,
      agentId: workflow.agentId,
      status: "running",
      governanceLayer: workflow.governanceLayer,
      observations: [],
      orientation: null,
      decision: null,
      actions: [],
      startedAt: new Date(),
    };

    try {
      // --- OBSERVE ---
      for (const step of workflow.observe) {
        const result = await this.handlers.observe(step, execution);
        execution.observations.push({ stepId: step.id, data: result });
      }

      // --- ORIENT ---
      let orientation: Record<string, unknown> = {};
      for (const step of workflow.orient) {
        const result = await this.handlers.orient(
          step,
          execution.observations,
          execution
        );
        orientation = { ...orientation, ...result };
      }
      execution.orientation = orientation;

      // --- DECIDE ---
      const decision = await this.handlers.decide(
        workflow.decide,
        orientation
      );
      execution.decision = decision;

      // Check escalation threshold
      if (decision.confidence < workflow.decide.escalationThreshold) {
        execution.status = "escalated";
        execution.completedAt = new Date();
        return execution;
      }

      // --- ACT ---
      for (const step of workflow.act) {
        if (this.shouldExecuteAction(step, decision)) {
          const result = await this.handlers.act(step, decision, execution);
          execution.actions.push({ stepId: step.id, result });
        }
      }

      execution.status = "completed";
    } catch (error) {
      execution.status = "failed";
      execution.actions.push({
        stepId: "error",
        result: { error: String(error) },
      });
    }

    execution.completedAt = new Date();
    return execution;
  }

  /**
   * Evaluate whether an act step's condition is met.
   * Conditions are simple expressions like "recommendation == APPROVE".
   */
  private shouldExecuteAction(
    step: ActStep,
    decision: WorkflowDecision
  ): boolean {
    if (!step.condition) return true;

    // Simple condition evaluation: "recommendation == APPROVE"
    const match = step.condition.match(
      /^(\w+)\s*(==|!=|>=|<=|>|<)\s*(.+)$/
    );
    if (!match) return true;

    const [, field, op, rawValue] = match;
    const actual = (decision as unknown as Record<string, unknown>)[field];
    const expected = isNaN(Number(rawValue))
      ? rawValue.trim()
      : Number(rawValue);

    switch (op) {
      case "==":
        return actual === expected;
      case "!=":
        return actual !== expected;
      case ">=":
        return Number(actual) >= Number(expected);
      case "<=":
        return Number(actual) <= Number(expected);
      case ">":
        return Number(actual) > Number(expected);
      case "<":
        return Number(actual) < Number(expected);
      default:
        return true;
    }
  }
}
