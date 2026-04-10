/**
 * m015 — Contribution-Weighted Rewards: KPI computation.
 *
 * Aggregates distribution results into a KPI block conforming to
 * schemas/m015_kpi.schema.json.
 *
 * KPI outputs:
 *   total_distributed_uregen      — sum of stability + activity distributions
 *   stability_allocation_uregen   — stability tier payout for the period
 *   activity_pool_uregen          — remaining pool after stability allocation
 *   stability_utilization         — ratio of stability allocation to cap
 *   participant_count             — participants with activity score > 0
 *   gini_coefficient              — inequality measure of reward distribution
 *   top_earner_share              — share of rewards going to highest scorer
 *   revenue_constraint_satisfied  — total payout <= community pool inflow
 *   stability_cap_satisfied       — stability allocation <= 30% cap
 *
 * All monetary values are in uregen (1 REGEN = 1,000,000 uregen).
 *
 * @module m015_kpi
 */

import {
  computeActivityScore,
  computeStabilityAllocation,
  computeDistribution,
} from "./m015_score.js";

/**
 * Compute the Gini coefficient for a set of values.
 * Returns 0 for perfectly equal distributions, approaches 1 for maximum inequality.
 *
 * @param {number[]} values - array of non-negative numeric values
 * @returns {number} Gini coefficient in [0, 1]
 */
export function giniCoefficient(values) {
  const n = values.length;
  if (n === 0) return 0;
  const sum = values.reduce((s, v) => s + v, 0);
  if (sum === 0) return 0;

  const sorted = [...values].sort((a, b) => a - b);
  let weightedSum = 0;
  for (let i = 0; i < n; i++) {
    weightedSum += (i + 1) * sorted[i];
  }
  // Gini = (2 * sum(rank_i * x_i)) / (n * sum) - (n + 1) / n
  return Number(((2 * weightedSum) / (n * sum) - (n + 1) / n).toFixed(6));
}

/**
 * Compute m015 KPI from a distribution period input.
 *
 * @param {Object} opts
 * @param {number} opts.community_pool_inflow_uregen - inflow for this period
 * @param {number} [opts.periods_per_year=52] - distribution periods per year
 * @param {number} [opts.max_stability_share=0.30] - cap on stability allocation
 * @param {Array} opts.stability_commitments - active stability commitments
 * @param {Array} opts.participants - participant records with { address, activities }
 * @returns {Object} KPI block
 */
export function computeM015KPI({
  community_pool_inflow_uregen,
  periods_per_year = 52,
  max_stability_share = 0.30,
  stability_commitments = [],
  participants = [],
}) {
  // 1. Stability allocation
  const { stability_allocation, activity_pool } = computeStabilityAllocation({
    community_pool_inflow: community_pool_inflow_uregen,
    stability_commitments,
    periods_per_year,
    max_stability_share,
  });

  // 2. Score each participant
  const scoredParticipants = participants.map((p) => {
    const result = computeActivityScore({ activities: p.activities });
    return { address: p.address, ...result };
  });

  // 3. Distribution
  const distribution = computeDistribution({
    activity_pool_amount: activity_pool,
    participants: scoredParticipants,
  });

  // 4. KPI metrics
  const totalActivityDistributed = distribution.reduce((s, d) => s + d.reward, 0);
  const total_distributed_uregen = stability_allocation + totalActivityDistributed;

  const activeParticipants = scoredParticipants.filter((p) => p.total_score > 0);
  const participant_count = activeParticipants.length;

  const stabilityCap = Math.floor(community_pool_inflow_uregen * max_stability_share);
  const stability_utilization = stabilityCap > 0
    ? Number((stability_allocation / stabilityCap).toFixed(6))
    : 0;

  const rewards = distribution.map((d) => d.reward);
  const gini = giniCoefficient(rewards);

  const topShare = distribution.length > 0
    ? Math.max(...distribution.map((d) => d.share))
    : 0;

  const revenue_constraint_satisfied = total_distributed_uregen <= community_pool_inflow_uregen;
  const stability_cap_satisfied = stability_allocation <= stabilityCap;

  return {
    total_distributed_uregen,
    stability_allocation_uregen: stability_allocation,
    activity_pool_uregen: activity_pool,
    stability_utilization,
    participant_count,
    gini_coefficient: gini,
    top_earner_share: topShare,
    revenue_constraint_satisfied,
    stability_cap_satisfied,
  };
}

