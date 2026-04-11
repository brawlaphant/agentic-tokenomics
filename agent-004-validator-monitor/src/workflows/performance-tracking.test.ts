import { describe, it, expect } from "vitest";
import { bech32 } from "bech32";
import { operatorToAccountBech32 } from "./performance-tracking.js";

// ============================================================
// operatorToAccountBech32 — strip "valoper" from the HRP
// ============================================================
//
// A Cosmos operator address (e.g. `regenvaloper1abc...`) and its
// corresponding delegator address (`regen1abc...`) encode the same
// 20-byte payload with different human-readable prefixes. The
// converter decodes the bech32, strips "valoper" from the prefix,
// and re-encodes. We use real bech32.encode to construct test
// inputs so the test is not dependent on the npm bech32 library
// implementation details — any change in the library's output
// format would be caught by the encode helper, not by hand-rolled
// fixture strings.

function mkValoper(hrp: string, bytes: number[]): string {
  return bech32.encode(hrp, bech32.toWords(new Uint8Array(bytes)));
}

describe("operatorToAccountBech32", () => {
  // Twenty bytes — the canonical Cosmos address length.
  const payload = [
    0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
    0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
    0xfe, 0xdc, 0xba, 0x98,
  ];

  it("converts a regen valoper address to the regen delegator address", () => {
    const operator = mkValoper("regenvaloper", payload);
    const expected = bech32.encode(
      "regen",
      bech32.toWords(new Uint8Array(payload))
    );
    expect(operatorToAccountBech32(operator)).toBe(expected);
  });

  it("works for any chain prefix, not just regen", () => {
    const operator = mkValoper("cosmosvaloper", payload);
    const expected = bech32.encode(
      "cosmos",
      bech32.toWords(new Uint8Array(payload))
    );
    expect(operatorToAccountBech32(operator)).toBe(expected);
  });

  it("preserves the byte payload — round-trip decode produces identical bytes", () => {
    const operator = mkValoper("regenvaloper", payload);
    const delegator = operatorToAccountBech32(operator)!;
    const decoded = bech32.decode(delegator);
    const decodedBytes = bech32.fromWords(decoded.words);
    expect(Array.from(decodedBytes)).toEqual(payload);
  });

  it("returns null for a non-bech32 input", () => {
    expect(operatorToAccountBech32("not-a-bech32-address")).toBeNull();
    expect(operatorToAccountBech32("")).toBeNull();
  });

  it("returns null for a bech32 address that does not end in 'valoper'", () => {
    // A plain delegator address has no valoper suffix — converting
    // from it doesn't make sense.
    const delegator = bech32.encode(
      "regen",
      bech32.toWords(new Uint8Array(payload))
    );
    expect(operatorToAccountBech32(delegator)).toBeNull();
  });

  it("returns null when stripping 'valoper' leaves an empty prefix", () => {
    // "valoper" is the entire HRP — nothing left after stripping.
    const weird = bech32.encode(
      "valoper",
      bech32.toWords(new Uint8Array(payload))
    );
    expect(operatorToAccountBech32(weird)).toBeNull();
  });

  it("returns null for a bech32 address with a bad checksum", () => {
    // Build a valid address, mutate the last data character so the
    // checksum no longer matches.
    const operator = mkValoper("regenvaloper", payload);
    const broken = operator.slice(0, -1) + (operator.endsWith("q") ? "p" : "q");
    expect(operatorToAccountBech32(broken)).toBeNull();
  });
});
