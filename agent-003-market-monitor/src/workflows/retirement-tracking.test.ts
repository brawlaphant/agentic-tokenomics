import { describe, it, expect } from "vitest";
import {
  computeDemandIndex,
  aggregateRetirementsByClass,
} from "./retirement-tracking.js";
import type { Retirement } from "../types.js";

// ============================================================
// computeDemandIndex — bounded 0-100 demand signal
// ============================================================

describe("computeDemandIndex", () => {
  it("returns 0 for zero-activity inputs", () => {
    expect(computeDemandIndex(0, 0, 0)).toBe(0);
  });

  it("caps the volume component at 60", () => {
    expect(computeDemandIndex(1e100, 0, 0)).toBe(60);
  });

  it("caps the count component at 20 (10 retirements)", () => {
    const big = computeDemandIndex(0, 50, 0);
    const exact = computeDemandIndex(0, 10, 0);
    expect(big).toBe(20);
    expect(exact).toBe(20);
  });

  it("caps the breadth component at 20 (5 unique retirees)", () => {
    const big = computeDemandIndex(0, 0, 100);
    const exact = computeDemandIndex(0, 0, 5);
    expect(big).toBe(20);
    expect(exact).toBe(20);
  });

  it("caps the total at 100 when all three components max out", () => {
    expect(computeDemandIndex(1e100, 50, 50)).toBe(100);
  });

  it("returns a mid-range value for moderate activity", () => {
    expect(computeDemandIndex(10000, 5, 3)).toBe(82);
  });

  it("uses log10 scaling so each order of magnitude adds ~20 pre-cap", () => {
    const a = computeDemandIndex(10, 0, 0);
    const b = computeDemandIndex(100, 0, 0);
    expect(b - a).toBe(20);
  });

  it("treats totalQuantity < 1 as 1 (log10 floor)", () => {
    expect(computeDemandIndex(0.5, 0, 0)).toBe(0);
    expect(computeDemandIndex(0.0001, 0, 0)).toBe(0);
  });

  it("rounds the final index to the nearest integer", () => {
    expect(computeDemandIndex(3, 0, 0)).toBe(10);
  });
});

// ============================================================
// aggregateRetirementsByClass — per-class summary builder
// ============================================================

function mkRetirement(overrides: Partial<Retirement> = {}): Retirement {
  return {
    txHash: "tx-test",
    batchDenom: "C01-001-20240101-20241231-001",
    classId: "C01",
    retiree: "regen1retiree",
    quantity: 100,
    jurisdiction: null,
    reason: null,
    retiredAt: "2026-02-18T12:00:00Z",
    ...overrides,
  };
}

describe("aggregateRetirementsByClass", () => {
  it("returns an empty map for zero retirements", () => {
    const result = aggregateRetirementsByClass([]);
    expect(result.size).toBe(0);
  });

  it("aggregates a single retirement into a one-entry summary", () => {
    const result = aggregateRetirementsByClass([
      mkRetirement({ quantity: 500 }),
    ]);
    expect(result.size).toBe(1);
    const summary = result.get("C01")!;
    expect(summary.retirementCount).toBe(1);
    expect(summary.totalQuantity).toBe(500);
    expect(summary.uniqueRetirees).toBe(1);
    expect(summary.topRetiree).toBe("regen1retiree");
    expect(summary.topRetireeQuantity).toBe(500);
  });

  it("groups multiple retirements by class id", () => {
    const result = aggregateRetirementsByClass([
      mkRetirement({ classId: "C01", quantity: 100 }),
      mkRetirement({ classId: "C01", quantity: 200, retiree: "regen1other" }),
      mkRetirement({ classId: "BT", quantity: 50 }),
    ]);
    expect(result.size).toBe(2);
    expect(result.get("C01")!.totalQuantity).toBe(300);
    expect(result.get("C01")!.retirementCount).toBe(2);
    expect(result.get("C01")!.uniqueRetirees).toBe(2);
    expect(result.get("BT")!.totalQuantity).toBe(50);
    expect(result.get("BT")!.retirementCount).toBe(1);
  });

  it("identifies the top retiree by cumulative quantity, not count", () => {
    const result = aggregateRetirementsByClass([
      mkRetirement({ quantity: 10, retiree: "regen1spammer" }),
      mkRetirement({ quantity: 10, retiree: "regen1spammer" }),
      mkRetirement({ quantity: 10, retiree: "regen1spammer" }),
      mkRetirement({ quantity: 100, retiree: "regen1whale" }),
    ]);
    const summary = result.get("C01")!;
    expect(summary.uniqueRetirees).toBe(2);
    expect(summary.topRetiree).toBe("regen1whale");
    expect(summary.topRetireeQuantity).toBe(100);
  });

  it("computes the jurisdiction metadata percentage", () => {
    const result = aggregateRetirementsByClass([
      mkRetirement({ jurisdiction: "US-CA" }),
      mkRetirement({ jurisdiction: "US-NY" }),
      mkRetirement({ jurisdiction: null }),
      mkRetirement({ jurisdiction: null }),
    ]);
    const summary = result.get("C01")!;
    expect(summary.pctWithJurisdiction).toBe(50);
  });

  it("skips classes whose retirements all sum to zero quantity", () => {
    const result = aggregateRetirementsByClass([
      mkRetirement({ quantity: 0 }),
    ]);
    expect(result.size).toBe(0);
  });

  it("handles retirements with empty retiree string without crashing", () => {
    const result = aggregateRetirementsByClass([
      mkRetirement({ retiree: "", quantity: 100 }),
    ]);
    const summary = result.get("C01")!;
    expect(summary.totalQuantity).toBe(100);
    expect(summary.uniqueRetirees).toBe(0);
    expect(summary.topRetiree).toBeNull();
  });
});
