import { createHash } from "node:crypto";
import { bech32 } from "bech32";

/**
 * Cosmos bech32 derivation helpers.
 *
 * These are the minimum set of conversions the validator monitor
 * needs to join across the three address spaces that Cosmos uses:
 *
 *   - operator bech32  (`regenvaloper1…`)  — governs the validator
 *   - delegator bech32 (`regen1…`)         — votes in governance
 *   - consensus bech32 (`regenvalcons1…`)  — signs blocks
 *
 * The agent cannot reliably match a validator against its slashing
 * signing info or its governance votes without these conversions —
 * that mismatch was the bug behind AGENT-004's uptime always showing
 * 100% and governance participation always showing 0.
 */

const HRP_OPERATOR = "regenvaloper";
const HRP_ACCOUNT = "regen";
const HRP_CONS = "regenvalcons";

/** Re-encode a bech32 address under a different human-readable prefix. */
function reencodeBech32(addr: string, newHrp: string): string | null {
  try {
    const decoded = bech32.decode(addr);
    return bech32.encode(newHrp, decoded.words);
  } catch {
    return null;
  }
}

/** Convert `regenvaloper1…` to the underlying `regen1…` delegator bech32. */
export function operatorToDelegator(operatorAddress: string): string | null {
  return reencodeBech32(operatorAddress, HRP_ACCOUNT);
}

/** Convert `regen1…` delegator bech32 back to the operator form. */
export function delegatorToOperator(delegatorAddress: string): string | null {
  return reencodeBech32(delegatorAddress, HRP_OPERATOR);
}

/**
 * Derive the `regenvalcons1…` consensus address from a validator's
 * ed25519 consensus public key. The pubkey bytes are SHA256-hashed
 * and the first 20 bytes of the digest are bech32-encoded under the
 * consensus HRP. This matches the standard Cosmos derivation in
 * `tmcrypto.PubKey.Address()`.
 *
 * `pubkeyBase64` is the base64 string that Cosmos LCDs expose under
 * `validator.consensus_pubkey.key`.
 */
export function consensusPubkeyToConsAddress(
  pubkeyBase64: string
): string | null {
  if (!pubkeyBase64) return null;
  let pubkeyBytes: Buffer;
  try {
    pubkeyBytes = Buffer.from(pubkeyBase64, "base64");
  } catch {
    return null;
  }
  if (pubkeyBytes.length === 0) return null;

  const digest = createHash("sha256").update(pubkeyBytes).digest();
  const addressBytes = digest.subarray(0, 20);
  try {
    return bech32.encode(HRP_CONS, bech32.toWords(addressBytes));
  } catch {
    return null;
  }
}
