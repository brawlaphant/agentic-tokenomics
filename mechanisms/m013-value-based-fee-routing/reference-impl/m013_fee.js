/**
 * m013 — Value-Based Fee Routing: reference implementation (v0).
 *
 * Computes value-proportional fees for ecological credit transactions
 * and distributes fee revenue across four purpose-specific pools.
 *
 * All monetary values are in uregen (1 REGEN = 1,000,000 uregen).
 *
 * @module m013_fee
 */

/** Default fee rates by transaction type (v0 Model A). */
export const DEFAULT_FEE_RATES = {
  CreditIssuance: 0.02,
  CreditTransfer: 0.001,
  CreditRetirement: 0.005,
  MarketplaceTrade: 0.01
};

/** Default distribution shares (v0 Model A). */
export const DEFAULT_DISTRIBUTION_SHARES = {
  burn: 0.30,
  validator: 0.40,
  community: 0.25,
  agent: 0.05
};

/** Default minimum fee in uregen (1 REGEN). */
export const DEFAULT_MIN_FEE_UREGEN = 1000000;

/**
 * Compute the fee and distribution for a single credit transaction.
 *
 * @param {Object} opts
 * @param {string} opts.tx_type - One of: CreditIssuance, CreditTransfer, CreditRetirement, MarketplaceTrade
 * @param {number} opts.value - Transaction value in uregen
 * @param {Object} [opts.fee_config] - Fee configuration (rates, shares, min_fee)
 * @param {Object} [opts.fee_config.fee_rates] - Fee rates by tx type
 * @param {Object} [opts.fee_config.distribution_shares] - Distribution shares (must sum to 1.0)
 * @param {number} [opts.fee_config.min_fee_uregen] - Minimum fee floor in uregen
 * @returns {{ fee_amount: number, min_fee_applied: boolean, distribution: { burn: number, validator: number, community: number, agent: number } }}
 */
export function computeFee({ tx_type, value, fee_config }) {
  const rates = fee_config?.fee_rates ?? DEFAULT_FEE_RATES;
  const shares = fee_config?.distribution_shares ?? DEFAULT_DISTRIBUTION_SHARES;
  const minFee = fee_config?.min_fee_uregen ?? DEFAULT_MIN_FEE_UREGEN;

  const rate = rates[tx_type];
  if (rate === undefined) {
    throw new Error(`Unknown tx_type: ${tx_type}`);
  }

  const rawFee = Math.floor(value * rate);
  const minFeeApplied = rawFee < minFee;
  const feeAmount = Math.max(rawFee, minFee);

  // Floor 3 pools, derive validator as remainder to preserve Fee Conservation invariant
  const d_burn = Math.floor(feeAmount * shares.burn);
  const d_community = Math.floor(feeAmount * shares.community);
  const d_agent = Math.floor(feeAmount * shares.agent);
  const d_validator = feeAmount - d_burn - d_community - d_agent;

  const distribution = {
    burn: d_burn,
    validator: d_validator,
    community: d_community,
    agent: d_agent
  };

  return {
    fee_amount: feeAmount,
    min_fee_applied: minFeeApplied,
    distribution
  };
}

// ---------------------------------------------------------------------------
// Self-test harness: discovers every *.input.json file in test_vectors/
// and checks each against its matching *.expected.json sibling. Adding a
// new edge-case vector is a zero-touch change to this harness — just drop
// both files into test_vectors/ and rerun.
// ---------------------------------------------------------------------------
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import path from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const vectorDir = path.join(__dirname, "test_vectors");

function runVector(inputFile) {
  const base = inputFile.replace(".input.json", "");
  const inputPath = path.join(vectorDir, inputFile);
  const expectedPath = path.join(vectorDir, `${base}.expected.json`);

  if (!fs.existsSync(inputPath) || !fs.existsSync(expectedPath)) {
    console.error(`MISSING vector files for ${base}`);
    return { failures: 1, events: 0 };
  }

  const input = JSON.parse(fs.readFileSync(inputPath, "utf8"));
  const expected = JSON.parse(fs.readFileSync(expectedPath, "utf8"));

  const feeConfig = input.fee_config;
  let failures = 0;

  for (let i = 0; i < input.fee_events.length; i++) {
    const ev = input.fee_events[i];
    const result = computeFee({
      tx_type: ev.tx_type,
      value: ev.value_uregen,
      fee_config: feeConfig
    });

    const exp = expected.fee_results[i];
    if (!exp) {
      console.error(`MISSING expected[${i}] for ${base}`);
      failures++;
      continue;
    }

    if (result.fee_amount !== exp.fee_amount_uregen) {
      console.error(
        `FAIL ${base} [${ev.tx_hash}]: fee_amount ${result.fee_amount} !== expected ${exp.fee_amount_uregen}`
      );
      failures++;
    }
    if (result.min_fee_applied !== exp.min_fee_applied) {
      console.error(
        `FAIL ${base} [${ev.tx_hash}]: min_fee_applied ${result.min_fee_applied} !== expected ${exp.min_fee_applied}`
      );
      failures++;
    }
    for (const pool of ["burn", "validator", "community", "agent"]) {
      if (result.distribution[pool] !== exp.distribution[pool]) {
        console.error(
          `FAIL ${base} [${ev.tx_hash}]: distribution.${pool} ${result.distribution[pool]} !== expected ${exp.distribution[pool]}`
        );
        failures++;
      }
    }
  }

  return { failures, events: input.fee_events.length };
}

if (fs.existsSync(vectorDir)) {
  const inputFiles = fs
    .readdirSync(vectorDir)
    .filter((f) => f.endsWith(".input.json"))
    .sort();

  let totalFailures = 0;
  let totalEvents = 0;

  for (const inputFile of inputFiles) {
    const { failures, events } = runVector(inputFile);
    totalFailures += failures;
    totalEvents += events;
  }

  if (totalFailures > 0) {
    console.error(`\nm013_fee self-test: ${totalFailures} failure(s)`);
    process.exit(1);
  }

  console.log(
    `m013_fee self-test: PASS (${totalEvents} events across ${inputFiles.length} vectors)`
  );
}
