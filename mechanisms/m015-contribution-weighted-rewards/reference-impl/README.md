# m015 reference implementation (v0)

This folder provides a **canonical computation** for m015 outputs so that different agents/runners
produce consistent numbers.

## Modules

### Activity scoring and distribution (`m015_score.js`)
- `computeActivityScore({ activities })` -- weighted score from five activity types
- `computeStabilityAllocation({ community_pool_inflow, stability_commitments, ... })` -- stability tier payout with 30% cap
- `computeDistribution({ activity_pool_amount, participants })` -- pro-rata reward distribution with remainder handling

### KPI computation (`m015_kpi.js`)
- `computeM015KPI({ community_pool_inflow_uregen, stability_commitments, participants, ... })` -- aggregated KPI block
- `giniCoefficient(values)` -- Gini inequality measure for reward distribution

## Inputs

### Activity scoring
- `activities.credit_purchase_value` (number, uregen) -- weight 0.30
- `activities.credit_retirement_value` (number, uregen) -- weight 0.30
- `activities.platform_facilitation_value` (number, uregen) -- weight 0.20
- `activities.governance_votes_cast` (number) -- weight 0.10
- `activities.proposals[]` (array of `{ passed, reached_quorum }`) -- weight 0.10, anti-gaming rules apply

### Stability allocation
- `community_pool_inflow` (number, uregen) -- Community Pool inflow for the period
- `stability_commitments[]` (array of `{ committed_amount_uregen }`) -- active commitments
- `periods_per_year` (number, default 52) -- weekly epochs
- `max_stability_share` (number, default 0.30) -- 30% cap on stability tier

## Outputs

### Activity score
- `total_score` -- weighted sum of all activity contributions
- `breakdown` -- per-activity detail (raw_value, weight, weighted_value)

### Stability allocation
- `stability_allocation` -- uregen allocated to stability tier (capped)
- `activity_pool` -- uregen remaining for activity distribution

### Distribution
- Per participant: `address`, `reward` (uregen), `share` (0-1)
- Remainder from `Math.floor()` truncation assigned to largest-share participant

### KPI block
- `total_distributed_uregen` -- stability + activity distributions
- `stability_allocation_uregen`, `activity_pool_uregen`
- `stability_utilization` -- fraction of 30% cap used
- `participant_count` -- participants with score > 0
- `gini_coefficient` -- inequality measure (0 = equal, 1 = max inequality)
- `top_earner_share` -- share of rewards going to highest scorer
- `revenue_constraint_satisfied` -- boolean: total payout <= inflow
- `stability_cap_satisfied` -- boolean: stability <= 30% cap

## Self-test

```bash
node m015_score.js
node m015_kpi.js
```

Each script reads all test vectors from `test_vectors/` and validates computed outputs against expected values.

## Test vectors

| Vector | Scenario |
|--------|----------|
| `vector_v0_sample` | 4 participants with diverse activity profiles, 1 stability commitment |
| `vector_v0_early_exit` | Stability tier with early exit penalty (50% forfeiture), 3 participants |
| `vector_v0_cap_overflow` | Stability obligations exceed 30% cap, demonstrating cap enforcement |
| `vector_v0_zero_activity` | All participants have zero activity, no stability commitments |

Each vector has a `.input.json` and `.expected.json` pair.

## Design notes
- All monetary values are integers in **uregen** (1 REGEN = 1,000,000 uregen).
- `Math.floor()` truncation is intentional for all reward computations, matching on-chain integer arithmetic.
- The remainder from floor truncation is assigned to the largest-share participant to ensure `sum(rewards) == activity_pool`.
- Stability allocation uses `Math.min(Math.floor(rawAllocation), Math.floor(cap))` to prevent over-allocation.
