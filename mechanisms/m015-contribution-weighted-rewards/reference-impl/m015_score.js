/**
 * m015 — Contribution-Weighted Rewards: Activity scoring and distribution.
 *
 * Activity weights:
 *   credit_purchase:        0.30
 *   credit_retirement:      0.30
 *   platform_facilitation:  0.20
 *   governance_voting:      0.10
 *   proposal_submission:    0.10
 *
 * Proposal anti-gaming:
 *   passed + quorum  -> 1.0 credit
 *   failed + quorum  -> 0.5 credit
 *   no quorum        -> 0.0 credit
 *
 * Stability tier: 6% annual, min 6 month lock, max 24 month, 50% early exit penalty,
 *   capped at 30% of community pool inflow per period.
 *
 * All monetary values are in uregen (1 REGEN = 1,000,000 uregen).
 * periods_per_year = 52 (weekly epochs).
 *
 * @module m015_score
 */

const WEIGHTS = {
  credit_purchase: 0.30,
  credit_retirement: 0.30,
  platform_facilitation: 0.20,
  governance_voting: 0.10,
  proposal_submission: 0.10,
};

const DEFAULT_ANNUAL_RETURN = 0.06;
const DEFAULT_PERIODS_PER_YEAR = 52;
const DEFAULT_MAX_STABILITY_SHARE = 0.30;

/**
 * Compute the effective proposal credit for a list of proposals.
 * @param {Array<{passed: boolean, reached_quorum: boolean}>} proposals
 * @returns {number} effective proposal credit
 */
function computeProposalCredit(proposals) {
  if (!proposals || !proposals.length) return 0;
  let credit = 0;
  for (const p of proposals) {
    if (p.reached_quorum && p.passed) {
      credit += 1.0;
    } else if (p.reached_quorum && !p.passed) {
      credit += 0.5;
    }
    // no quorum -> 0
  }
  return credit;
}

/**
 * Compute the activity score for a single participant.
 *
 * @param {Object} opts
 * @param {Object} opts.activities
 * @param {number} opts.activities.credit_purchase_value - total purchase value in uregen
 * @param {number} opts.activities.credit_retirement_value - total retirement value in uregen
 * @param {number} opts.activities.platform_facilitation_value - total facilitation value in uregen
 * @param {number} opts.activities.governance_votes_cast - number of votes cast
 * @param {Array<{passed: boolean, reached_quorum: boolean}>} opts.activities.proposals - proposals submitted
 * @returns {{ total_score: number, breakdown: Object }}
 */
export function computeActivityScore({ activities }) {
  const a = activities ?? {};
  const purchaseVal = a.credit_purchase_value ?? 0;
  const retireVal = a.credit_retirement_value ?? 0;
  const facilitateVal = a.platform_facilitation_value ?? 0;
  const votes = a.governance_votes_cast ?? 0;
  const proposalCredit = computeProposalCredit(a.proposals);

  const breakdown = {
    credit_purchase: {
      raw_value: purchaseVal,
      weight: WEIGHTS.credit_purchase,
      weighted_value: purchaseVal * WEIGHTS.credit_purchase,
    },
    credit_retirement: {
      raw_value: retireVal,
      weight: WEIGHTS.credit_retirement,
      weighted_value: retireVal * WEIGHTS.credit_retirement,
    },
    platform_facilitation: {
      raw_value: facilitateVal,
      weight: WEIGHTS.platform_facilitation,
      weighted_value: facilitateVal * WEIGHTS.platform_facilitation,
    },
    governance_voting: {
      raw_value: votes,
      weight: WEIGHTS.governance_voting,
      weighted_value: votes * WEIGHTS.governance_voting,
    },
    proposal_submission: {
      raw_value: proposalCredit,
      weight: WEIGHTS.proposal_submission,
      weighted_value: proposalCredit * WEIGHTS.proposal_submission,
    },
  };

  const total_score = Object.values(breakdown).reduce((s, b) => s + b.weighted_value, 0);

  return { total_score, breakdown };
}

/**
 * Compute the stability tier allocation for a period.
 *
 * @param {Object} opts
 * @param {number} opts.community_pool_inflow - inflow in uregen for this period
 * @param {Array<{committed_amount_uregen: number}>} opts.stability_commitments - active commitments
 * @param {number} [opts.periods_per_year=52]
 * @param {number} [opts.max_stability_share=0.30]
 * @param {number} [opts.annual_return=0.06]
 * @returns {{ stability_allocation: number, activity_pool: number }}
 */
export function computeStabilityAllocation({
  community_pool_inflow,
  stability_commitments,
  periods_per_year = DEFAULT_PERIODS_PER_YEAR,
  max_stability_share = DEFAULT_MAX_STABILITY_SHARE,
  annual_return = DEFAULT_ANNUAL_RETURN,
}) {
  const commitments = stability_commitments ?? [];
  const totalCommitted = commitments.reduce((s, c) => s + (c.committed_amount_uregen ?? 0), 0);
  const rawAllocation = totalCommitted * annual_return / periods_per_year;
  const cap = community_pool_inflow * max_stability_share;
  const stability_allocation = Math.min(Math.floor(rawAllocation), Math.floor(cap));
  const activity_pool = community_pool_inflow - stability_allocation;

  return { stability_allocation, activity_pool };
}

/**
 * Compute distribution across all participants for an epoch.
 *
 * @param {Object} opts
 * @param {number} opts.activity_pool_amount - total uregen available for activity distribution
 * @param {Array<{address: string, total_score: number}>} opts.participants - scored participants
 * @returns {Array<{address: string, reward: number, share: number}>}
 */
