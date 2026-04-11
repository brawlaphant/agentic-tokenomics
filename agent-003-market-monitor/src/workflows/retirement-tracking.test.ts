import { describe, it, expect } from "vitest";
// Import directly from utils.ts so the test does not transitively
// pull in the workflow module, which constructs the SQLite-backed
// Store singleton at module load.
import { computeDemandIndex } from "../utils.js";

// ============================================================
// computeDemandIndex — bounded 0-100 demand signal
// ============================================================

describe("computeDemandIndex", () => {
  it("returns 0 for zero-activity inputs", () => {
    // volume component: log10(max(1, 0)) * 20 = 0, count: 0, breadth: 0
    expect(computeDemandIndex(0, 0, 0)).toBe(0);
  });

  it("caps the volume component at 60", () => {
    // log10(1e100) * 20 = 2000, clamped to 60
    expect(computeDemandIndex(1e100, 0, 0)).toBe(60);
  });

  it("caps the count component at 20 (10 retirements)", () => {
    // 50 retirements → 50 * 2 = 100, clamped to 20
    const big = computeDemandIndex(0, 50, 0);
    const exact = computeDemandIndex(0, 10, 0);
    expect(big).toBe(20);
    expect(exact).toBe(20);
  });

  it("caps the breadth component at 20 (5 unique retirees)", () => {
    // 100 unique retirees → 100 * 4 = 400, clamped to 20
    const big = computeDemandIndex(0, 0, 100);
    const exact = computeDemandIndex(0, 0, 5);
    expect(big).toBe(20);
    expect(exact).toBe(20);
  });

  it("caps the total at 100 when all three components max out", () => {
    // volume 60 + count 20 + breadth 20 = 100
    expect(computeDemandIndex(1e100, 50, 50)).toBe(100);
  });

  it("returns a mid-range value for moderate activity", () => {
    // totalQuantity = 10000 → log10(10000)*20 = 80 → clamped to 60
    // retirementCount = 5 → 5*2 = 10
    // uniqueRetirees = 3 → 3*4 = 12
    // total = 60 + 10 + 12 = 82
    expect(computeDemandIndex(10000, 5, 3)).toBe(82);
  });

  it("uses log10 scaling so each order of magnitude adds ~20 pre-cap", () => {
    // totalQty 10 → log10(10) * 20 = 20
    // totalQty 100 → log10(100) * 20 = 40
    const a = computeDemandIndex(10, 0, 0);
    const b = computeDemandIndex(100, 0, 0);
    expect(b - a).toBe(20);
  });

  it("treats totalQuantity < 1 as 1 (log10 floor)", () => {
    // Math.max(1, 0.5) = 1, log10(1) = 0
    expect(computeDemandIndex(0.5, 0, 0)).toBe(0);
    expect(computeDemandIndex(0.0001, 0, 0)).toBe(0);
  });

  it("rounds the final index to the nearest integer", () => {
    // totalQty = 3 → log10(3) * 20 ≈ 9.542
    // Math.round(9.542) = 10
    expect(computeDemandIndex(3, 0, 0)).toBe(10);
  });
});
