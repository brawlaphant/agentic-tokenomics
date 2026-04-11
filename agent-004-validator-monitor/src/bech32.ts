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

const HRP_CONS = "regenvalcons";
const VALOPER_SUFFIX = "valoper";

/**
 * Convert an operator bech32 (e.g. `regenvaloper1…`) to the matching
 * delegator bech32 (`regen1…`). Cosmos encodes both forms from the
 * same 20-byte payload; only the HRP differs. The delegator prefix
 * is the operator prefix with the trailing `"valoper"` stripped, so
 * this helper works across chains — it produces `cosmos1…` for
 * `cosmosvaloper1…`, `osmo1…` for `osmovaloper1…`, etc.
 *
 * Returns `null` when the input is not a valid bech32 string, or
 * when its HRP does not end in `"valoper"`, or when stripping
 * `"valoper"` would leave an empty prefix. Callers must guard
 * against null — a missing conversion means the validator cannot
 * be queried for votes and should degrade to a 0 governance score
 * rather than crash the whole cycle.
 */
export function operatorToDelegator(operatorAddress: string): string | null {
  try {
    const decoded = bech32.decode(operatorAddress);
    if (!decoded.prefix.endsWith(VALOPER_SUFFIX)) return null;
    const delegatorPrefix = decoded.prefix.slice(0, -VALOPER_SUFFIX.length);
    if (!delegatorPrefix) return null;
    return bech32.encode(delegatorPrefix, decoded.words);
  } catch {
    return null;
  }
}

/** Convert a `regen1…` delegator bech32 back to the `regenvaloper1…` form. */
export function delegatorToOperator(delegatorAddress: string): string | null {
  try {
    const decoded = bech32.decode(delegatorAddress);
    if (decoded.prefix.endsWith(VALOPER_SUFFIX)) return null;
    return bech32.encode(decoded.prefix + VALOPER_SUFFIX, decoded.words);
  } catch {
    return null;
  }
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