export function computeDistribution({ activity_pool_amount, participants }) {
  const ps = participants ?? [];
  const totalScore = ps.reduce((s, p) => s + (p.total_score ?? 0), 0);
  if (totalScore === 0) return ps.map(p => ({ address: p.address, reward: 0, share: 0 }));

  const results = ps.map(p => {
    const share = (p.total_score ?? 0) / totalScore;
    const reward = Math.floor(activity_pool_amount * share);
    return { address: p.address, reward, share: Number(share.toFixed(6)) };
  });

  // Distribute remainder to largest share holder to avoid dust
  const allocated = results.reduce((s, r) => s + r.reward, 0);
  const remainder = activity_pool_amount - allocated;
  if (remainder > 0 && results.length > 0) {
    const maxIdx = results.reduce((best, r, i) => r.share > results[best].share ? i : best, 0);
    results[maxIdx].reward += remainder;
  }

  return results;
}

// ---------------------------------------------------------------------------
// Self-test harness (runs when executed directly)
// ---------------------------------------------------------------------------
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function runVector(inputPath, expectedPath) {
  const input = JSON.parse(readFileSync(inputPath, "utf8"));
  const expected = JSON.parse(readFileSync(expectedPath, "utf8"));

  // 1. Compute stability allocation
  const { stability_allocation, activity_pool } = computeStabilityAllocation({
    community_pool_inflow: input.community_pool_inflow_uregen,
    stability_commitments: input.stability_commitments,
    periods_per_year: input.periods_per_year,
    max_stability_share: input.max_stability_share,
  });

  console.log(`  Stability allocation: ${stability_allocation} uregen`);
  console.log(`  Activity pool:        ${activity_pool} uregen`);

  // Check stability allocation
  if (stability_allocation !== expected.stability_allocation_uregen) {
    throw new Error(`stability_allocation expected ${expected.stability_allocation_uregen}, got ${stability_allocation}`);
  }
  if (activity_pool !== expected.activity_pool_uregen) {
    throw new Error(`activity_pool expected ${expected.activity_pool_uregen}, got ${activity_pool}`);
  }

  // 2. Compute activity scores for each participant
  const scoredParticipants = [];
  for (const p of input.participants) {
    const result = computeActivityScore({ activities: p.activities });
    scoredParticipants.push({ address: p.address, ...result });
    console.log(`  Score ${p.address}: ${result.total_score}`);
  }

  // Check individual scores
  for (const exp of expected.participant_scores) {
    const actual = scoredParticipants.find(p => p.address === exp.address);
    if (!actual) {
      throw new Error(`participant ${exp.address} not found`);
    }
    if (Math.abs(actual.total_score - exp.total_score) > 0.001) {
      throw new Error(`${exp.address} score expected ${exp.total_score}, got ${actual.total_score}`);
    }
  }

  // 3. Compute distribution
  const dist = computeDistribution({
    activity_pool_amount: activity_pool,
    participants: scoredParticipants,
  });

  console.log("  Distribution:");
  for (const d of dist) {
    console.log(`    ${d.address}: ${d.reward} uregen (${(d.share * 100).toFixed(2)}%)`);
  }

  // Check distribution totals (skip for zero-activity: all rewards 0, pool is unallocated)
  const totalDistributed = dist.reduce((s, d) => s + d.reward, 0);
  const allZero = scoredParticipants.every(p => p.total_score === 0);
  if (!allZero && totalDistributed !== activity_pool) {
    throw new Error(`total distributed ${totalDistributed} != activity_pool ${activity_pool}`);
  }

  // Check individual rewards
  for (const exp of expected.distribution) {
    const actual = dist.find(d => d.address === exp.address);
    if (!actual) {
      throw new Error(`distribution for ${exp.address} not found`);
    }
    if (Math.abs(actual.reward - exp.reward) > 1) {
      throw new Error(`${exp.address} reward expected ${exp.reward}, got ${actual.reward}`);
    }
  }

  // Check security invariants
  const totalPayout = stability_allocation + totalDistributed;
  if (totalPayout > input.community_pool_inflow_uregen) {
    throw new Error(`total payout ${totalPayout} > inflow ${input.community_pool_inflow_uregen}`);
  }

  const stabilityCap = input.community_pool_inflow_uregen * input.max_stability_share;
  if (stability_allocation > stabilityCap) {
    throw new Error(`stability_allocation ${stability_allocation} > cap ${stabilityCap}`);
  }
}

function selfTest() {
  const vectorDir = join(__dirname, "test_vectors");
  const inputFiles = readdirSync(vectorDir)
    .filter(f => f.endsWith(".input.json"))
    .sort();

  let passed = 0;
  let failed = 0;

  for (const inputFile of inputFiles) {
    const baseName = inputFile.replace(".input.json", "");
    const expectedFile = baseName + ".expected.json";
    const inputPath = join(vectorDir, inputFile);
    const expectedPath = join(vectorDir, expectedFile);

    console.log(`--- ${baseName} ---`);
    try {
      runVector(inputPath, expectedPath);
      passed++;
      console.log("  PASS\n");
    } catch (err) {
      failed++;
      console.error(`  FAIL: ${err.message}\n`);
    }
  }

  console.log(`m015_score self-test: ${passed} passed, ${failed} failed`);
  if (failed > 0) process.exit(1);
}

// Run self-test if executed directly
if (process.argv[1] === __filename) {
  selfTest();
}
