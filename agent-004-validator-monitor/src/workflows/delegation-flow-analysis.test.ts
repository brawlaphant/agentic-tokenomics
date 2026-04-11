import { describe, it, expect } from "vitest";
import { absBig } from "./delegation-flow-analysis.js";

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
    // 2^53 + 1 — beyond Number.MAX_SAFE_INTEGER, only BigInt can represent exactly.
    const huge = 9_007_199_254_740_993n;
    expect(absBig(huge)).toBe(huge);
    expect(absBig(-huge)).toBe(huge);
  });

  it("is idempotent on positive inputs", () => {
    expect(absBig(absBig(42n))).toBe(42n);
    expect(absBig(absBig(-42n))).toBe(42n);
  });
});
