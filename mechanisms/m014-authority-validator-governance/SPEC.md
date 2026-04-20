# m014 — Authority Validator Governance (SPEC)

## 0. Header
- **ID:** m014
- **Name:** Authority Validator Governance
- **Status:** draft (v0)
- **Owner:** (unset)
- **Last updated:** 2026-02-18
- **Scope:** **v0** (mechanism specification for PoS-to-PoA transition with curated, compensated validator set)

## 1. Problem
Regen Network's current Proof of Stake model creates three structural mismatches:
1. **Security vulnerability**: Cost to disrupt drops when token price drops.
2. **Misaligned incentives**: Passive holders receive equivalent rewards as active contributors.
3. **Value disconnection**: Security is funded by inflation that dilutes all holders.

The validator set is currently unstable — sometimes dropping below 21 active validators — and all validators are operating at a loss. They participate for mission alignment, not profit. PoA replaces this with a curated, compensated validator set whose authority derives from demonstrated contribution to the network's ecological mission.

## 2. Target actor and action
- **Actors:** Authority Validators (approved network operators), Validator Governance (body managing validator set at Layer 3), Token Holders ($REGEN holders voting on constitutional changes at Layer 4), Agent (AGENT-004 Validator Monitor).
- **Action being evaluated:** validator performance across uptime, governance participation, and ecosystem contribution, determining compensation and continued membership in the authority set.
- **Event source:** validator lifecycle transitions (application, approval, activation, probation, removal, term expiration) and periodic performance evaluations by AGENT-004.

## 3. Signal definition
- **Signal name:** Validator Performance Score
- **Unit:** score (0.0 – 1.0)
- **Directionality:** higher = better
- **Granularity:** per validator address
- **Factors:**
  - `uptime`: weight 0.4 — blocks signed / blocks expected
  - `governance_participation`: weight 0.3 — votes cast / proposals available
  - `ecosystem_contribution`: weight 0.3 — measured by AGENT-004 (code contributions, dMRV tool development, credit class participation, etc.)

## 4. Evidence inputs

| Input | Source | Fields | Validity rules | Anti-spoof assumptions | Refresh cadence |
|---|---|---|---|---|---|
| Block signing data | Regen Ledger validator set | `blocks_signed`, `blocks_expected` | blocks_expected > 0 | Chain consensus is authoritative | Per epoch / daily |
| Governance votes | Regen governance module | `votes_cast`, `proposals_available` | proposals_available >= votes_cast | On-chain vote records are tamper-proof | Per proposal / weekly |
| Ecosystem contribution | AGENT-004 evaluation | `contribution_score` (0.0–1.0) | Must be assessed within current term | AGENT-004 operates with oversight (Layer 2) | Monthly / quarterly |
| Validator application | Validator governance process | `address`, `category`, `application_data` | Category must be valid; application must be complete | Governance process verifies identity | On application |
| Term metadata | Authority module | `term_start`, `term_end`, `status` | term_end > term_start; status in valid set | Module state is authoritative | On state change |

## 5. Scoring function

### 5.1 Composite performance score
A weighted sum of three performance factors:

```
performance_score = (uptime * 0.4) + (governance_participation * 0.3) + (ecosystem_contribution * 0.3)
```

Where:
- `uptime` = blocks_signed / blocks_expected (0.0–1.0)
- `governance_participation` = votes_cast / proposals_available (0.0–1.0; defaults to 1.0 if no proposals)
- `ecosystem_contribution` = AGENT-004 assessed score (0.0–1.0)

### 5.2 Confidence
Confidence is derived from data availability:
- All three factors available with sufficient data: confidence = 1.0
- One factor missing or estimated: confidence = 0.67
- Two factors missing: confidence = 0.33
- No data: confidence = 0.0

### 5.3 Performance threshold
- `POA_ELIGIBILITY_SCORE = 0.80` — the minimum composite `performance_score` a CANDIDATE must clear to join the Authority set. Stored as an on-chain governance parameter (see §10), not hardcoded in client tooling; integrators should read the current value rather than the 0.80 default.
- Active validators with `performance_score < 0.70` are flagged for review (below the review threshold but above the eligibility floor).
- Validators with `uptime < 0.995` (99.5%) are flagged for probation consideration.

