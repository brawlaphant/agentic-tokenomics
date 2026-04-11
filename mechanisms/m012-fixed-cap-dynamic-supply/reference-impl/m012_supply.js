/**
 * m012 — Fixed Cap Dynamic Supply: canonical reference computation.
 *
 * Supply algorithm (SPEC.md section 5):
 *   S[t+1] = S[t] + M[t] - B[t]
 *   M[t] = r * (C - S[t])
 *   r = r_base * effective_multiplier * ecological_multiplier
 *
 * All token amounts are in uregen (1 REGEN = 1,000,000 uregen).
 * Uses BigInt for supply arithmetic to avoid floating-point precision issues.
 *
 * @module m012_supply
 */

const DEFAULT_HARD_CAP = 221_000_000_000_000n; // 221M REGEN in uregen
const DEFAULT_R_BASE = 0.02;

/**
 * Clamp a number to [min, max].
 * @param {number} v
 * @param {number} lo
 * @param {number} hi
 * @returns {number}
 */
function clamp(v, lo, hi) {
  return Math.max(lo, Math.min(hi, v));
}

/**
 * Compute a single supply period.
 *
 * @param {Object} opts
 * @param {Object} opts.supply_state
 * @param {string} opts.supply_state.current_supply - uregen (string for BigInt)
 * @param {string} [opts.supply_state.hard_cap] - uregen (string, default 221T)
 * @param {string} opts.supply_state.staked_amount - uregen (string)
 * @param {string} [opts.supply_state.stability_committed] - uregen (string, default "0")
 * @param {string} [opts.supply_state.m014_state] - INACTIVE | TRANSITION | ACTIVE | EQUILIBRIUM
 * @param {boolean} [opts.supply_state.ecological_multiplier_enabled] - false in v0
 * @param {number} [opts.supply_state.delta_co2] - ppm, only when eco enabled
 * @param {number} [opts.supply_state.ecological_reference_value] - ppm, default 50
 * @param {string} opts.burn_amount - uregen (string)
 * @param {Object} [opts.config] - override defaults
 * @param {number} [opts.config.r_base] - base regrowth rate (default 0.02)
 * @returns {{ next_supply: string, minted: string, burned: string, regrowth_rate: number, effective_multiplier: number, staking_multiplier: number, stability_multiplier: number, ecological_multiplier: number, headroom_before: string, headroom_after: string }}
 */
export function computeSupplyPeriod({ supply_state, burn_amount, config }) {
  const r_base = config?.r_base ?? DEFAULT_R_BASE;

  // Parse BigInt values from strings
  const S = BigInt(supply_state.current_supply);
  const C = BigInt(supply_state.hard_cap ?? DEFAULT_HARD_CAP.toString());
  const staked = BigInt(supply_state.staked_amount);
  const stabilityCommitted = BigInt(supply_state.stability_committed ?? "0");
  const B = BigInt(burn_amount);
  const m014State = supply_state.m014_state ?? "INACTIVE";

  // Validate parameter bounds (Security Invariant 5)
  if (r_base < 0 || r_base > 0.10) {
    throw new RangeError(`r_base must be in [0, 0.10], got ${r_base}`);
  }

  // Compute staking_multiplier: 1 + (S_staked / S_total), clamped [1.0, 2.0]
  const sFloat = Number(S);
  const stakingMult = sFloat > 0
    ? clamp(1 + Number(staked) / sFloat, 1.0, 2.0)
    : 1.0;

  // Compute stability_multiplier: 1 + (S_stability_committed / S_total), clamped [1.0, 2.0]
  const stabilityMult = sFloat > 0
    ? clamp(1 + Number(stabilityCommitted) / sFloat, 1.0, 2.0)
    : 1.0;

  // Phase-gated effective multiplier (SPEC.md section 5.3)
  let effectiveMult;
  if (m014State === "INACTIVE") {
    effectiveMult = stakingMult;
  } else if (m014State === "TRANSITION") {
    effectiveMult = Math.max(stakingMult, stabilityMult);
  } else {
    // ACTIVE or EQUILIBRIUM
    effectiveMult = stabilityMult;
  }

  // Ecological multiplier (SPEC.md section 5.4)
  let ecoMult = 1.0;
  if (supply_state.ecological_multiplier_enabled) {
    const deltaCo2 = supply_state.delta_co2 ?? 0;
    const refValue = supply_state.ecological_reference_value ?? 50;
    ecoMult = Math.max(0, 1 - (deltaCo2 / refValue));
  }

  // Regrowth rate
  const r = r_base * effectiveMult * ecoMult;

  // Headroom before: C - S[t]
  const headroomBefore = C > S ? C - S : 0n;

  // Minting: M[t] = r * (C - S[t])
  // Use floating point for rate * headroom, then truncate to BigInt
  const mintedFloat = r * Number(headroomBefore);
  const M = BigInt(Math.floor(mintedFloat));

  // Next supply: S[t+1] = S[t] + M[t] - B[t]
  // With safety: cap inviolability and non-negative supply
  let nextSupply = S + M - B;

  // Security Invariant 1: Cap Inviolability
  if (nextSupply > C) {
    nextSupply = C;
  }

  // Security Invariant 2: Non-Negative Supply
  if (nextSupply < 0n) {
    nextSupply = 0n;
  }

  const headroomAfter = C - nextSupply;

  return {
    next_supply: nextSupply.toString(),
    minted: M.toString(),
    burned: B.toString(),
    regrowth_rate: r,
    effective_multiplier: effectiveMult,
    staking_multiplier: stakingMult,
    stability_multiplier: stabilityMult,
    ecological_multiplier: ecoMult,
    headroom_before: headroomBefore.toString(),
    headroom_after: headroomAfter.toString()
  };
}

