/**
 * v0 (advisory): 4-factor weighted composite scoring for milestone deliverable review.
 *
 * Factors:
 *   deliverable_quality    (weight 0.40): Methodology compliance, technical quality
 *   evidence_completeness  (weight 0.25): Evidence IRI resolvability, document completeness
 *   milestone_consistency  (weight 0.20): Consistency with prior milestones and spec
 *   provider_reputation    (weight 0.15): M010 reputation score for provider
 *
 * See SPEC.md section 5 for full formula.
 *
 * @param {Object} opts
 * @param {Object} opts.milestone - Milestone metadata
 * @param {Object} opts.factors - Pre-computed factor scores (each 0-1000)
 * @returns {{ score: number, confidence: number, recommendation: string, factors: Object }}
 */
export function computeM009Score({ milestone, factors }) {
  const W_QUALITY = 0.40;
  const W_EVIDENCE = 0.25;
  const W_CONSISTENCY = 0.20;
  const W_REPUTATION = 0.15;

  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

  const fQuality = clamp(factors.deliverable_quality ?? 0, 0, 1000);
  const fEvidence = clamp(factors.evidence_completeness ?? 0, 0, 1000);
  const fConsistency = clamp(factors.milestone_consistency ?? 0, 0, 1000);
  const fReputation = clamp(factors.provider_reputation ?? 300, 0, 1000);

  const score = Math.round(
    W_QUALITY * fQuality +
    W_EVIDENCE * fEvidence +
    W_CONSISTENCY * fConsistency +
    W_REPUTATION * fReputation
  );

  const confidence = computeConfidence(factors);
  const recommendation = computeRecommendation(clamp(score, 0, 1000), confidence);

  return {
    score: clamp(score, 0, 1000),
    confidence,
    recommendation,
    factors: {
      deliverable_quality: fQuality,
      evidence_completeness: fEvidence,
      milestone_consistency: fConsistency,
      provider_reputation: fReputation
    }
  };
}

/**
 * Compute recommendation based on score and confidence.
 * @param {number} score
 * @param {number} confidence
 * @returns {string}
 */
function computeRecommendation(score, confidence) {
  if (score >= 700 && confidence >= 750) return "APPROVE";
  if (score < 400 || confidence < 250) return "FLAG_FOR_CLIENT";
  return "NEEDS_REVISION";
}

function computeConfidence(factors) {
  let available = 0;
  const total = 4;
  if (factors.reputation_available === true) available++;
  if (factors.iri_resolvable === true) available++;
  if (factors.has_prior_milestones === true) available++;
  if (factors.spec_available === true) available++;
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
  (process.argv[1].endsWith("m009_score.js") || process.argv[1].endsWith("m009_score"));

if (isMain) {
  const fs = await import("node:fs");
  const path = await import("node:path");
  const url = await import("node:url");

  const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
  const vectorsDir = path.join(__dirname, "test_vectors");
  const entries = fs.existsSync(vectorsDir) ? fs.readdirSync(vectorsDir) : [];
  const inputFiles = entries
    .filter((f) => f.endsWith(".input.json"))
    .sort();

  if (inputFiles.length === 0) {
    console.error(`FAIL: no test vectors found in ${vectorsDir}`);
    process.exit(1);
  }

  let pass = true;
  let totalChecked = 0;

  for (const inputFile of inputFiles) {
    const name = inputFile.replace(/\.input\.json$/, "");
    const expectedFile = `${name}.expected.json`;
    if (!entries.includes(expectedFile)) {
      console.error(`MISSING expected file for ${inputFile}`);
      pass = false;
      continue;
    }

    let input;
    let expected;
    try {
      input = JSON.parse(fs.readFileSync(path.join(vectorsDir, inputFile), "utf8"));
      expected = JSON.parse(fs.readFileSync(path.join(vectorsDir, expectedFile), "utf8"));
    } catch (err) {
      console.error(`FAIL ${name}: could not parse vector files — ${err.message}`);
      pass = false;
      continue;
    }

    const results = input.milestones.map((m) => computeM009Score({
      milestone: m.milestone,
      factors: m.factors
    }));

    const expectedScores = expected.scores || [];
    for (let i = 0; i < results.length; i++) {
      const r = results[i];
      const e = expectedScores[i];
      if (!e) {
        console.error(`MISSING expected[${i}] in ${expectedFile}`);
        pass = false;
        continue;
      }
      let matched = true;
      if (r.score !== e.score) {
        console.error(
          `FAIL ${name}[${i}]: got score=${r.score}, expected score=${e.score}`
        );
        pass = false;
        matched = false;
      }
      if (r.recommendation !== e.recommendation) {
        console.error(
          `FAIL ${name}[${i}]: got recommendation=${r.recommendation}, expected=${e.recommendation}`
        );
        pass = false;
        matched = false;
      }
      if (e.confidence !== undefined && r.confidence !== e.confidence) {
        console.error(
          `FAIL ${name}[${i}]: got confidence=${r.confidence}, expected=${e.confidence}`
        );
        pass = false;
        matched = false;
      }
      if (matched) {
        totalChecked++;
      }
    }
  }

  if (pass) {
    console.log(
      `m009_score self-test: PASS (${totalChecked} milestones across ${inputFiles.length} vectors)`
    );
  } else {
    process.exit(1);
  }
}
