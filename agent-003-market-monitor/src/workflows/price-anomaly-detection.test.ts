import { describe, it, expect } from "vitest";
import {
  median,
  stddev,
  classifyAnomaly,
  classIdFromBatchDenom,
  isUsdStableDenom,
} from "./price-anomaly-detection.js";

// ============================================================
// median — deterministic statistical median
// ============================================================

describe("median", () => {
  it("returns 0 for an empty list", () => {
    expect(median([])).toBe(0);
  });

  it("returns the single value for a one-element list", () => {
    expect(median([42])).toBe(42);
  });

  it("returns the middle value for an odd-length list", () => {
    expect(median([1, 5, 3])).toBe(3);
    expect(median([7, 2, 9, 4, 5])).toBe(5);
  });

  it("returns the mean of the two middle values for an even-length list", () => {
    expect(median([1, 2, 3, 4])).toBe(2.5);
    expect(median([10, 20])).toBe(15);
  });

  it("does not mutate the input array", () => {
    const input = [3, 1, 2];
    median(input);
    expect(input).toEqual([3, 1, 2]);
  });

  it("handles negative numbers correctly", () => {
    expect(median([-5, -1, -3])).toBe(-3);
    expect(median([-4, -2, 2, 4])).toBe(0);
  });

  it("handles floating point values", () => {
    expect(median([0.1, 0.2, 0.3])).toBeCloseTo(0.2, 10);
    expect(median([1.5, 2.5, 3.5, 4.5])).toBe(3);
  });
});

// ============================================================
// stddev — sample standard deviation
// ============================================================

describe("stddev", () => {
  it("returns 0 for an empty list", () => {
    expect(stddev([], 0)).toBe(0);
  });

  it("returns 0 for a single value (no variance possible)", () => {
    expect(stddev([42], 42)).toBe(0);
  });

  it("computes sample stddev for a two-element list", () => {
    // [1, 3], mean 2, sum of sq = 1 + 1 = 2, divide by n-1 = 2, sqrt = sqrt(2)
    expect(stddev([1, 3], 2)).toBeCloseTo(Math.SQRT2, 10);
  });

  it("computes sample stddev for a known five-element list", () => {
    // [2, 4, 4, 4, 5, 5, 7, 9] has a textbook stddev of 2 (sample)
    // but let's use a cleaner one: [1, 2, 3, 4, 5] mean 3
    // sum of sq dev = 4 + 1 + 0 + 1 + 4 = 10, / 4 = 2.5, sqrt ≈ 1.5811
    const values = [1, 2, 3, 4, 5];
    const mean = values.reduce((a, b) => a + b, 0) / values.length;
    expect(stddev(values, mean)).toBeCloseTo(Math.sqrt(2.5), 10);
  });

  it("uses n-1 denominator (sample, not population)", () => {
    // Population stddev of [1, 2] with mean 1.5 would be 0.5
    // Sample stddev is sqrt(0.5/1) ≈ 0.7071
    expect(stddev([1, 2], 1.5)).toBeCloseTo(Math.SQRT1_2, 10);
  });
});

// ============================================================
// classifyAnomaly — severity classifier
// ============================================================

describe("classifyAnomaly", () => {
  it("returns INFO for zero z-scores", () => {
    expect(classifyAnomaly(0, 0)).toBe("INFO");
  });

  it("returns INFO for z-scores below the WARNING threshold (2.0)", () => {
    expect(classifyAnomaly(1.9, 1.5)).toBe("INFO");
    expect(classifyAnomaly(-1.9, 1.5)).toBe("INFO");
  });

  it("returns WARNING at the exact WARNING threshold", () => {
    expect(classifyAnomaly(2.0, 0)).toBe("WARNING");
    expect(classifyAnomaly(0, 2.0)).toBe("WARNING");
  });

  it("returns WARNING between 2.0 and CRITICAL", () => {
    expect(classifyAnomaly(2.5, 1.0)).toBe("WARNING");
    expect(classifyAnomaly(3.4, 0)).toBe("WARNING");
  });

  it("returns CRITICAL at the exact CRITICAL threshold (3.5)", () => {
    expect(classifyAnomaly(3.5, 0)).toBe("CRITICAL");
    expect(classifyAnomaly(0, 3.5)).toBe("CRITICAL");
  });

  it("returns CRITICAL above the CRITICAL threshold", () => {
    expect(classifyAnomaly(5, 1)).toBe("CRITICAL");
    expect(classifyAnomaly(10, 0)).toBe("CRITICAL");
  });

  it("picks the larger absolute z-score from the two inputs", () => {
    expect(classifyAnomaly(1.5, 4.0)).toBe("CRITICAL");
    expect(classifyAnomaly(2.8, 1.0)).toBe("WARNING");
  });

  it("treats negative z-scores symmetrically with positive", () => {
    expect(classifyAnomaly(-3.6, 0)).toBe("CRITICAL");
    expect(classifyAnomaly(-2.1, 0)).toBe("WARNING");
  });
});

// ============================================================
// classIdFromBatchDenom — class ID extractor
// ============================================================

describe("classIdFromBatchDenom", () => {
  it("extracts the class ID from a canonical Regen batch denom", () => {
    expect(classIdFromBatchDenom("C01-001-20240101-20241231-001")).toBe("C01");
  });

  it("handles single-letter class codes", () => {
    expect(classIdFromBatchDenom("C-001-20240101-20241231-001")).toBe("C");
    expect(classIdFromBatchDenom("BT-001-20240101-20241231-001")).toBe("BT");
  });

  it("returns the whole string when no dash is present", () => {
    expect(classIdFromBatchDenom("UNKNOWN")).toBe("UNKNOWN");
  });

  it("handles an empty string", () => {
    expect(classIdFromBatchDenom("")).toBe("");
  });

  it("returns the whole string when the first character is a dash", () => {
    // Degenerate but deterministic — the slice is empty before the dash.
    expect(classIdFromBatchDenom("-leading-dash")).toBe("-leading-dash");
  });
});

// ============================================================
// isUsdStableDenom — stablecoin denom check
// ============================================================

describe("isUsdStableDenom", () => {
  it("recognizes USDC denoms case-insensitively", () => {
    expect(isUsdStableDenom("usdc")).toBe(true);
    expect(isUsdStableDenom("USDC")).toBe(true);
    expect(isUsdStableDenom("ibc/USDC-regen")).toBe(true);
  });

  it("recognizes USDT", () => {
    expect(isUsdStableDenom("usdt")).toBe(true);
    expect(isUsdStableDenom("ibc/USDT-axelar")).toBe(true);
  });

  it("recognizes DAI", () => {
    expect(isUsdStableDenom("dai")).toBe(true);
    expect(isUsdStableDenom("ibc/DAI-cosmos")).toBe(true);
  });

  it("rejects REGEN and other non-stable denoms", () => {
    expect(isUsdStableDenom("uregen")).toBe(false);
    expect(isUsdStableDenom("uatom")).toBe(false);
    expect(isUsdStableDenom("eco-credits")).toBe(false);
  });

  it("rejects the empty string", () => {
    expect(isUsdStableDenom("")).toBe(false);
  });
});
