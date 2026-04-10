# m015 datasets (replay fixtures)

These fixtures are **deterministic inputs** for generating non-zero m015 KPI outputs **without MCP**.

## Files
- `schema.json` -- JSON schema for replay datasets
- `fixtures/v0_sample.json` -- single distribution period with 4 participants and 1 stability commitment
- `fixtures/v0_stability_sample.json` -- stability tier scenarios (committed, matured, early exit, cap overflow)

## How they are used
A replay runner (e.g., in `regen-heartbeat`) can read a fixture file and compute:
- Per-participant activity scores using `computeActivityScore` from `reference-impl/m015_score.js`
- Stability tier allocation using `computeStabilityAllocation` from `reference-impl/m015_score.js`
- Pro-rata distribution using `computeDistribution` from `reference-impl/m015_score.js`
- Aggregated KPIs using `computeM015KPI` from `reference-impl/m015_kpi.js`

Key metrics produced:
- `total_distributed_uregen` -- total rewards distributed (stability + activity)
- `activity_pool_uregen` -- pool available for activity-based distribution
- `stability_allocation_uregen` -- stability tier payout (capped at 30% of inflow)
- `participant_count` -- participants with non-zero activity scores
- `gini_coefficient` -- inequality measure of reward distribution

## Units
All token amounts are in **uregen** (1 REGEN = 1,000,000 uregen) and represented as integers.

These datasets are **reference-only** and do not imply enforcement or on-chain actions.
