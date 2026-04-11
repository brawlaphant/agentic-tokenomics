import { describe, it, expect } from "vitest";
import {
  absBig,
  aggregateEventsToFlows,
  summarizeFlows,
} from "./delegation-flow-analysis.js";
import type { DelegationEvent, DelegationFlow } from "../types.js";

// ============================================================
// absBig — BigInt absolute value
// ============================================================

describe("absBig", () => {
  it("returns 0 for 0n", () => {
    expect(absBig(0n)).toBe(0n);
  });

  it("returns the value for positive inputs", () => {
    expect(absBig(1n)).toBe(1n);
    expect(absBig(1_000_000_000_000_000n)).toBe(1_000_000_000_000_000n);
  });

  it("returns the negated value for negative inputs", () => {
    expect(absBig(-1n)).toBe(1n);
    expect(absBig(-1_000_000_000_000_000n)).toBe(1_000_000_000_000_000n);
  });

  it("handles values beyond JS number safe integer range", () => {
    const huge = 9_007_199_254_740_993n;
    expect(absBig(huge)).toBe(huge);
    expect(absBig(-huge)).toBe(huge);
  });

  it("is idempotent on positive inputs", () => {
    expect(absBig(absBig(42n))).toBe(42n);
    expect(absBig(absBig(-42n))).toBe(42n);
  });
});

// ============================================================
// aggregateEventsToFlows — group events into per-validator flows
// ============================================================

function mkEvent(overrides: Partial<DelegationEvent> = {}): DelegationEvent {
  return {
    txHash: "tx-test",
    eventType: "delegate",
    delegator: "regen1delegator",
    validator: "regenvaloper1alpha",
    sourceValidator: null,
    amountUregen: "100",
    occurredAt: "2026-02-18T12:00:00Z",
    ...overrides,
  };
}

describe("aggregateEventsToFlows", () => {
  it("returns empty for empty input", () => {
    expect(aggregateEventsToFlows([])).toEqual([]);
  });

  it("aggregates a single delegate event into one inflow", () => {
    const flows = aggregateEventsToFlows([
      mkEvent({ amountUregen: "500", validator: "regenvaloper1alpha" }),
    ]);
    expect(flows).toHaveLength(1);
    expect(flows[0]!.operatorAddress).toBe("regenvaloper1alpha");
    expect(flows[0]!.flowDirection).toBe("INFLOW");
    expect(flows[0]!.deltaUregen).toBe("500");
  });

  it("aggregates a single undelegate event into one outflow", () => {
    const flows = aggregateEventsToFlows([
      mkEvent({
        eventType: "undelegate",
        amountUregen: "500",
        validator: "regenvaloper1alpha",
      }),
    ]);
    expect(flows).toHaveLength(1);
    expect(flows[0]!.flowDirection).toBe("OUTFLOW");
    expect(flows[0]!.deltaUregen).toBe("-500");
    expect(flows[0]!.deltaAbsUregen).toBe("500");
  });

  it("splits a redelegate event into source outflow + destination inflow", () => {
    const flows = aggregateEventsToFlows([
      mkEvent({
        eventType: "redelegate",
        amountUregen: "1000",
        validator: "regenvaloper1dest",
        sourceValidator: "regenvaloper1src",
      }),
    ]);
    expect(flows).toHaveLength(2);
    const src = flows.find((f) => f.operatorAddress === "regenvaloper1src")!;
    const dst = flows.find((f) => f.operatorAddress === "regenvaloper1dest")!;
    expect(src.flowDirection).toBe("OUTFLOW");
    expect(src.deltaUregen).toBe("-1000");
    expect(dst.flowDirection).toBe("INFLOW");
    expect(dst.deltaUregen).toBe("1000");
  });

  it("nets delegate and undelegate on the same validator", () => {
    const flows = aggregateEventsToFlows([
      mkEvent({ validator: "regenvaloper1alpha", amountUregen: "1000" }),
      mkEvent({
        eventType: "undelegate",
        validator: "regenvaloper1alpha",
        amountUregen: "300",
      }),
    ]);
    expect(flows).toHaveLength(1);
    expect(flows[0]!.deltaUregen).toBe("700");
    expect(flows[0]!.flowDirection).toBe("INFLOW");
  });

  it("skips validators whose net delta is exactly zero", () => {
    const flows = aggregateEventsToFlows([
      mkEvent({ validator: "regenvaloper1alpha", amountUregen: "500" }),
      mkEvent({
        eventType: "undelegate",
        validator: "regenvaloper1alpha",
        amountUregen: "500",
      }),
    ]);
    expect(flows).toEqual([]);
  });

  it("flags whale-sized deltas above the configured threshold", () => {
    // whaleDelegationUregen default is 100_000_000_000 (100K REGEN).
    const flows = aggregateEventsToFlows([
      mkEvent({
        validator: "regenvaloper1whale",
        amountUregen: "200000000000",
      }),
      mkEvent({
        validator: "regenvaloper1minnow",
        amountUregen: "1000",
      }),
    ]);
    const whale = flows.find((f) => f.operatorAddress === "regenvaloper1whale")!;
    const minnow = flows.find((f) => f.operatorAddress === "regenvaloper1minnow")!;
    expect(whale.isWhale).toBe(true);
    expect(minnow.isWhale).toBe(false);
  });

  it("skips events with zero or negative amountUregen", () => {
    const flows = aggregateEventsToFlows([
      mkEvent({ amountUregen: "0" }),
      mkEvent({ amountUregen: "-100" }),
    ]);
    expect(flows).toEqual([]);
  });

  it("skips events with a non-numeric amountUregen", () => {
    const flows = aggregateEventsToFlows([mkEvent({ amountUregen: "not-a-number" })]);
    expect(flows).toEqual([]);
  });
});

