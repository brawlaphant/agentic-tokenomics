/**
 * v0 (advisory): 4-factor weighted composite scoring for bonded attestations.
 *
 * Factors:
 *   bond_adequacy       (weight 0.30): Bond amount relative to minimum for type
 *   attester_reputation  (weight 0.30): M010 reputation score for attester
 *   evidence_completeness(weight 0.25): Attestation document completeness
 *   type_risk            (weight 0.15): Attestation type risk factor
 *
 * See SPEC.md section 5 for full formula.
 *
 * @param {Object} opts
 * @param {Object} opts.attestation - Attestation with type, bond amount
 * @param {Object} opts.factors - Pre-computed factor scores (each 0-1000)
 * @returns {{ score: number, confidence: number, factors: Object }}
 */
export function computeM008Score({ attestation, factors }) {
  const W_BOND = 0.30;
  const W_REPUTATION = 0.30;
  const W_EVIDENCE = 0.25;
  const W_TYPE = 0.15;

  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

  const fBond = clamp(factors.bond_adequacy ?? 0, 0, 1000);
  const fRep = clamp(factors.attester_reputation ?? 300, 0, 1000);
  const fEvid = clamp(factors.evidence_completeness ?? 0, 0, 1000);
  const fType = clamp(factors.type_risk ?? 0, 0, 1000);

  const score = Math.round(
    W_BOND * fBond +
    W_REPUTATION * fRep +
    W_EVIDENCE * fEvid +
    W_TYPE * fType
  );

  const confidence = computeConfidence(factors);

  return {
    score: clamp(score, 0, 1000),
    confidence,
    factors: {
      bond_adequacy: fBond,
      attester_reputation: fRep,
      evidence_completeness: fEvid,
      type_risk: fType
    }
  };
}

/**
 * Compute bond adequacy factor.
 * Bonds at minimum get 500; bonds at 2x minimum get 1000 (capped).
 *
 * @param {number} bondAmount - Actual bond amount
 * @param {number} minBond - Minimum bond for attestation type
 * @returns {number} f_bond (0-1000)
 */
export function computeBondAdequacy(bondAmount, minBond) {
  if (minBond <= 0) return 0;
  return Math.min(1000, Math.round((bondAmount / minBond) * 500));
}

/**
 * Get type risk factor by attestation type.
 * @param {string} attestationType
 * @returns {number} f_type (0-1000)
 */
export function getTypeRiskFactor(attestationType) {
  const TYPE_RISK = {
    MethodologyValidation: 1000,
    CreditIssuanceClaim: 800,
    BaselineMeasurement: 600,
    ProjectBoundary: 400
  };
  return TYPE_RISK[attestationType] ?? 0;
}

/**
 * Get minimum bond for attestation type.
 * @param {string} attestationType
 * @returns {number} minimum bond in REGEN
 */
export function getMinBond(attestationType) {
  const MIN_BONDS = {
    ProjectBoundary: 500,
    BaselineMeasurement: 1000,
    CreditIssuanceClaim: 2000,
    MethodologyValidation: 5000
  };
  return MIN_BONDS[attestationType] ?? 0;
}

function computeConfidence(factors) {
  let available = 0;
  const total = 4;
  if (factors.reputation_available) available++;
  if (factors.iri_resolvable !== false) available++;
  if (factors.has_prior_attestations) available++;
  if (factors.type_recognized !== false) available++;
  return Math.round((available / total) * 1000);
}

// --- Self-test ---
//
// Discovers every *.input.json file in the test_vectors/ directory and
// checks each one against its matching *.expected.json sibling. Adding
// a new edge-case vector is a zero-touch change to this file — just
// drop both files into test_vectors/ and rerun.
const isMain = typeof process !== "undefined" &&
  process.argv[1] &&
  (process.argv[1].endsWith("m008_score.js") || process.argv[1].endsWith("m008_score"));

if (isMain) {
  const fs = await import("node:fs");
  const path = await import("node:path");
  const url = await import("node:url");

  const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
  const vectorsDir = path.join(__dirname, "test_vectors");
  const entries = fs.readdirSync(vectorsDir);
  const inputFiles = entries
    .filter((f) => f.endsWith(".input.json"))
    .sort();

  let pass = true;
  let totalChecked = 0;

  for (const inputFile of inputFiles) {
    const name = inputFile.replace(".input.json", "");
    const expectedFile = `${name}.expected.json`;
    if (!entries.includes(expectedFile)) {
      console.error(`MISSING expected file for ${inputFile}`);
      pass = false;
      continue;
    }

    const input = JSON.parse(fs.readFileSync(path.join(vectorsDir, inputFile), "utf8"));
    const expected = JSON.parse(fs.readFileSync(path.join(vectorsDir, expectedFile), "utf8"));

    const results = input.attestations.map((a) => computeM008Score({
      attestation: a.attestation,
      factors: a.factors
    }));

    for (let i = 0; i < results.length; i++) {
      const r = results[i];
      const e = expected.scores[i];
      if (!e) {
        console.error(`MISSING expected[${i}] in ${expectedFile}`);
        pass = false;
        continue;
      }
      const confidenceMatch =
        e.confidence === undefined || r.confidence === e.confidence;
      const factorsMatch =
        e.factors === undefined ||
        JSON.stringify(r.factors) === JSON.stringify(e.factors);
      if (r.score !== e.score || !confidenceMatch || !factorsMatch) {
        console.error(
          `FAIL ${name}[${i}]: got ${JSON.stringify({
            score: r.score,
            confidence: r.confidence,
            factors: r.factors,
          })}, expected ${JSON.stringify({
            score: e.score,
            confidence: e.confidence,
            factors: e.factors,
          })}`
        );
        pass = false;
      } else {
        totalChecked++;
      }
    }
  }

  if (pass) {
    console.log(
      `m008_score self-test: PASS (${totalChecked} attestations across ${inputFiles.length} vectors)`
    );
  } else {
    process.exit(1);
  }
}