### 5.4 Compensation allocation
```
base_compensation_per_validator = (validator_fund_balance * 0.90) / active_validator_count
performance_bonus_per_validator = (validator_fund_balance * 0.10) * (validator_score / total_scores)
total_compensation = base_compensation + performance_bonus
```

Where:
- `validator_fund_balance` is sourced from M013 fee routing.
- `0.90` = base pool (90% of fund).
- `0.10` = performance bonus pool (10% of fund).
- `total_scores` = sum of all active validators' composite performance scores.

## 6. Validator lifecycle

### 6.1 State machine
```
States: {CANDIDATE, APPROVED, ACTIVE, PROBATION, REMOVED, TERM_EXPIRED}

CANDIDATE -> APPROVED
  trigger: validator_governance.approve(candidate_application)
  guard: meets_composition_criteria, slot_available, no_active_conflicts
  action: add to approved set, schedule activation

APPROVED -> ACTIVE
  trigger: node_operational AND bonded_minimum_stake
  guard: infrastructure_verified (uptime test, key management audit)
  action: add to active validator set, begin block production

ACTIVE -> PROBATION
  trigger: performance_below_threshold OR governance_concern_raised
  guard: AGENT-004 performance report OR governance motion
  action: issue warning, set probation_period(30 days), reduce compensation

PROBATION -> ACTIVE
  trigger: performance_restored AND probation_period_elapsed
  guard: AGENT-004 confirms restoration
  action: restore full compensation

PROBATION -> REMOVED
  trigger: probation_period_elapsed AND performance_not_restored
  guard: validator_governance.confirm_removal()
  action: remove from active set, unbond stake, archive performance record

ACTIVE -> TERM_EXPIRED
  trigger: term_end_date_reached
  guard: none
  action: initiate re-application or graceful exit

TERM_EXPIRED -> CANDIDATE
  trigger: validator.reapply()
  action: enter re-evaluation process (streamlined for incumbents with good records)

Terminal states: REMOVED (no further transitions without new application)
```

### 6.2 Validator composition
```yaml
authority_set:
  target_size: 15-21 validators
  composition:
    infrastructure_builders:
      minimum: 5
      criteria:
        - active development of verification systems, dMRV tools, or registry infrastructure
        - demonstrable code contributions to regen-ledger or ecosystem repos
        - operational history >= 6 months
      examples: "RND engineering team, KOI developers, dMRV tool builders"

    trusted_refi_partners:
      minimum: 5
      criteria:
        - established ReFi organization with public mission alignment
        - active participation in Regen ecosystem (credit origination, marketplace activity, or governance)
        - operational infrastructure meeting minimum uptime requirements (99.5%)
      examples: "ReFiDAO, Toucan, Kolektivo, regional partners"

    ecological_data_stewards:
      minimum: 5
      criteria:
        - organizations attesting to ecological data quality
        - active participation in credit class development or verification
        - domain expertise in ecology, land management, or environmental science
      examples: "Verification bodies, research institutions, land steward cooperatives"
```

## 7. Economic linkage
- **Source:** Validator fund from M013 (value-based fee routing). No inflationary fallback.
- **Model:** Fixed base + optional performance bonus.
  - Base: equal_share — `validator_fund_balance / active_validator_count / period`. All active validators receive equal base compensation.
  - Bonus: 10% of total validator fund, distributed proportional to composite performance score.
- **Term structure:** 12-month terms with quarterly compensation review. Early exit allowed with 30-day notice; forfeits current quarter bonus.
- **Compensation cap:** Total validator compensation must not exceed `validator_fund_balance`. No inflationary fallback.
- **Dependencies:** M013 (validator fund provides compensation), M012 (inflation disabled).

## 8. On-chain vs off-chain boundary
- **On-chain:** Authority module (`x/authority`) stores `AuthorityValidator` (address, category, term_start, term_end, performance), `ValidatorApplication`, `PerformanceRecord`. Events: `EventValidatorApproved`, `EventValidatorRemoved`, `EventTermExpired`, `EventCompensationDistributed`.
- **Off-chain:** AGENT-004 computes ecosystem contribution scores, generates performance reports, and recommends governance actions. Performance scoring and KPI computation are off-chain advisory in v0.
- **Migration:** Gradual transition — PoS and PoA coexist during migration window:
  - Phase 1: Reduce active set to qualified validators via governance.
  - Phase 2: Enable authority module with curated set.
  - Phase 3: Disable PoS inflation module.