// ============================================================
// summarizeFlows — summary math (inflow/outflow/net/whale/top)
// ============================================================

function mkFlow(overrides: Partial<DelegationFlow> = {}): DelegationFlow {
  return {
    operatorAddress: "regenvaloper1test",
    moniker: "Test",
    previousTokens: "0",
    currentTokens: "0",
    deltaUregen: "0",
    deltaAbsUregen: "0",
    isWhale: false,
    flowDirection: "FLAT",
    capturedAt: "2026-02-18T12:00:00Z",
    ...overrides,
  };
}

describe("summarizeFlows", () => {
  it("returns zero totals for empty input", () => {
    const summary = summarizeFlows([]);
    expect(summary.totalInflowUregen).toBe("0");
    expect(summary.totalOutflowUregen).toBe("0");
    expect(summary.netFlowUregen).toBe("0");
    expect(summary.validatorsWithFlow).toBe(0);
    expect(summary.whaleFlowCount).toBe(0);
    expect(summary.topInflow).toBeNull();
    expect(summary.topOutflow).toBeNull();
  });

  it("sums inflows and outflows correctly", () => {
    const summary = summarizeFlows([
      mkFlow({ deltaUregen: "1000", flowDirection: "INFLOW" }),
      mkFlow({ deltaUregen: "500", flowDirection: "INFLOW" }),
      mkFlow({
        deltaUregen: "-300",
        deltaAbsUregen: "300",
        flowDirection: "OUTFLOW",
      }),
    ]);
    expect(summary.totalInflowUregen).toBe("1500");
    expect(summary.totalOutflowUregen).toBe("300");
    expect(summary.netFlowUregen).toBe("1200");
    expect(summary.validatorsWithFlow).toBe(3);
  });

  it("identifies the top inflow and top outflow", () => {
    const summary = summarizeFlows([
      mkFlow({
        operatorAddress: "regenvaloper1big",
        deltaUregen: "10000",
        flowDirection: "INFLOW",
      }),
      mkFlow({
        operatorAddress: "regenvaloper1small",
        deltaUregen: "100",
        flowDirection: "INFLOW",
      }),
      mkFlow({
        operatorAddress: "regenvaloper1bigout",
        deltaUregen: "-5000",
        deltaAbsUregen: "5000",
        flowDirection: "OUTFLOW",
      }),
    ]);
    expect(summary.topInflow?.operatorAddress).toBe("regenvaloper1big");
    expect(summary.topOutflow?.operatorAddress).toBe("regenvaloper1bigout");
  });

  it("counts whale flows separately from the total", () => {
    const summary = summarizeFlows([
      mkFlow({ deltaUregen: "500000000000", isWhale: true, flowDirection: "INFLOW" }),
      mkFlow({ deltaUregen: "300", isWhale: false, flowDirection: "INFLOW" }),
    ]);
    expect(summary.whaleFlowCount).toBe(1);
    expect(summary.validatorsWithFlow).toBe(2);
  });
});