// --------------- Self-test harness ---------------
//
// Discovers every *.input.json file in test_vectors/ and checks each
// one against its matching *.expected.json sibling. Adding a new
// edge-case vector is a zero-touch change to this harness — just drop
// both files into test_vectors/ and rerun.

import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function runVector(inputFile) {
  const base = inputFile.replace(".input.json", "");
  const inputPath = join(__dirname, "test_vectors", inputFile);
  const expectedPath = join(__dirname, "test_vectors", `${base}.expected.json`);

  const input = JSON.parse(readFileSync(inputPath, "utf8"));
  const expected = JSON.parse(readFileSync(expectedPath, "utf8"));

  let pass = 0;
  let fail = 0;

  for (let i = 0; i < input.periods.length; i++) {
    const p = input.periods[i];
    const exp = expected.periods[i];

    const result = computeSupplyPeriod({
      supply_state: p.supply_state,
      burn_amount: p.burn_amount,
      config: input.config
    });

    const checks = [
      ["next_supply", result.next_supply, exp.next_supply],
      ["minted", result.minted, exp.minted],
      ["burned", result.burned, exp.burned],
      ["regrowth_rate", result.regrowth_rate, exp.regrowth_rate],
      ["effective_multiplier", result.effective_multiplier, exp.effective_multiplier],
      ["staking_multiplier", result.staking_multiplier, exp.staking_multiplier],
      ["ecological_multiplier", result.ecological_multiplier, exp.ecological_multiplier],
      ["headroom_before", result.headroom_before, exp.headroom_before],
      ["headroom_after", result.headroom_after, exp.headroom_after]
    ];

    for (const [field, got, want] of checks) {
      if (String(got) !== String(want)) {
        console.error(
          `FAIL ${base} period ${i + 1} (${p.label}) ${field}: got ${got}, want ${want}`
        );
        fail++;
      } else {
        pass++;
      }
    }
  }

  return { pass, fail, vectorName: base, periodCount: input.periods.length };
}

function selfTest() {
  const vectorsDir = join(__dirname, "test_vectors");
  const inputFiles = readdirSync(vectorsDir)
    .filter((f) => f.endsWith(".input.json"))
    .sort();

  let totalPass = 0;
  let totalFail = 0;
  let totalPeriods = 0;

  for (const inputFile of inputFiles) {
    const { pass, fail, periodCount } = runVector(inputFile);
    totalPass += pass;
    totalFail += fail;
    totalPeriods += periodCount;
  }

  console.log(
    `m012_supply self-test: ${totalPass} passed, ${totalFail} failed (` +
    `${totalPeriods} periods across ${inputFiles.length} vectors)`
  );
  if (totalFail > 0) process.exit(1);
}

// Run self-test when executed directly
if (process.argv[1] === __filename) {
  selfTest();
}
