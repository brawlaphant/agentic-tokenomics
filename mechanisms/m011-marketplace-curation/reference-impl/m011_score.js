/**
 * v0 (advisory): 7-factor weighted composite quality scoring for credit batches.
 *
 * Factors:
 *   project_reputation        (weight 0.25): M010 reputation for the project
 *   class_reputation          (weight 0.20): M010 reputation for the credit class
 *   vintage_freshness         (weight 0.15): Linear decay from issuance to 10 years
 *   verification_recency      (weight 0.15): Linear decay from last verification to 3 years
 *   seller_reputation         (weight 0.10): M010 reputation for the seller
 *   price_fairness            (weight 0.10): Deviation from class median price
 *   additionality_confidence  (weight 0.05): Methodology additionality assessment
 *
 * See SPEC.md section 5 for full formula.
 *
 * @param {Object} opts
 * @param {Object} opts.batch - Batch metadata
 * @param {Object} opts.factors - Pre-computed factor scores (each 0-1000)
 * @returns {{ score: number, confidence: number, factors: Object }}
 */
export function computeM011Score({ batch, factors }) {
  const W_PROJECT = 0.25;
  const W_CLASS = 0.20;
  const W_VINTAGE = 0.15;
  const W_VERIFICATION = 0.15;
  const W_SELLER = 0.10;
  const W_PRICE = 0.10;
  const W_ADDITIONALITY = 0.05;

  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

  const fProject = clamp(factors.project_reputation ?? 0, 0, 1000);
  const fClass = clamp(factors.class_reputation ?? 0, 0, 1000);
  const fVintage = clamp(factors.vintage_freshness ?? 0, 0, 1000);
  const fVerification = clamp(factors.verification_recency ?? 0, 0, 1000);
  const fSeller = clamp(factors.seller_reputation ?? 0, 0, 1000);
  const fPrice = clamp(factors.price_fairness ?? 0, 0, 1000);
  const fAdditionality = clamp(factors.additionality_confidence ?? 0, 0, 1000);

  const score = Math.round(
    W_PROJECT * fProject +
    W_CLASS * fClass +
    W_VINTAGE * fVintage +
    W_VERIFICATION * fVerification +
    W_SELLER * fSeller +
    W_PRICE * fPrice +
    W_ADDITIONALITY * fAdditionality
  );

  const confidence = computeConfidence(factors);

  return {
    score: clamp(score, 0, 1000),
    confidence,
    factors: {
      project_reputation: fProject,
      class_reputation: fClass,
      vintage_freshness: fVintage,
      verification_recency: fVerification,
      seller_reputation: fSeller,
      price_fairness: fPrice,
      additionality_confidence: fAdditionality
    }
  };
}

/**
 * Compute vintage freshness factor.
 * Linear decay: 1000 at issuance, 0 at 10 years.
 *
 * @param {number} ageYears - Years since batch start date
 * @returns {number} 0-1000
 */
export function computeVintageFreshness(ageYears) {
  if (ageYears <= 0) return 1000;
  if (ageYears >= 10) return 0;
  return Math.round((1.0 - ageYears / 10) * 1000);
}

/**
 * Compute verification recency factor.
 * Linear decay: 1000 at last verification, 0 at 3 years.
 *
 * @param {number} ageYears - Years since last verification
 * @returns {number} 0-1000
 */
export function computeVerificationRecency(ageYears) {
  if (ageYears <= 0) return 1000;
  if (ageYears >= 3) return 0;
  return Math.round((1.0 - ageYears / 3) * 1000);
}

/**
 * Compute price fairness factor.
 * 1000 at median, 0 at ±50% deviation.
 *
 * @param {number} listingPrice
 * @param {number} medianPrice
 * @returns {number} 0-1000
 */
export function computePriceFairness(listingPrice, medianPrice) {
  if (medianPrice <= 0) return 0;
  const deviation = Math.abs(listingPrice - medianPrice) / medianPrice;
  return Math.max(0, Math.round((1.0 - deviation * 2) * 1000));
}

function computeConfidence(factors) {
  let available = 0;
  const total = 7;
  if (factors.project_reputation_available) available++;
  if (factors.class_reputation_available) available++;
  if (factors.seller_reputation_available) available++;
  if (factors.vintage_known !== false) available++;
  if (factors.verification_date_known) available++;
  if (factors.price_data_available) available++;
  if (factors.methodology_available) available++;
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
  (process.argv[1].endsWith("m011_score.js") || process.argv[1].endsWith("m011_score"));

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

    const results = input.batches.map((b) => computeM011Score({
      batch: b.batch,
      factors: b.factors
    }));

    for (let i = 0; i < results.length; i++) {
      const r = results[i];
      const e = expected.scores[i];
      if (!e) {
        console.error(`MISSING expected[${i}] in ${expectedFile}`);
        pass = false;
        continue;
      }
      if (r.score !== e.score) {
        console.error(
          `FAIL ${name}[${i}]: got score=${r.score}, expected score=${e.score}`
        );
        pass = false;
      } else {
        totalChecked++;
      }
    }
  }

  if (pass) {
    console.log(
      `m011_score self-test: PASS (${totalChecked} batches across ${inputFiles.length} vectors)`
    );
  } else {
    process.exit(1);
  }
}
