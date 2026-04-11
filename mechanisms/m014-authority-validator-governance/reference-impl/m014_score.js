/**
 * m014 Authority Validator Governance — Performance Score
 *
 * Computes a composite performance score for a validator based on three
 * weighted factors:
 *   - uptime:                   weight 0.4  (blocks_signed / blocks_expected)
 *   - governance_participation: weight 0.3  (votes_cast / proposals_available)
 *   - ecosystem_contribution:   weight 0.3  (AGENT-004 assessed score, 0–1)
 *
 * Confidence is based on data availability:
 *   - 3 factors present → 1.0
 *   - 2 factors present → 0.67
 *   - 1 factor present  → 0.33
 *   - 0 factors         → 0.0
 *
 * See SPEC.md section 5 for full specification.
 *
 * @param {Object} opts
 * @param {Object} opts.validator - { address }
 * @param {Object} opts.factors   - { uptime, governance_participation, ecosystem_contribution }
 *        Each factor is a number 0–1 or null/undefined if unavailable.
 * @returns {{ address: string, performance_score: number, confidence: number,
 *             factors: Object, flags: string[] }}
 */
export function computeM014Score({ validator, factors }) {
  const WEIGHTS = {
    uptime: 0.4,
    governance_participation: 0.3,
    ecosystem_contribution: 0.3
  };

  const FACTOR_KEYS = Object.keys(WEIGHTS);
  const address = validator?.address ?? "unknown";

  // Determine available factors
  let availableCount = 0;
  const resolvedFactors = {};

  for (const key of FACTOR_KEYS) {
    const val = factors?.[key];
    if (val != null && Number.isFinite(val)) {
      resolvedFactors[key] = Math.max(0, Math.min(1, val));
      availableCount++;
    } else {
      resolvedFactors[key] = null;
    }
  }

  // Confidence based on data availability
  const confidenceMap = { 3: 1.0, 2: 0.67, 1: 0.33, 0: 0.0 };
  const confidence = confidenceMap[availableCount] ?? 0.0;

  // Compute weighted score using only available factors
  let weightedSum = 0;
  let totalWeight = 0;

  for (const key of FACTOR_KEYS) {
    if (resolvedFactors[key] !== null) {
      weightedSum += resolvedFactors[key] * WEIGHTS[key];
      totalWeight += WEIGHTS[key];
    }
  }

  // When all factors present, use direct weighted sum.
  // When some are missing, re-normalize based on available weights.
  const performance_score = totalWeight > 0
    ? weightedSum / totalWeight
    : 0.0;

  const normalizedScore = Number(performance_score.toFixed(4));

  // Performance flags
  const flags = [];
  if (normalizedScore < 0.70 && availableCount > 0) {
    flags.push("below_performance_threshold");
  }
  if (resolvedFactors.uptime !== null && resolvedFactors.uptime < 0.995) {
    flags.push("below_uptime_minimum");
  }
  if (flags.includes("below_performance_threshold") || flags.includes("below_uptime_minimum")) {
    flags.push("probation_recommended");
  }

  return {
    address,
    performance_score: normalizedScore,
    confidence,
    factors: {
      uptime: resolvedFactors.uptime,
      governance_participation: resolvedFactors.governance_participation,
      ecosystem_contribution: resolvedFactors.ecosystem_contribution
    },
    flags
  };
}

// ── Self-test harness ──────────────────────────────────────────────────
// Run: node mechanisms/m014-authority-validator-governance/reference-impl/m014_score.js
//
// Discovers every *.input.json file in test_vectors/ and checks each one
// against its matching *.expected.json sibling. Adding a new edge-case
// vector is a zero-touch change to this harness — just drop both files
// into test_vectors/ and rerun.
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

function runVector(inputFile) {
  const base = inputFile.replace(".input.json", "");
  const inputPath = path.join(__dirname, "test_vectors", inputFile);
  const expectedPath = path.join(__dirname, "test_vectors", `${base}.expected.json`);

  const input = JSON.parse(readFileSync(inputPath, "utf8"));
  const expected = JSON.parse(readFileSync(expectedPath, "utf8"));

  let pass = 0;
  let fail = 0;

  for (const validatorInput of input.validators) {
    const result = computeM014Score({
      validator: { address: validatorInput.address },
      factors: validatorInput.factors
    });

    const exp = expected.scores.find((s) => s.address === validatorInput.address);
    if (!exp) {
      console.error(`  FAIL ${base}: no expected output for ${validatorInput.address}`);
      fail++;
      continue;
    }

    const scoreMatch = Math.abs(result.performance_score - exp.performance_score) < 0.0002;
    const confMatch = Math.abs(result.confidence - exp.confidence) < 0.01;
    const flagsMatch =
      JSON.stringify(result.flags.sort()) === JSON.stringify((exp.flags ?? []).sort());

    if (scoreMatch && confMatch && flagsMatch) {
      pass++;
    } else {
      console.error(`  FAIL ${base}: ${validatorInput.address}`);
      if (!scoreMatch)
        console.error(`    score: got ${result.performance_score}, expected ${exp.performance_score}`);
      if (!confMatch)
        console.error(`    confidence: got ${result.confidence}, expected ${exp.confidence}`);
      if (!flagsMatch)
        console.error(`    flags: got [${result.flags}], expected [${exp.flags}]`);
      fail++;
    }
  }

  return { pass, fail };
}

function selfTest() {
  const vectorsDir = path.join(__dirname, "test_vectors");
  const inputFiles = readdirSync(vectorsDir)
    .filter((f) => f.endsWith(".input.json"))
    .sort();

  let totalPass = 0;
  let totalFail = 0;

  for (const inputFile of inputFiles) {
    const { pass, fail } = runVector(inputFile);
    totalPass += pass;
    totalFail += fail;
  }

  console.log(
    `m014_score self-test: ${totalPass} passed, ${totalFail} failed ` +
      `(across ${inputFiles.length} vectors)`
  );
  if (totalFail > 0) process.exit(1);
}

// Run self-test if executed directly
if (process.argv[1] === __filename) {
  selfTest();
}
