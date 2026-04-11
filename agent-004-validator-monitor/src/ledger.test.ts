import { describe, it, expect } from "vitest";
import { LedgerClient } from "./ledger.js";

// ============================================================
// parseDelegationEventsFromTx — staking event extraction
// ============================================================

describe("LedgerClient.parseDelegationEventsFromTx", () => {
  const client = new LedgerClient();

  it("returns empty array for a tx with no events", () => {
    const tx = { txhash: "tx-empty", timestamp: "2026-02-18T12:00:00Z", logs: [] };
    expect(client.parseDelegationEventsFromTx(tx)).toEqual([]);
  });

  it("extracts a MsgDelegate event with the sender as delegator", () => {
    const tx = {
      txhash: "tx-delegate",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "message",
              attributes: [
                { key: "action", value: "/cosmos.staking.v1beta1.MsgDelegate" },
                { key: "sender", value: "regen1delegator1" },
              ],
            },
            {
              type: "delegate",
              attributes: [
                { key: "validator", value: "regenvaloper1alpha" },
                { key: "amount", value: "1000uregen" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseDelegationEventsFromTx(tx);
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({
      txHash: "tx-delegate",
      eventType: "delegate",
      delegator: "regen1delegator1",
      validator: "regenvaloper1alpha",
      sourceValidator: null,
      amountUregen: "1000",
    });
  });

  it("extracts a MsgUndelegate event from the `unbond` type", () => {
    const tx = {
      txhash: "tx-undelegate",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "message",
              attributes: [
                { key: "action", value: "/cosmos.staking.v1beta1.MsgUndelegate" },
                { key: "sender", value: "regen1delegator2" },
              ],
            },
            {
              type: "unbond",
              attributes: [
                { key: "validator", value: "regenvaloper1beta" },
                { key: "amount", value: "500uregen" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseDelegationEventsFromTx(tx);
    expect(out).toHaveLength(1);
    expect(out[0]!.eventType).toBe("undelegate");
    expect(out[0]!.validator).toBe("regenvaloper1beta");
    expect(out[0]!.amountUregen).toBe("500");
  });

  it("extracts a MsgBeginRedelegate event with source + destination", () => {
    const tx = {
      txhash: "tx-redelegate",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "message",
              attributes: [
                { key: "action", value: "/cosmos.staking.v1beta1.MsgBeginRedelegate" },
                { key: "sender", value: "regen1delegator3" },
              ],
            },
            {
              type: "redelegate",
              attributes: [
                { key: "source_validator", value: "regenvaloper1src" },
                { key: "destination_validator", value: "regenvaloper1dst" },
                { key: "amount", value: "2000uregen" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseDelegationEventsFromTx(tx);
    expect(out).toHaveLength(1);
    expect(out[0]!.eventType).toBe("redelegate");
    expect(out[0]!.sourceValidator).toBe("regenvaloper1src");
    expect(out[0]!.validator).toBe("regenvaloper1dst");
    expect(out[0]!.amountUregen).toBe("2000");
  });

  it("parses multiple staking events in a single batched tx", () => {
    // Two delegations and one undelegation in one tx, with positional
    // sender matching — sender[0] → first staking event, sender[1] →
    // second, sender[2] → third.
    const tx = {
      txhash: "tx-batched",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            { type: "message", attributes: [{ key: "sender", value: "regen1d1" }] },
            { type: "message", attributes: [{ key: "sender", value: "regen1d2" }] },
            { type: "message", attributes: [{ key: "sender", value: "regen1d3" }] },
            {
              type: "delegate",
              attributes: [
                { key: "validator", value: "regenvaloper1a" },
                { key: "amount", value: "100uregen" },
              ],
            },
            {
              type: "delegate",
              attributes: [
                { key: "validator", value: "regenvaloper1b" },
                { key: "amount", value: "200uregen" },
              ],
            },
            {
              type: "unbond",
              attributes: [
                { key: "validator", value: "regenvaloper1c" },
                { key: "amount", value: "50uregen" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseDelegationEventsFromTx(tx);
    expect(out).toHaveLength(3);
    expect(out[0]!.delegator).toBe("regen1d1");
    expect(out[1]!.delegator).toBe("regen1d2");
    expect(out[2]!.delegator).toBe("regen1d3");
  });

  it("ignores delegate events with missing validator attribute", () => {
    const tx = {
      txhash: "tx-missing-validator",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "message",
              attributes: [{ key: "sender", value: "regen1d" }],
            },
            {
              type: "delegate",
              attributes: [{ key: "amount", value: "1000uregen" }],
            },
          ],
        },
      ],
    };
    expect(client.parseDelegationEventsFromTx(tx)).toEqual([]);
  });

  it("ignores delegate events with malformed amount attribute", () => {
    const tx = {
      txhash: "tx-bad-amount",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            {
              type: "message",
              attributes: [{ key: "sender", value: "regen1d" }],
            },
            {
              type: "delegate",
              attributes: [
                { key: "validator", value: "regenvaloper1x" },
                { key: "amount", value: "no-numbers-here" },
              ],
            },
          ],
        },
      ],
    };
    expect(client.parseDelegationEventsFromTx(tx)).toEqual([]);
  });

  it("parses the amount prefix when the denom follows the number", () => {
    // The Cosmos SDK coin-amount format is "<uint>uregen". The parser
    // extracts the numeric prefix and drops the denom.
    const tx = {
      txhash: "tx-coin-format",
      timestamp: "2026-02-18T12:00:00Z",
      logs: [
        {
          events: [
            { type: "message", attributes: [{ key: "sender", value: "regen1d" }] },
            {
              type: "delegate",
              attributes: [
                { key: "validator", value: "regenvaloper1x" },
                { key: "amount", value: "12345678uregen" },
              ],
            },
          ],
        },
      ],
    };
    const out = client.parseDelegationEventsFromTx(tx);
    expect(out[0]!.amountUregen).toBe("12345678");
  });

  it("reads events from tx.events[] alongside logs[].events[]", () => {
    const tx = {
      txhash: "tx-flat",
      timestamp: "2026-02-18T12:00:00Z",
      events: [
        { type: "message", attributes: [{ key: "sender", value: "regen1d" }] },
        {
          type: "delegate",
          attributes: [
            { key: "validator", value: "regenvaloper1y" },
            { key: "amount", value: "42uregen" },
          ],
        },
      ],
    };
    const out = client.parseDelegationEventsFromTx(tx);
    expect(out).toHaveLength(1);
    expect(out[0]!.amountUregen).toBe("42");
  });
});