// ---------------------------------------------------------------------------
// Self-test harness (runs when executed directly)
// ---------------------------------------------------------------------------
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function selfTest() {
  const vectorDir = join(__dirname, "test_vectors");
  const inputFiles = readdirSync(vectorDir)
    .filter((f) => f.endsWith(".input.json"))
    .sort();

  let passed = 0;
  let failed = 0;

  for (const inputFile of inputFiles) {
    const baseName = inputFile.replace(".input.json", "");
    const expectedFile = baseName + ".expected.json";
    const inputPath = join(vectorDir, inputFile);
    const expectedPath = join(vectorDir, expectedFile);

    console.log(`--- ${baseName} ---`);

    const input = JSON.parse(readFileSync(inputPath, "utf8"));
    const expected = JSON.parse(readFileSync(expectedPath, "utf8"));

    const kpi = computeM015KPI({
      community_pool_inflow_uregen: input.community_pool_inflow_uregen,
      periods_per_year: input.periods_per_year,
      max_stability_share: input.max_stability_share,
      stability_commitments: input.stability_commitments,
      participants: input.participants,
    });

    console.log(`  total_distributed:   ${kpi.total_distributed_uregen}`);
    console.log(`  stability_alloc:     ${kpi.stability_allocation_uregen}`);
    console.log(`  activity_pool:       ${kpi.activity_pool_uregen}`);
    console.log(`  stability_util:      ${kpi.stability_utilization}`);
    console.log(`  participant_count:   ${kpi.participant_count}`);
    console.log(`  gini_coefficient:    ${kpi.gini_coefficient}`);
    console.log(`  top_earner_share:    ${kpi.top_earner_share}`);
    console.log(`  revenue_ok:          ${kpi.revenue_constraint_satisfied}`);
    console.log(`  stability_cap_ok:    ${kpi.stability_cap_satisfied}`);

    // Validate KPI outputs against expected values
    let vectorFailed = false;

    if (kpi.total_distributed_uregen !== expected.total_distributed_uregen) {
      console.error(`  FAIL: total_distributed expected ${expected.total_distributed_uregen}, got ${kpi.total_distributed_uregen}`);
      vectorFailed = true;
    }

    if (kpi.stability_allocation_uregen !== expected.stability_allocation_uregen) {
      console.error(`  FAIL: stability_allocation expected ${expected.stability_allocation_uregen}, got ${kpi.stability_allocation_uregen}`);
      vectorFailed = true;
    }

    if (kpi.activity_pool_uregen !== expected.activity_pool_uregen) {
      console.error(`  FAIL: activity_pool expected ${expected.activity_pool_uregen}, got ${kpi.activity_pool_uregen}`);
      vectorFailed = true;
    }

    if (kpi.stability_utilization !== expected.stability_utilization) {
      console.error(`  FAIL: stability_utilization expected ${expected.stability_utilization}, got ${kpi.stability_utilization}`);
      vectorFailed = true;
    }

    if (kpi.participant_count !== expected.participant_count) {
      console.error(`  FAIL: participant_count expected ${expected.participant_count}, got ${kpi.participant_count}`);
      vectorFailed = true;
    }

    if (Math.abs(kpi.gini_coefficient - expected.gini_coefficient) > 0.000001) {
      console.error(`  FAIL: gini_coefficient expected ${expected.gini_coefficient}, got ${kpi.gini_coefficient}`);
      vectorFailed = true;
    }

    if (Math.abs(kpi.top_earner_share - expected.top_earner_share) > 0.000001) {
      console.error(`  FAIL: top_earner_share expected ${expected.top_earner_share}, got ${kpi.top_earner_share}`);
      vectorFailed = true;
    }

    // Security invariants
    if (!kpi.revenue_constraint_satisfied) {
      console.error(`  FAIL: revenue constraint violated — total ${kpi.total_distributed_uregen} > inflow ${input.community_pool_inflow_uregen}`);
      vectorFailed = true;
    }

    if (!kpi.stability_cap_satisfied) {
      console.error(`  FAIL: stability cap violated`);
      vectorFailed = true;
    }

    if (vectorFailed) {
      failed++;
    } else {
      passed++;
      console.log(`  PASS`);
    }
    console.log();
  }

  console.log(`m015_kpi self-test: ${passed} passed, ${failed} failed`);
  if (failed > 0) process.exit(1);
}

// Run self-test if executed directly
if (process.argv[1] === __filename) {
  selfTest();
}
