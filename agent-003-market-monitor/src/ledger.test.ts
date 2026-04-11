import { describe, it, expect } from "vitest";
import { LedgerClient } from "./ledger.js";

// ============================================================
// parseRetirementsFromTx — event extraction from tx response
// ============================================================

describe("LedgerClient.parseRetirementsFromTx", () => {
  const client = new LedgerClient();

  it("returns empty array for a tx with no events", () => {
    const tx = { txhash: "tx-empty", timestamp: "2026-02-18T12:00:00Z", logs: [] };
    const out = client.parseRetirementsFromTx(tx);
    expect(out).toEqual([]);
  });

  it("extracts a single retirement from an EventRetire attribute set", () => {
    const tx = {
      txhash: "tx-single",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "regen.ecocredit.v1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1retiree1" },
                { key: "batch_denom", value: "C01-001-20240101-20241231-001" },
                { key: "amount", value: "1000" },
                { key: "jurisdiction", value: "US-CA" },
                { key: "reason", value: "voluntary offset" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseRetirementsFromTx(tx);
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({
      txHash: "tx-single",
      batchDenom: "C01-001-20240101-20241231-001",
      classId: "C01",
      retiree: "regen1retiree1",
      quantity: 1000,
      jurisdiction: "US-CA",
      reason: "voluntary offset",
    });
  });

  it("accepts the v1beta1 event type as a fallback", () => {
    const tx = {
      txhash: "tx-beta",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "regen.ecocredit.v1beta1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1retiree2" },
                { key: "batch_denom", value: "BT-001-20240101-20241231-001" },
                { key: "amount", value: "500" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseRetirementsFromTx(tx);
    expect(out).toHaveLength(1);
    expect(out[0]!.classId).toBe("BT");
    expect(out[0]!.quantity).toBe(500);
    expect(out[0]!.jurisdiction).toBeNull();
    expect(out[0]!.reason).toBeNull();
  });

  it("ignores events that are not EventRetire", () => {
    const tx = {
      txhash: "tx-other",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "coin_spent",
              attributes: [{ key: "amount", value: "100uregen" }],
            },
            {
              type: "transfer",
              attributes: [{ key: "recipient", value: "regen1other" }],
            },
          ],
        },
      ],
    };
    const out = client.parseRetirementsFromTx(tx);
    expect(out).toEqual([]);
  });

  it("ignores EventRetire with missing batch_denom", () => {
    const tx = {
      txhash: "tx-missing-denom",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "regen.ecocredit.v1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1retiree" },
                { key: "amount", value: "1000" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseRetirementsFromTx(tx);
    expect(out).toEqual([]);
  });

  it("ignores EventRetire with non-finite or non-positive amount", () => {
    const tx = {
      txhash: "tx-bad-amount",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "regen.ecocredit.v1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1retiree" },
                { key: "batch_denom", value: "C01-001-2024-2024-001" },
                { key: "amount", value: "not-a-number" },
              ],
            },
            {
              type: "regen.ecocredit.v1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1retiree" },
                { key: "batch_denom", value: "C01-001-2024-2024-002" },
                { key: "amount", value: "0" },
              ],
            },
            {
              type: "regen.ecocredit.v1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1retiree" },
                { key: "batch_denom", value: "C01-001-2024-2024-003" },
                { key: "amount", value: "-500" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseRetirementsFromTx(tx);
    expect(out).toEqual([]);
  });

  it("extracts multiple retirements from a batched tx", () => {
    const tx = {
      txhash: "tx-batched",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "regen.ecocredit.v1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1retiree1" },
                { key: "batch_denom", value: "C01-001-2024-2024-001" },
                { key: "amount", value: "100" },
              ],
            },
            {
              type: "regen.ecocredit.v1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1retiree2" },
                { key: "batch_denom", value: "C01-002-2024-2024-001" },
                { key: "amount", value: "200" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseRetirementsFromTx(tx);
    expect(out).toHaveLength(2);
    expect(out[0]!.quantity).toBe(100);
    expect(out[1]!.quantity).toBe(200);
  });

  it("reads events from tx.events[] as well as logs[].events[]", () => {
    // Some LCD versions flatten the events onto the top-level tx
    // rather than nesting them under logs[].events. The parser
    // harvests both paths.
    const tx = {
      txhash: "tx-flat",
      timestamp: "2026-02-18T12:00:00Z",
      events: [
        {
          type: "regen.ecocredit.v1.EventRetire",
          attributes: [
            { key: "owner", value: "regen1retiree" },
            { key: "batch_denom", value: "C01-001-2024-2024-001" },
            { key: "amount", value: "42" },
          ],
        },
      ],
    };
    const out = client.parseRetirementsFromTx(tx);
    expect(out).toHaveLength(1);
    expect(out[0]!.quantity).toBe(42);
  });

  it("carries the tx hash and timestamp through to the Retirement record", () => {
    const tx = {
      txhash: "ABCDEF1234567890",
      timestamp: "2026-03-01T08:30:00Z",
      logs: [
        {
          events: [
            {
              type: "regen.ecocredit.v1.EventRetire",
              attributes: [
                { key: "owner", value: "regen1r" },
                { key: "batch_denom", value: "C-1-2024-2024-1" },
                { key: "amount", value: "10" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseRetirementsFromTx(tx);
    expect(out[0]!.txHash).toBe("ABCDEF1234567890");
    expect(out[0]!.retiredAt).toBe("2026-03-01T08:30:00Z");
  });
});