## 9. Attack model
- **Capture:** A single category dominating the validator set is prevented by the composition guarantee (minimum 5 per category).
- **Collusion:** Byzantine tolerance requires active_set > 3f + 1 where f = maximum tolerated Byzantine validators (standard Tendermint). With 15 validators, tolerates up to 4 Byzantine nodes.
- **Self-approval:** Existing validators cannot unilaterally approve new validators; requires governance process (Layer 3).
- **Entrenchment:** 12-month terms with mandatory re-application prevent indefinite incumbency.
- **Compensation gaming:** Performance metrics are multi-factor (uptime + governance + ecosystem), making single-dimension gaming ineffective. AGENT-004 provides independent evaluation.
- **Degradation:** If active set drops below `min_validators` (15), trigger emergency governance escalation (P0).

## 10. Governance parameters

| Parameter | Initial Value | Governance Authority | Rationale |
|-----------|--------------|---------------------|-----------|
| `max_validators` | 21 | Layer 4 (Constitutional) | Fundamental network structure |
| `min_validators` | 15 | Layer 4 | Security minimum |
| `term_length` | 12 months | Layer 3 (Human-in-Loop) | Significant governance decision |
| `min_uptime` | 99.5% | Layer 2 (Agentic + Oversight) | Operational parameter |
| `POA_ELIGIBILITY_SCORE` | 0.80 | Layer 2 | Minimum composite score for Authority-set eligibility |
| `probation_period` | 30 days | Layer 2 | Operational parameter |
| `composition_ratios` | 5/5/5 minimum per category | Layer 3 | Structural governance decision |
| `performance_bonus_share` | 10% of validator fund | Layer 2 | Operational adjustment |

## 11. Security invariants
1. **Composition Guarantee**: Active set must maintain minimum representation from each category (>= 5 per category).
2. **Byzantine Tolerance**: Active set > 3f + 1 where f = maximum tolerated Byzantine validators (standard Tendermint).
3. **No Self-Approval**: Existing validators cannot unilaterally approve new validators; requires governance process.
4. **Term Accountability**: No validator serves beyond term without re-approval.
5. **Compensation Cap**: Total validator compensation <= validator_fund balance; no inflationary fallback.
6. **Graceful Degradation**: If active set drops below `min_validators`, trigger emergency governance escalation (P0).

## 12. Open questions (for WG resolution)

> **OQ-M014-1**: Exact validator set size. The WG discusses 15-21. What is the right target, and should it be fixed or allowed to float within the range based on qualified applicants?

> **OQ-M014-2**: Should a performance bonus exist, or should all validators receive equal compensation? Equal compensation is simpler and reduces gaming; performance bonuses incentivize operational excellence.

> **OQ-M014-3**: How is "trusted partner" status determined during the initial transition? Who constitutes the seed set of authority validators? Is this bootstrapped by existing active validators who meet criteria, or selected by governance vote?

> **OQ-M014-4**: PoA socialization timeline. Gregory noted PoA was first socialized ~18 months ago (mid-2024). What is the target activation date? The Economic Reboot Roadmap suggests Q3-Q4 2026 for pilot, 2027 for full migration.

> **OQ-M014-5**: What happens to delegated REGEN when PoS is disabled? Staked tokens must be unbonded gracefully. The transition plan should include a mandatory unbonding period with clear communication.

---

## Appendix A — Source anchors
- `phase-2/2.6-economic-reboot-mechanisms.md`
  - "PROTOCOL SPECIFICATION: M014" (lines 370-532)
  - "Authority Validator Governance" (purpose, design philosophy, validator composition, lifecycle, compensation, governance parameters, security invariants)
- `phase-1/1.4-governance-architecture.md`
  - Governance layers (Layer 1-4 delegation model)
- `phase-2/2.4-agent-orchestration.md`
  - "AGENT-004: Validator Monitor" (performance tracking, recommendation engine)
