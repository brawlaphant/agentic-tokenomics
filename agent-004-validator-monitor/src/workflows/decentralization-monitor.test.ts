import { describe, it, expect } from "vitest";
import {
  nakamotoCoefficient,
  giniIndex,
  topNSharePct,
  classifyHealth,
} from "./decentralization-monitor.js";
import { config } from "../config.js";

// ============================================================
// nakamotoCoefficient — smallest set of validators whose cumulative
// share strictly exceeds 33.4% of the total bonded stake.
// ============================================================

describe("nakamotoCoefficient", () => {
  it("returns 0 for empty input", () => {
    expect(nakamotoCoefficient([], 0n)).toBe(0);
    expect(nakamotoCoefficient([], 100n)).toBe(0);
  });

  it("returns 0 when total is zero", () => {
    expect(nakamotoCoefficient([10n, 20n], 0n)).toBe(0);
  });

  it("returns 1 when a single validator holds the entire stake", () => {
    expect(nakamotoCoefficient([1000n], 1000n)).toBe(1);
  });

  it("returns 1 when a single validator holds more than 33.4%", () => {
    // Top validator has 40% (400 out of 1000) — a single validator can halt.
    expect(nakamotoCoefficient([400n, 300n, 200n, 100n], 1000n)).toBe(1);
  });

  it("returns 2 when the top validator holds exactly 33.4%", () => {
    // 334/1000 = 33.4% — the strict-inequality predicate `> threshold`
    // means the top validator alone does not clear the line. Need a
    // second validator. The SPEC defines Nakamoto as the SMALLEST n
    // whose cumulative share STRICTLY EXCEEDS 33.4%.
    expect(nakamotoCoefficient([334n, 333n, 333n], 1000n)).toBe(2);
  });

  it("returns 2 when the top two combined exceed 33.4%", () => {
    // Top two: 200+180 = 380, exceeds 334 (threshold for 33.4% of 1000)
    expect(nakamotoCoefficient([200n, 180n, 170n, 150n, 150n, 150n], 1000n)).toBe(2);
  });

  it("returns a larger N when stake is distributed evenly", () => {
    // 10 validators, each with 100 out of 1000 total.
    // Cumulative share reaches 33.4% between the 3rd and 4th validator
    // (3*100 = 300 < 334, 4*100 = 400 > 334). So Nakamoto = 4.
    const tokens = Array(10).fill(100n);
    expect(nakamotoCoefficient(tokens, 1000n)).toBe(4);
  });

  it("returns the length when no subset exceeds the threshold", () => {
    // Degenerate: total is larger than the sum of the list (would not
    // happen in practice, but the guard clamps to the array length).
    expect(nakamotoCoefficient([10n, 10n], 1000n)).toBe(2);
  });
});

// ============================================================
// giniIndex — textbook Gini on an ascending-sorted array
// ============================================================

describe("giniIndex", () => {
  it("returns 0 for empty input", () => {
    expect(giniIndex([])).toBe(0);
  });

  it("returns 0 for a single-element distribution", () => {
    expect(giniIndex([100])).toBe(0);
  });

  it("returns 0 for perfect equality", () => {
    expect(giniIndex([100, 100, 100, 100])).toBe(0);
    expect(giniIndex([5, 5, 5, 5, 5, 5])).toBe(0);
  });

  it("returns a positive number for unequal distribution", () => {
    expect(giniIndex([10, 20, 30, 40])).toBeGreaterThan(0);
  });

  it("approaches 1 for maximally unequal distribution", () => {
    // One player has everything, others have nothing
    const gini = giniIndex([0, 0, 0, 0, 100]);
    expect(gini).toBeGreaterThan(0.6);
    expect(gini).toBeLessThan(1);
  });

  it("returns 0 when the cumulative sum is zero (all zeros)", () => {
    expect(giniIndex([0, 0, 0])).toBe(0);
  });

  it("is monotonic — more inequality, higher Gini", () => {
    const nearEqual = giniIndex([95, 96, 97, 98, 99, 100]);
    const moreUnequal = giniIndex([10, 20, 30, 40, 50, 60]);
    expect(moreUnequal).toBeGreaterThan(nearEqual);
  });
});

// ============================================================
// topNSharePct — cumulative share of the top N
// ============================================================

describe("topNSharePct", () => {
  it("returns 0 when total is zero", () => {
    expect(topNSharePct([10n, 20n], 0n, 2)).toBe(0);
  });

  it("returns the correct share for the single largest validator", () => {
    expect(topNSharePct([400n, 300n, 200n, 100n], 1000n, 1)).toBe(40);
  });

  it("returns the correct share for the top 3", () => {
    // 400 + 300 + 200 = 900 of 1000 = 90%
    expect(topNSharePct([400n, 300n, 200n, 100n], 1000n, 3)).toBe(90);
  });

  it("returns 100% when n exceeds the array length", () => {
    expect(topNSharePct([400n, 300n, 200n, 100n], 1000n, 100)).toBe(100);
  });

  it("returns 100% when n equals the array length", () => {
    expect(topNSharePct([400n, 300n, 200n, 100n], 1000n, 4)).toBe(100);
  });

  it("has two-decimal precision on the returned percentage", () => {
    // 333 / 1000 = 33.3%
    expect(topNSharePct([333n, 333n, 334n], 1000n, 1)).toBe(33.3);
  });
});

// ============================================================
// classifyHealth — composite health tier
// ============================================================

describe("classifyHealth", () => {
  const v = config.validator;

  it("returns HEALTHY for high Nakamoto, low Gini, low concentration", () => {
    expect(classifyHealth(20, 0.3, 10)).toBe("HEALTHY");
  });

  it("returns CRITICAL when Nakamoto is at or below the critical floor", () => {
    expect(classifyHealth(v.nakamotoCriticalFloor, 0.3, 10)).toBe("CRITICAL");
    expect(classifyHealth(v.nakamotoCriticalFloor - 1, 0.3, 10)).toBe("CRITICAL");
  });

  it("returns CRITICAL when a single validator exceeds the critical concentration", () => {
    expect(classifyHealth(20, 0.3, v.criticalConcentrationPct)).toBe("CRITICAL");
    expect(classifyHealth(20, 0.3, v.criticalConcentrationPct + 5)).toBe("CRITICAL");
  });

  it("returns WARNING when Nakamoto is at the warning floor but above critical", () => {
    expect(classifyHealth(v.nakamotoWarningFloor, 0.3, 10)).toBe("WARNING");
  });

  it("returns WARNING when Gini exceeds the warning ceiling", () => {
    expect(classifyHealth(20, v.giniWarningCeiling, 10)).toBe("WARNING");
    expect(classifyHealth(20, v.giniWarningCeiling + 0.05, 10)).toBe("WARNING");
  });

  it("returns WARNING when a single validator is at the warning concentration", () => {
    expect(classifyHealth(20, 0.3, v.warningConcentrationPct)).toBe("WARNING");
  });

  it("CRITICAL wins over WARNING when multiple thresholds trip", () => {
    expect(
      classifyHealth(v.nakamotoCriticalFloor, v.giniWarningCeiling, v.warningConcentrationPct)
    ).toBe("CRITICAL");
  });
});
