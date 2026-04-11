import { describe, it, expect } from "vitest";
import { askUsd, scoreHealth } from "./liquidity-monitor.js";
import type { SellOrder } from "../types.js";
import { config } from "../config.js";

// Small helper to build a well-typed SellOrder for a test case.
function makeOrder(overrides: Partial<SellOrder> = {}): SellOrder {
  return {
    id: "order-test",
    seller: "regen1seller",
    batch_denom: "C-001-20240101-20241231-001",
    quantity: "100",
    ask_denom: "uusdc",
    ask_amount: "5000",
    disable_auto_retire: false,
    expiration: "2030-01-01T00:00:00Z",
    ...overrides,
  };
}

// ============================================================
// askUsd — per-unit ask price in USD
// ============================================================

describe("askUsd", () => {
  it("divides ask_amount by quantity for a normal order", () => {
    const order = makeOrder({ ask_amount: "5000", quantity: "100" });
    expect(askUsd(order)).toBe(50);
  });

  it("returns 0 when quantity is zero", () => {
    expect(askUsd(makeOrder({ quantity: "0" }))).toBe(0);
  });

  it("returns 0 when quantity is negative", () => {
    expect(askUsd(makeOrder({ quantity: "-10" }))).toBe(0);
  });

  it("returns 0 when ask_amount is not a finite number", () => {
    expect(askUsd(makeOrder({ ask_amount: "not-a-number" }))).toBe(0);
  });

  it("returns 0 when quantity is not a finite number", () => {
    expect(askUsd(makeOrder({ quantity: "NaN" }))).toBe(0);
  });

  it("handles fractional prices correctly", () => {
    const order = makeOrder({ ask_amount: "125", quantity: "50" });
    expect(askUsd(order)).toBe(2.5);
  });
});

// ============================================================
// scoreHealth — liquidity health tier classifier
// ============================================================

describe("scoreHealth", () => {
  const floor = config.market.liquidityDepthFloor; // 5000 in the default config

  it("returns CRITICAL when depth is below half the floor", () => {
    // depthUsd = 2000, floor = 5000 → 2000 < 2500 → CRITICAL
    const result = scoreHealth(floor * 0.4, 5);
    expect(result.tier).toBe("CRITICAL");
  });

  it("returns DEGRADED when depth is below the floor but above half", () => {
    // depthUsd = 3000 (floor 5000 → half 2500, so above half),
    // 10 orders so countScore = 0.5, depthRatio = 3000/10000 = 0.3,
    // score = 0.3*0.7 + 0.5*0.3 = 0.21 + 0.15 = 0.36 — not CRITICAL
    // (0.36 >= 0.3), not HEALTHY (3000 < 5000), so DEGRADED.
    const result = scoreHealth(floor * 0.6, 10);
    expect(result.tier).toBe("DEGRADED");
  });

  it("returns HEALTHY when depth clears the floor and order count is strong", () => {
    // depthUsd = 15000, 20 orders → depthRatio clamped to 1.0, countScore = 1.0, score = 1.0
    const result = scoreHealth(floor * 3, 20);
    expect(result.tier).toBe("HEALTHY");
    expect(result.score).toBeCloseTo(1.0, 10);
  });

  it("returns DEGRADED when depth is above the floor but score drops below 0.6", () => {
    // depth = exactly floor, sellOrderCount = 2 → depthRatio = 0.5, countScore = 0.1, score = 0.38
    // 0.38 < 0.6 → DEGRADED
    const result = scoreHealth(floor, 2);
    expect(result.tier).toBe("DEGRADED");
    expect(result.score).toBeLessThan(0.6);
  });

  it("caps the depth score at 1.0 even for very high depth", () => {
    const result = scoreHealth(floor * 100, 50);
    expect(result.score).toBeLessThanOrEqual(1.0);
  });

  it("caps the count score at 20 orders", () => {
    const low = scoreHealth(floor * 2, 20);
    const high = scoreHealth(floor * 2, 200);
    expect(low.score).toBeCloseTo(high.score, 10);
  });

  it("scores zero depth and zero orders at 0.0", () => {
    const result = scoreHealth(0, 0);
    expect(result.score).toBe(0);
    expect(result.tier).toBe("CRITICAL");
  });
});
