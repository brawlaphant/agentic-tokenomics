# PoA Migration Open Questions Triage

**Status:** Draft for WG Review
**Date:** 2026-03-25
**Author:** Tokenomics Working Group
**Scope:** Prioritized triage of all open questions across the PoA migration and Economic Reboot
**Companion doc:** [Open Questions Resolution](./open-questions-resolution.md)

---

## Executive Summary

This document triages all 33 open questions from the Phase 2 mechanism specifications and OQ resolution doc (31 from module specs plus 2 additional governance transition questions), plus 4 implicit questions from the bioregional validator framework, and identifies 8 additional implicit questions that the existing OQ process did not surface -- 45 items total. Each question is assigned a priority, resolution owner, recommended answer, and dependency mapping.

### Counts by Priority

| Priority | Count | Description |
|----------|-------|-------------|
| P0 (blocks implementation) | 10 | Must be resolved before any code is written |
| P1 (blocks mainnet) | 18 | Must be resolved before mainnet launch |
| P2 (deferrable) | 17 | Can be deferred to v1 or later iteration |

### Counts by Resolution Owner

| Owner | Count |
|-------|-------|
| Engineering | 16 |
| Community Governance | 8 |
| Tokenomics WG | 14 |
| Cross-functional | 7 |

### Counts by Status (all 45 items in this triage)

| Status | Count |
|--------|-------|
| RESOLVED (technical recommendation ready) | 26 |
| NEEDS_GOVERNANCE (requires community vote) | 9 |
| NOT_ADDRESSED (gaps identified in this triage) | 10 |

> **Note:** The companion OQ Resolution Doc covers the original 31 questions (22 RESOLVED, 9 NEEDS_GOVERNANCE). This triage adds 4 bioregional and 8 implicit questions. Two items initially marked NOT_ADDRESSED (OQ-BIO-2, OQ-IMPL-4) are reclassified here as NEEDS_GOVERNANCE because they require governance decisions and are included in the governance proposal packaging below.

### Critical Path

All 10 P0 items must be resolved before implementation can proceed. The primary dependency chain:

```
Phase A: Foundation Decisions (parallel, no upstream deps)
  OQ-M012-4 (permanent burn vs reserve)
  OQ-M013-2 (credit value determination)
  OQ-IMPL-2 (SDK v0.54 readiness assessment)

Phase B: Economic Parameters (sequential, governance-gated)
  OQ-M013-1 (fee distribution model)
      -> OQ-M013-5 (burn pool existence/size)
          -> OQ-M012-1 (hard cap value)

Phase C: Operational Setup (after Phase B)
  OQ-M014-3 (initial validator seed set)
  OQ-IMPL-1 (zero fee revenue contingency)
  OQ-IMPL-3 (IBC safety during migration)
  OQ-IMPL-8 (cross-mechanism state consistency)
      -> Implementation can begin
```

See the [full Critical Path Analysis](#critical-path-analysis) below for resolution order, timelines, and dependencies.

---

## M012: Fixed Cap Dynamic Supply

### OQ-M012-1: Hard Cap Value

**Question (from spec):** The exact hard cap value. Token-economics-synthesis says "~221M" based on current total supply (~224M). Should the cap be set at current total supply, slightly below (to create immediate scarcity), or at a round number?

**Priority:** P0 -- blocks implementation. The hard cap is a constitutional parameter (Layer 4, 67% supermajority). Every supply simulation, equilibrium analysis, and regrowth rate calibration depends on this value. The M012 module cannot be finalized without it.

**Recommended Resolution Owner:** Community Governance

**Concrete Recommended Answer:** Set the hard cap at 221,000,000 REGEN (221,000,000,000,000 uregen). This is approximately 1.3% below current circulating supply (~224M), creating immediate but manageable scarcity pressure. The 3M token gap is achievable through organic fee-driven burns within 12-18 months at moderate fee volumes.

**Rationale:** A below-current-supply cap is the only credible deflationary signal. Setting the cap at or above current supply eliminates any urgency for burn mechanics to operate. The 221M figure balances credibility (meaningful scarcity) with achievability (not so far below current supply that it requires years of burning). The governance proposals document (Proposal 3) already assumes 221M.

**Status:** NEEDS_GOVERNANCE -- The OQ resolution doc provides a recommendation but flags this for governance vote given its direct economic impact on all token holders.

**Implementation Dependencies:** OQ-M013-1 and OQ-M013-5 must be resolved first (the burn share determines how quickly supply approaches cap from above).

**Blocks:**
- M012 `x/supply` module parameter configuration
- All supply simulation models (PR #54)
- Governance Proposal 3 (M012 activation)
- Equilibrium timeline projections

---

### OQ-M012-2: Ecological Multiplier Oracle

**Question (from spec):** The ecological multiplier oracle. What data source provides delta_co2 or equivalent ecological metric? Is this sourced from on-chain attestation data (M008) or from an external oracle? The v0 spec disables this until resolved.

**Priority:** P2 -- deferrable. The v0 spec explicitly disables the ecological multiplier (set to 1.0). This question does not block any v0 implementation.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Disable in v0 (ecological_multiplier = 1.0). In v1, implement using M008 attestation aggregate data with a 30-day rolling window, weighted by credit class diversity and geographic distribution. The M008 attestation system needs months of operational data before it can serve as a reliable oracle.

**Rationale:** Launching with a premature oracle creates a manipulation surface. By v1, M008 attestation patterns will be well-understood, providing a credible on-chain data source.

**Status:** RESOLVED -- The OQ resolution doc provides a clear technical recommendation.

**Implementation Dependencies:** M008 must be deployed and operational (Stage 1 of migration) before v1 oracle design can be informed by real data.

**Blocks:**
- Nothing in v0
- v1 ecological multiplier feature

---

### OQ-M012-3: Burn Period Length

**Question (from spec):** Period length for mint/burn cycles. Is per-epoch (weekly) the right cadence, or should it be per-block (like EIP-1559) for finer granularity?

**Priority:** P1 -- blocks mainnet. The period length determines the `x/supply` module's execution cadence and must be set before mainnet deployment, but does not block initial implementation work (can code the module with the period as a configurable parameter).

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** 7-day (weekly) burn epochs. Fees accumulate in the burn pool during the epoch and are burned in a single transaction at epoch end. This aligns with M015's weekly distribution period (52 periods/year) and provides a natural governance circuit breaker for anomalous periods.

**Rationale:** Weekly epochs balance burn frequency with operational clarity. Per-block burns add computational overhead with marginal benefit. The accumulated-then-burned model allows governance intervention before anomalous burns execute.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation. Engineering can proceed.

**Implementation Dependencies:** None -- this is an independent parameter choice.

**Blocks:**
- M012 epoch scheduler implementation
- M013 burn pool accumulation logic
- M015 distribution period alignment

---

### OQ-M012-4: Permanent Burn vs Reserve

**Question (from spec):** Should burned tokens be permanently destroyed or sent to a reserve pool that can be re-minted under governance control?

**Priority:** P0 -- blocks implementation. This is an architectural decision: permanent burn means `SendCoinsFromModuleToModule` to a null address vs. a reserve means maintaining a recoverable pool. The module design differs fundamentally.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Permanent burn. Burned tokens are irrecoverably destroyed via `BurnCoins` on the bank module. No reserve pool. Burn rate adjustments are forward-looking parameter changes, not retroactive reserve access.

**Rationale:** Irreversibility is the foundation of deflationary credibility. A reserve mechanism creates a governance attack surface (proposals to "unlock the reserve for X") and undermines market confidence. Every token holder's pricing of scarcity depends on burns being final.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation with strong rationale.

**Implementation Dependencies:** None -- this is a foundational architectural decision.

**Blocks:**
- M012 burn execution implementation (permanent destroy vs. reserve transfer)
- Audit scope for burn mechanism
- Market messaging around deflation

---

### OQ-M012-5: Which Multiplier Phase at Launch

**Question (from spec):** Should the staking_multiplier be replaced by a stability_multiplier (from M015 commitments) or a validator_participation_multiplier (from M014 active set health)?

**Priority:** P1 -- blocks mainnet. The multiplier selection determines the regrowth rate formula's behavior across the PoA transition. Must be finalized before Stage 3 (M014 + M012 combined upgrade).

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Phase-gated progression triggered by on-chain conditions:
- **Phase 1 (launch):** staking_multiplier -- incentivizes staking participation to secure the validator set during the early PoA period
- **Phase 2 (staking ratio > 60%):** maximum deflation multiplier -- highest burn rates once staking security is established
- **Phase 3 (supply reaches governance-defined target):** stability_multiplier -- adaptive burn to maintain supply near target

During the PoA transition specifically, the M012 spec's `max(staking_multiplier, stability_multiplier)` selection prevents a regrowth cliff as staked tokens unbond while M015 stability commitments ramp up.

Transitions are triggered by on-chain conditions (staking ratio thresholds, supply levels), not calendar dates.

**Rationale:** Each phase addresses the network's most pressing need at that stage. The phase progression is consistent with the OQ resolution doc's three-phase model (staking -> maximum deflation -> stability). The M012 spec adds the `max()` bridging logic for the PoA transition period specifically, which is complementary to (not in conflict with) the general progression.

**Status:** RESOLVED -- The OQ resolution doc describes the three-phase progression; the M012 spec adds transition-period bridging logic. Both are consistent.

**Implementation Dependencies:** M014 state machine must be implemented first (provides the phase gate signal). M015 stability tier must be at least specified (provides the stability_committed input).

**Blocks:**
- M012 effective_multiplier computation logic
- Integration testing between M012, M014, and M015

---

## M013: Value-Based Fee Routing

### OQ-M013-1: Model A vs Model B Fee Distribution

**Question (from spec):** Which distribution model should be adopted? Model A provides a dedicated Agent Infrastructure fund; Model B routes a larger share through governance.

**Priority:** P0 -- blocks implementation. This is the single most consequential parameter decision. Every downstream mechanism's economics (M012 burn input, M014 validator compensation, M015 community pool funding, agent infrastructure) depends on the fee split. The `x/feerouter` module can be coded generically, but all testing, simulation, and governance proposals require concrete values.

**Recommended Resolution Owner:** Community Governance

**Concrete Recommended Answer:** Adopt the compromise distribution: 28% burn / 25% validator / 45% community / 2% agent infrastructure. This is the middle ground between Model A (30/40/25/5) and Model B (30/20/50/0). The governance proposals document already uses this compromise.

**Rationale:** Neither Model A nor Model B commands consensus. The compromise preserves all four pools (maintaining the agent infrastructure fund's existence as a signal of commitment to AI governance) while shifting emphasis toward the community pool (45% is the largest single allocation). The distribution is adjustable via governance parameter change after 90 days of observation.

**Status:** NEEDS_GOVERNANCE -- The OQ resolution doc recommends the compromise but flags this for governance vote.

**Implementation Dependencies:** None upstream -- this is the root of the fee distribution dependency chain.

**Blocks:**
- All fee distribution parameter configuration
- M012 burn pool sizing and equilibrium analysis
- M014 validator compensation projections
- M015 community pool inflow modeling
- All revenue projection models
- Governance Proposal 1 (M013 activation)

---

### OQ-M013-2: Credit Value Determination

**Question (from spec):** How is credit value determined for non-marketplace transactions (issuance, transfer, retirement)? Options: (A) most recent marketplace price for that credit class, (B) governance-set reference price per credit type, (C) external oracle via KOI.

**Priority:** P0 -- blocks implementation. Fee calculation accuracy for the majority of transactions depends on this. Without a value determination method, the `x/feerouter` module cannot compute fees for issuance, transfer, or retirement transactions.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Use 7-day Time-Weighted Average Price (TWAP) from the on-chain marketplace for actively traded credit classes. Governance-set reference prices (updated quarterly minimum) for untraded or thinly traded classes. TWAP computed at epoch boundaries aligned with the 7-day burn epoch.

**Rationale:** TWAP provides manipulation-resistant, timely pricing. The 7-day window smooths volatility while remaining responsive. Governance fallback covers new or illiquid credit classes. Epoch alignment with the burn mechanism simplifies implementation.

**Status:** RESOLVED -- The OQ resolution doc provides a clear technical recommendation.

**Implementation Dependencies:** Marketplace module must expose price history for TWAP computation. Governance module must support reference price parameter storage.

**Blocks:**
- `x/feerouter` fee calculation implementation
- TWAP computation module or query
- Governance reference price parameter design

---

### OQ-M013-3: Fee Denomination

**Question (from spec):** In what denomination should fees be collected and distributed? REGEN-only, native denom, or hybrid?

**Priority:** P1 -- blocks mainnet. The denomination strategy has significant implementation and UX implications but does not block initial module development (can start with REGEN-only and add multi-denom later).

**Recommended Resolution Owner:** Community Governance

**Concrete Recommended Answer:** Hybrid approach: collect fees in the transaction denomination (preserving user experience), auto-convert the burn portion to REGEN at epoch boundaries (burn mechanism requires REGEN), distribute the remainder in the collected denomination. Batch conversion at epoch boundaries minimizes slippage.

**Rationale:** REGEN-only creates friction for users transacting in USDC or ATOM. The hybrid approach threads the needle: users pay in what they have, the burn mechanism gets REGEN, and validators/community receive the asset most natural to them.

**Status:** NEEDS_GOVERNANCE -- The OQ resolution doc recommends the hybrid approach but flags for governance vote given UX implications.

**Implementation Dependencies:** On-chain DEX or IBC swap infrastructure must exist for auto-conversion. Batch conversion logic at epoch boundaries.

**Blocks:**
- `x/feerouter` denomination handling
- Auto-conversion integration with DEX/IBC
- Governance Proposal 1 parameter configuration

---

### OQ-M013-4: Agent Infrastructure Fund Governance

**Question (from spec):** How should the Agent Infrastructure fund be governed? As a separate module account with its own spending authority, or as a tagged allocation within the Community Pool subject to governance proposals?

**Priority:** P2 -- deferrable. The agent fund is 2% of fees. Its governance structure can be refined after launch. Initial implementation can use a simple module account with multisig control.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Separate module account for the agent infrastructure fund. Multisig governance (3-of-5) in Stage 1 with expedited disbursement for amounts under 10,000 REGEN. Full governance control in Stage 2+.

**Rationale:** Staged governance matches fund maturity to network maturity. Separate accounting ensures transparency. The multisig provides efficiency during bootstrapping when the fund is small and rapid experimentation is valuable.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation.

**Implementation Dependencies:** M013 fee distribution must route to a separate module account. Multisig tooling (likely DAO DAO or Cosmos SDK group module).

**Blocks:**
- Agent infrastructure module account creation
- Multisig configuration
- Agent operational budget planning

---

### OQ-M013-5: Burn Pool Existence and Size

**Question (from spec):** Should the Burn Pool exist at all, and if so, at what share?

**Priority:** P0 -- blocks implementation. If the burn pool does not exist, M012's entire supply management model changes fundamentally. This question is architecturally load-bearing.

**Recommended Resolution Owner:** Community Governance

**Concrete Recommended Answer:** Yes, maintain a burn pool, reduced to 15% of fee distribution in the final steady state. For v0 launch, use 28% (the compromise value from OQ-M013-1) and reduce via governance proposal after observing 6-12 months of fee data. The burn pool's existence is essential for M012 to function.

**Rationale:** Without a burn pool, M012's deflationary mechanism has no fuel. The 15% long-term target preserves deflationary credibility while shifting emphasis toward contributor compensation. Starting at 28% and reducing is lower-risk than starting low and trying to increase (increasing burns is politically harder than decreasing them).

**Status:** NEEDS_GOVERNANCE -- The OQ resolution doc recommends 15% with governance vote. Note the tension between the 28% compromise in OQ-M013-1 and the 15% long-term recommendation here -- the governance proposal should address this explicitly.

**Implementation Dependencies:** OQ-M013-1 (the burn share is part of the overall distribution model).

**Blocks:**
- M012 supply model viability
- Equilibrium timeline projections
- All deflationary narrative and messaging
- Governance Proposal 1 parameters

---

## M014: Authority Validator Governance

### OQ-M014-1: Validator Set Size

**Question (from spec):** Exact validator set size. The WG discusses 15-21. What is the right target, and should it be fixed or allowed to float within the range based on qualified applicants?

**Priority:** P1 -- blocks mainnet. The set size determines BFT safety margins, compensation per validator, and composition enforcement. Must be finalized before the seed set is selected.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Start at 15 validators. Grow to 21 organically as qualified applicants emerge via streamlined governance proposals. The set size floats within [15, 21] based on applicant quality, not a fixed target.

**Rationale:** 15 provides adequate BFT tolerance (tolerates 4 Byzantine validators) and keeps per-validator compensation meaningful at current fee projections. Organic growth avoids diluting compensation or admitting unqualified validators to hit a number.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation.

**Implementation Dependencies:** None -- this is a governance parameter.

**Blocks:**
- M014 `max_validators` and `min_validators` parameter setting
- Compensation modeling (per-validator amounts depend on set size)
- Seed set selection process

---

### OQ-M014-2: Performance Bonus

**Question (from spec):** Should a performance bonus exist, or should all validators receive equal compensation?

**Priority:** P2 -- deferrable. The performance bonus is a refinement on top of base compensation. v0 can launch with equal distribution and add performance bonuses once AGENT-004 monitoring is calibrated.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Yes, maintain a 10% performance bonus from the validator fund. Base it on objective, on-chain metrics: uptime (weight 0.4), governance participation (weight 0.3), and block production consistency (weight 0.3). Distribute weekly at epoch boundaries.

**Rationale:** Simple, meaningful incentive for operational excellence. Objective metrics prevent gaming and favoritism. Can be implemented after v0 launch once AGENT-004 has baseline performance data.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation.

**Implementation Dependencies:** AGENT-004 must be operational and producing performance scores. M013 must be routing to the validator fund.

**Blocks:**
- Performance bonus calculation logic
- AGENT-004 scoring calibration
- Validator compensation contract design

---

### OQ-M014-3: Initial Trusted Partners / Seed Set

**Question (from spec):** How is "trusted partner" status determined during the initial transition? Who constitutes the seed set of authority validators?

**Priority:** P0 -- blocks implementation. No code can be deployed for Stage 3 (PoA + Dynamic Supply) without a validator seed set. The seed set selection process must be completed before the M014 governance proposal can be submitted.

**Recommended Resolution Owner:** Community Governance

**Concrete Recommended Answer:** Bootstrap from current active validators who meet PoA criteria (assessed via the validator-selection-rubric.md scoring framework). Submit the proposed seed set for governance vote, allowing individual objections during the voting period. Target composition: 5 Infrastructure Builders, 5 Trusted ReFi Partners, 5 Ecological Data Stewards.

**Rationale:** Existing validators have demonstrated commitment through sustained operation. The selection rubric (1000-point scale, 600-point minimum threshold) provides a transparent, repeatable evaluation framework. Governance vote ensures community legitimacy.

**Status:** NEEDS_GOVERNANCE -- The OQ resolution doc recommends bootstrapping from current validators but flags this for governance vote. The validator-selection-rubric.md provides the scoring framework but the actual candidate evaluation has not been performed.

**Implementation Dependencies:** Validator selection rubric must be finalized. Candidate outreach must begin 6+ months before activation. Applications must be collected and scored.

**Blocks:**
- M014 genesis configuration (seed validator set)
- Governance Proposal 2 (M014 activation) -- cannot be submitted without a named seed set
- Stage 3 deployment timeline

---

### OQ-M014-4: PoA Activation Timeline

**Question (from spec):** PoA socialization timeline. What is the target activation date?

**Priority:** P1 -- blocks mainnet. The timeline gates all downstream planning: testnet scheduling, governance proposal submission, delegator communication, and ecosystem partner coordination.

**Recommended Resolution Owner:** Cross-functional

**Concrete Recommended Answer:** Q3 2026 testnet pilot (minimum 8-week duration). Q4 2026 mainnet activation, gated on testnet success criteria (no critical bugs, validator set stability, governance mechanism validation). Timeline aligned with the Economic Reboot Roadmap.

**Rationale:** Provides 3-4 months for specification finalization and implementation from now (March 2026). The 8-week testnet minimum ensures operational stress-testing. Mainnet activation gated on success criteria rather than a fixed date prevents premature deployment.

**Status:** RESOLVED -- The OQ resolution doc provides a clear timeline recommendation.

**Implementation Dependencies:** All P0 questions must be resolved before testnet. M013 must be active on mainnet before M014 proposal submission.

**Blocks:**
- Testnet scheduling and infrastructure provisioning
- Validator application deadline setting
- Delegator communication timeline (T-90, T-60, T-30 schedule)

---

### OQ-M014-5: Impact on Delegated REGEN

**Question (from spec):** What happens to delegated REGEN when PoS is disabled?

**Priority:** P1 -- blocks mainnet. Delegator communication and the unbonding plan must be finalized before the M014 governance proposal is submitted. This is the highest-sensitivity stakeholder impact question.

**Recommended Resolution Owner:** Cross-functional

**Concrete Recommended Answer:** 90-day advance notice before PoA activation. The standard 21-day unbonding period remains unchanged. PoS inflation ramps down gradually during M014 Phase 2 (50% reduction in first quarter, 75% in second). All remaining delegations begin forced unbonding when PoS is disabled in Phase 3. Multi-channel communication: on-chain governance, social media, validator announcements, ecosystem partner notifications.

**Rationale:** Respects delegators' economic interests with ample notice. Gradual inflation reduction prevents economic shock. The 21-day unbonding period is a Cosmos SDK parameter that should not be modified during an already-complex transition.

**Status:** RESOLVED -- The OQ resolution doc and the mainnet migration plan (Phase 4.2) both provide detailed treatment.

**Implementation Dependencies:** M015 stability tier must be specified and communicated as the replacement income opportunity before M014 activation announcement.

**Blocks:**
- Delegator communication materials
- Governance Proposal 2 "Staker Impact Analysis" section
- M014 Phase 2/Phase 3 inflation ramp-down schedule

---

## M015: Contribution-Weighted Rewards

### OQ-M015-1: 6% Stability Tier Sustainability

**Question (from spec):** Is 6% the right stability tier return? Must be sustainable from Community Pool inflows.

**Priority:** P1 -- blocks mainnet. The stability tier is a core value proposition for the PoA transition narrative ("here is your replacement for staking yield"). If 6% is unsustainable, it must be adjusted before launch to avoid a credibility crisis.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** 6% is sustainable above approximately $3M committed capital at $50K/month fee revenue. Start at 6% with governance ability to adjust. Monitor quarterly and propose adjustments if revenue falls below operational cost thresholds. The 30% cap on stability tier allocation within the Community Pool provides a safety valve.

**Rationale:** 6% is competitive with DeFi staking yields while being achievable under moderate fee projections. The cap at 30% of Community Pool inflow prevents the stability tier from consuming all contributor rewards. Governance adjustability provides a release valve if assumptions prove wrong.

**Status:** RESOLVED -- The OQ resolution doc provides analysis with revenue thresholds.

**Implementation Dependencies:** OQ-M013-1 must be resolved (the community pool share determines the inflow that funds the stability tier). Revenue projections from the economic-reboot-proposals.md inform sustainability analysis.

**Blocks:**
- M015 stability tier parameter configuration
- Delegator messaging ("what replaces staking yield")
- Community Pool sustainability modeling

---

### OQ-M015-2: Facilitation Identification

**Question (from spec):** Should platform facilitation credit use metadata fields or originating API key / registered dApp address?

**Priority:** P2 -- deferrable. Facilitation identification is a refinement that can be implemented after v0 launch. The core M015 activity scoring works without facilitation credit (it uses the other four activity types).

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Primary identification via transaction metadata (memo field) with the facilitator's registered identifier. Fallback identification via registered dApp address matching. Address registration through a facilitation registry maintained by governance.

**Rationale:** Dual identification (memo + address) ensures comprehensive coverage. The memo is flexible for ad-hoc facilitation; address matching is automatic for programmatic dApps. A governance-maintained registry provides an authoritative source of truth.

**Status:** RESOLVED -- The OQ resolution doc provides a clear technical recommendation.

**Implementation Dependencies:** x/gov must support a facilitation address registry parameter. Memo field parsing must be added to the `x/rewards` module.

**Blocks:**
- Facilitation credit calculation in `x/rewards`
- dApp registration process
- Platform partnership agreements

---

### OQ-M015-3: Community Pool Split (Auto vs Governance-Directed)

**Question (from spec):** What share of Community Pool goes to automatic distribution vs. remaining available for governance proposals?

**Priority:** P1 -- blocks mainnet. The split determines contributor income predictability and governance flexibility. Must be resolved before M015 activates (Stage 4).

**Recommended Resolution Owner:** Community Governance

**Concrete Recommended Answer:** 70% automatic M015 distribution / 30% governance-directed. Both percentages adjustable through governance proposals. Review annually.

**Rationale:** Prioritizes operational predictability for contributor retention (contributors need reliable income to commit to the ecosystem) while preserving meaningful governance discretion (30% is enough for grants, emergency funding, and strategic initiatives). The split can be adjusted as the network matures and the WG gains data on actual usage patterns.

**Status:** NEEDS_GOVERNANCE -- The OQ resolution doc recommends 70/30 but flags for governance vote.

**Implementation Dependencies:** M013 must be active (provides Community Pool inflow). M015 module must support the split as a governance parameter.

**Blocks:**
- M015 distribution logic (automatic vs. governance-directed allocation)
- Governance Proposal 4 (M015 activation)
- Contributor compensation predictability

---

### OQ-M015-4: Anti-Gaming Measures

**Question (from spec):** Are additional anti-gaming measures needed beyond M013 fee friction (e.g., minimum transaction size for reward eligibility)?

**Priority:** P2 -- deferrable. M013 fee friction provides the primary economic defense. Additional measures can be calibrated after observing 3 months of tracking data (M015 starts in TRACKING state before DISTRIBUTING).

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** M013 transaction fees as primary economic disincentive. Minimum 1 REGEN transaction value for reward eligibility. AGENT-003 address correlation analysis for pattern-based wash trading detection. Flagged transactions have facilitation attribution suspended pending review.

**Rationale:** Layered defense combining economic disincentives (fees make wash trading unprofitable), simple rules (minimum transaction size eliminates dust attacks), and AI monitoring (AGENT-003 detects sophisticated gaming patterns). The 3-month tracking period provides real data to calibrate thresholds before payouts begin.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation.

**Implementation Dependencies:** AGENT-003 must be operational for pattern detection. M013 must be active (provides fee friction).

**Blocks:**
- M015 reward eligibility filters
- AGENT-003 monitoring workflow configuration
- Tracking period anomaly detection criteria

---

## GOV-POA: Governance and PoA Transition

### OQ-GOV-POA-1: Per-Process Governance Weights

**Question (from resolution doc):** Should governance tally weights vary by process type, and if so, what should the weights be for each type?

**Priority:** P1 -- blocks mainnet. Per-process weights affect the `x/gov` tally logic. Must be implemented in the governance module upgrade before PoA activation.

**Recommended Resolution Owner:** Community Governance

**Concrete Recommended Answer:** Per-process weight table:
- Software upgrades: 70/30 (validator/holder)
- Treasury proposals: 50/50
- Registry changes: 60/40
- Parameter changes: 60/40
- Default (uncategorized): 60/40

Adjustable through governance proposals.

**Rationale:** Matches decision authority to decision type. Validators get stronger voice on technical matters (they bear the operational burden of upgrades). Treasury decisions affect all stakeholders equally (balanced voice). The default 60/40 provides a sensible fallback.

**Status:** NEEDS_GOVERNANCE -- The OQ resolution doc recommends specific weights but flags for governance vote.

**Implementation Dependencies:** OQ-M014-3 must be resolved (the validator set must be known before weighting it in governance tallies). The `x/gov` module must be extended to support per-process-type weight configuration.

**Blocks:**
- `x/gov` tally logic modification
- Governance Proposal 2 (M014 activation) -- includes governance weight parameters
- Proposal categorization mechanism

---

### OQ-GOV-POA-2: Parallel PoS/PoA Duration

**Question (from resolution doc):** How long should the PoS and PoA governance systems run in parallel during the transition?

**Priority:** P1 -- blocks mainnet. The parallel duration determines the transition safety net window and the timeline for full PoA commitment.

**Recommended Resolution Owner:** Cross-functional

**Concrete Recommended Answer:** 6-month minimum, 12-month maximum parallel operation. Transition to full PoA requires a governance proposal passing under both PoS and PoA systems (dual-system passage ensures mutual consent). If PoA criteria are not met after 12 months, a mandatory review triggers extension or reversion.

**Rationale:** Minimum provides sufficient stress-testing across multiple governance cycles and seasonal variation. Maximum prevents indefinite parallelism and the confusion of dual governance systems. Dual-passage requirement for the transition proposal ensures both stakeholder groups consent.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation.

**Implementation Dependencies:** M014 must support parallel operation with `x/staking`. The dual-passage governance mechanism must be implemented.

**Blocks:**
- M014 Phase 2 (PoA coexistence) duration configuration
- Transition proposal mechanism (dual-system voting)
- PoS sunset timeline planning

---

### OQ-GOV-POA-3: Existing Validator Treatment

**Question (from resolution doc):** How should existing PoS validators be treated in the PoA transition?

**Priority:** P1 -- blocks mainnet. Validator messaging and the transition path must be clear before any PoA announcement. This is critical for preventing validator exodus.

**Recommended Resolution Owner:** Cross-functional

**Concrete Recommended Answer:** Existing PoS validators transition to the token holder track in PoA governance. Operational experience is explicitly valued in PoA applications (the validator selection rubric awards points for node operation history). Existing validators who meet PoA criteria are encouraged to apply and have a natural advantage through their demonstrated track records.

**Rationale:** Respects existing validators' contributions without automatic grandfathering that could compromise PoA criteria. Clear path from PoS experience to PoA application maintains incentive for current validators to participate. The holder track provides meaningful governance participation for those who do not join the authority set.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation. The validator selection rubric operationalizes this with specific scoring criteria.

**Implementation Dependencies:** Validator selection rubric must be finalized and published. Communication materials must be prepared before announcement.

**Blocks:**
- Validator outreach and application process
- Communication plan execution (T-90, T-60, T-30 schedule)
- Validator selection rubric publication

---

## M001-ENH: Dual-Track Voting

### OQ-M001-1: 60/40 Validator-to-Holder Weight Split

**Question (from spec):** Should the default governance tally weight be fixed at 60% validator / 40% token holder, or should it vary by proposal type?

**Priority:** P2 -- deferrable. The *principle* of using 60/40 as the default with per-process variation is technically resolved. The *specific per-process weights* are the P1 governance decision tracked in OQ-GOV-POA-1. This question does not independently block any work beyond what OQ-GOV-POA-1 already gates.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Adopt 60/40 as the default with per-process variation: 70/30 for software upgrades, 50/50 for treasury, 60/40 for registry and parameter changes. See OQ-GOV-POA-1 for the full weight table.

**Rationale:** The principle that weights should vary by process type is resolved (yes, they should). The specific weights are governed by OQ-GOV-POA-1, which is P1/NEEDS_GOVERNANCE. The 60/40 default is a sensible fallback for uncategorized proposals regardless of the per-process weights chosen.

**Status:** RESOLVED -- The principle is resolved; the specific weights are governed by OQ-GOV-POA-1 (P1, NEEDS_GOVERNANCE). No contradiction: this question resolves the "should weights vary?" design question, while GOV-POA-1 resolves the "what should the specific weights be?" governance question.

**Implementation Dependencies:** OQ-GOV-POA-1 resolution (for specific per-process weight values).

**Blocks:**
- `x/gov` tally weight default configuration

---

### OQ-M001-2: Agent Score Influence on Governance Track

**Question (from spec):** Should agent reputation scores (from M010) influence which governance track a proposal follows?

**Priority:** P2 -- deferrable. The agent-influenced fast track is an optimization on top of the base governance system. Can be added in v1 after the agent reputation system (M010) has operational history.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Yes, agent scores >= 0.85 enable a validator-only fast track with a 48-hour voting window. The fast track applies only to parameter changes and registry updates, not to software upgrades or treasury proposals.

**Rationale:** Improves governance responsiveness for low-risk changes while preserving full deliberation for high-impact decisions. The 0.85 threshold limits access to the top 10-15% of agents by score.

**Status:** RESOLVED -- The OQ resolution doc provides a clear recommendation.

**Implementation Dependencies:** M010 must be deployed and producing scores (Stage 1). AGENT-002 workflows must be operational.

**Blocks:**
- Governance fast track implementation (v1)
- M010 score threshold configuration

---

## M009: Service Provision Escrow

### OQ-M009-1: Automatic Reputation Feedback

**Question (from spec):** Should successful escrow completion automatically generate a reputation signal in M010?

**Priority:** P2 -- deferrable. This is a cross-module integration refinement.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Yes, implement automatic M010 reputation signal on escrow completion with a 30-day delay for positive signals. Negative signals (dispute loss, deadline miss) apply immediately.

**Status:** RESOLVED

**Implementation Dependencies:** M009 and M010 must both be deployed (Stage 1).

**Blocks:**
- M009-M010 integration hook

---

### OQ-M009-2: Partial Milestones

**Question (from spec):** Should M009 support partial milestone completion and proportional escrow release in v0?

**Priority:** P2 -- deferrable. Binary milestones are sufficient for v0.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** No partial milestones in v0. Encourage granular milestone decomposition as a workaround. Add partial milestone support in v1 based on observed usage patterns.

**Status:** RESOLVED

**Implementation Dependencies:** None for v0.

**Blocks:**
- Nothing in v0; v1 partial milestone feature

---

### OQ-M009-3: Validator Override for Stuck Disputes

**Question (from spec):** Should validators have the ability to override or force-resolve stuck disputes?

**Priority:** P2 -- deferrable. The `resolution_deadline` parameter provides automatic escalation. Override is a v1 consideration.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** No validator override. Rely on the existing `resolution_deadline` parameter for automatic escalation to governance. Add 50/50 proportional return fallback for disputes unresolved after governance escalation.

**Status:** RESOLVED

**Implementation Dependencies:** Governance module must support dispute escalation proposals.

**Blocks:**
- M009 dispute resolution logic

---

### OQ-M009-4: Escrow and M013 Interaction

**Question (from spec):** How should platform fees from escrow transactions interact with the M013 fee distribution model?

**Priority:** P1 -- blocks mainnet. Escrow fees must route through M013 correctly before mainnet.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Platform fees from escrow transactions follow the standard M013 distribution model. Fees at release time are calculated on the actual released amount (not the original deposit).

**Status:** RESOLVED

**Implementation Dependencies:** M013 must be deployed. M009 release logic must compute fees on released amounts.

**Blocks:**
- M009-M013 fee integration

---

## M011: Marketplace Curation

### OQ-M011-1: Curator Token Holdings Requirement

**Question (from spec):** Should curators be required to hold a minimum amount of REGEN tokens?

**Priority:** P2 -- deferrable.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** No minimum holdings requirement. The curator bond is sufficient for alignment.

**Status:** RESOLVED

**Implementation Dependencies:** None.

**Blocks:**
- M011 curator eligibility criteria

---

### OQ-M011-2: Quality Scores On-Chain vs KOI

**Question (from spec):** Should quality scores be stored fully on-chain or hash-anchored with full details in KOI?

**Priority:** P2 -- deferrable.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Anchor the quality score hash on-chain. Store full details in KOI. Include score witnesses in transactions that depend on quality scores.

**Status:** RESOLVED

**Implementation Dependencies:** KOI integration must be operational.

**Blocks:**
- M011 quality score storage design

---

### OQ-M011-3: Curation Fee and M013 Interaction

**Question (from spec):** Should the curation fee be carved from the existing M013 trade fee or be separate?

**Priority:** P2 -- deferrable.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Separate fee paid by the requesting party, not carved from M013 trade fee. The curation fee payment itself is subject to standard M013 protocol fees.

**Status:** RESOLVED

**Implementation Dependencies:** M013 must be active.

**Blocks:**
- M011 curation fee implementation

---

### OQ-M011-4: Curator Reputation Tracking

**Question (from spec):** Should curator performance be tracked via M010?

**Priority:** P2 -- deferrable.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Yes, track via M010 with a dedicated "curator" category. Weight challenge outcomes so upheld scores boost reputation.

**Status:** RESOLVED

**Implementation Dependencies:** M010 must be deployed.

**Blocks:**
- M010 curator category extension

---

### OQ-M011-5: Basket Token Quality Scores

**Question (from spec):** Should quality scores apply to baskets as a whole or per-constituent?

**Priority:** P2 -- deferrable.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Basket-level scores assessing construction methodology, diversification, and basket-level performance. Constituent scores may inform but do not mechanically determine the basket score.

**Status:** RESOLVED

**Implementation Dependencies:** M011 quality scoring framework must be defined.

**Blocks:**
- M011 basket scoring logic

---

## Bioregional Validator Framework (Implicit OQs)

The bioregional-validators.md document identifies four additional open questions that were not formally numbered in the mechanism specs but are essential for the PoA migration.

### OQ-BIO-1: Bioregional Claim Verification

**Question (from bioregional-validators.md):** How to verify "bioregional" claims? Is self-attestation sufficient, or is third-party verification needed?

**Priority:** P1 -- blocks mainnet. If the validator set claims bioregional legitimacy, the verification mechanism must be credible.

**Recommended Resolution Owner:** Cross-functional

**Concrete Recommended Answer:** Require documentary evidence as part of the validator application (partnership agreements, community engagement reports, bioregional initiative participation records). AGENT-004 reviews evidence as part of the rubric scoring process. Self-attestation alone is insufficient; at minimum, one external reference or published organizational commitment must be provided. The validator selection rubric's "Bioregional Engagement" criterion (IB-6/RP-6/DS-6) already operationalizes this with scored evidence requirements.

**Rationale:** Self-attestation without verification makes the bioregional framing performative. Documentary evidence provides accountability without requiring a formal third-party audit (which would be expensive and slow).

**Status:** NOT_ADDRESSED -- Not covered in the OQ resolution doc. Partially addressed by the validator selection rubric's evidence requirements.

**Implementation Dependencies:** Validator selection rubric must be finalized with bioregional evidence requirements.

**Blocks:**
- Validator application process design
- Seed set evaluation criteria

---

### OQ-BIO-2: Bioregional Representation as Constraint vs Goal

**Question (from bioregional-validators.md):** Should bioregional representation be a formal constraint or aspirational goal?

**Priority:** P1 -- blocks mainnet. Determines whether M014 composition enforcement includes a bioregional dimension.

**Recommended Resolution Owner:** Community Governance

**Concrete Recommended Answer:** Aspirational goal for v0, with a target of representation across at least 4 continents and 6+ distinct bioregions. Do not encode as a hard constraint in v0 because the qualified applicant pool may not permit it. Formalize as a constraint in v1 after the first term cycle demonstrates feasibility.

**Rationale:** Hard constraints on bioregional distribution could make the 5/5/5 composition requirement impossible to fill if qualified applicants from certain bioregions do not exist yet. Starting as an aspirational goal with specific targets creates pressure toward diversity without creating an impossible constraint.

**Status:** NEEDS_GOVERNANCE -- Not covered in the OQ resolution doc. Reclassified from NOT_ADDRESSED because this decision directly affects validator composition enforcement and should be included in the Validator and Governance Structure governance proposal (Proposal B).

**Implementation Dependencies:** Validator selection rubric geographic diversity scoring (IB-5/RP-5/DS-5) already creates incentive structure.

**Blocks:**
- Validator composition enforcement logic (constraint vs. scoring)
- Validator outreach strategy (which bioregions to target)

---

### OQ-BIO-3: Bioregional Validator Coordination

**Question (from bioregional-validators.md):** How do bioregional validators coordinate with each other?

**Priority:** P2 -- deferrable. Coordination infrastructure can evolve after the initial set is established.

**Recommended Resolution Owner:** Cross-functional

**Concrete Recommended Answer:** Establish a shared communication channel (likely a dedicated forum category and a real-time chat channel) for bioregional validators. Encourage quarterly coordination calls. Consider a joint proposal mechanism where validators from different bioregions can co-author governance proposals reflecting cross-bioregional interests. This does not require protocol-level implementation.

**Status:** NOT_ADDRESSED -- Not covered in the OQ resolution doc.

**Implementation Dependencies:** Validator set must be established.

**Blocks:**
- Nothing at the protocol level; social coordination infrastructure

---

### OQ-BIO-4: Non-Technical Validator Support

**Question (from bioregional-validators.md):** What support infrastructure do non-technical bioregional orgs need to operate a validator?

**Priority:** P1 -- blocks mainnet. If the Ecological Data Stewards category requires 5 seats but qualified ecological organizations lack technical capacity, the composition requirement fails.

**Recommended Resolution Owner:** Cross-functional

**Concrete Recommended Answer:** Provide shared validator tooling and infrastructure partnerships. Options include: (a) hosted validator services where a technical partner runs the node on behalf of the ecological organization, (b) a validator-as-a-service package maintained by Infrastructure Builder validators, (c) AGENT-004 monitoring and alerting to reduce operational burden. The ecological organization retains the authority and governance rights; the technical infrastructure is delegated.

**Rationale:** The bioregional validator vision's success depends on ecological organizations -- not just technical operators -- participating as validators. Without support infrastructure, the Data Stewards category will default to technically capable organizations that happen to have ecological programs, rather than genuinely ecology-first organizations.

**Status:** NOT_ADDRESSED -- Not covered in the OQ resolution doc.

**Implementation Dependencies:** Infrastructure Builder validators must be willing to offer hosted services.

**Blocks:**
- Ecological Data Steward recruitment
- 5/5/5 composition feasibility
- Validator support documentation

---

## Implicit Questions (Gaps)

The following questions were not asked in any spec or supporting document but should have been. They represent gaps in the analysis that could cause implementation surprises or governance conflicts.

### OQ-IMPL-1: Zero Fee Revenue Periods

**Question:** What happens if fee revenue is zero for extended periods (e.g., market downturn, credit market freeze)?

**Priority:** P0 -- blocks implementation. The entire Economic Reboot assumes non-zero fee revenue. If revenue goes to zero, validators receive no compensation, the burn pool receives nothing, M015 rewards stop, and the network's economic model collapses.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Implement a minimum viable revenue threshold. If total fee revenue falls below a governance-defined floor for 4 consecutive epochs (28 days), trigger an emergency governance escalation. Options include: (a) temporary re-enabling of minimal inflation as a safety net (requires Layer 4 supermajority), (b) drawing from a pre-funded emergency reserve within the Community Pool, (c) reducing the validator set size to maintain per-validator compensation above sustainability thresholds. The mainnet migration plan should include a pre-funded emergency reserve of 6 months of projected validator compensation deposited in the Community Pool before M014 activates.

**Rationale:** Fee-only economics are inherently procyclical. During market downturns, transaction volume (and thus fee revenue) drops precisely when the network most needs stability. A contingency mechanism is essential.

**Status:** NOT_ADDRESSED -- Neither the specs nor the OQ resolution doc address the zero-revenue scenario.

**Implementation Dependencies:** M013 must include revenue monitoring and threshold alerting. AGENT-003 must track fee revenue trends.

**Blocks:**
- Validator compensation sustainability under adverse conditions
- M014 Validator Fund minimum balance policy
- Emergency governance procedures

---

### OQ-IMPL-2: Cosmos SDK v0.54 x/poa Module Readiness

**Question:** How is the PoA module implemented given the Cosmos SDK version dependency? The native x/poa ships in SDK v0.54 (expected Q2 2026), but Regen Ledger is currently on v0.53.4.

**Priority:** P0 -- blocks implementation. The implementation path for M014 depends on which module is used. If the Cosmos Labs native x/poa is not ready in time, the team must either use the Strangelove PoA wrapper (which wraps `x/staking` rather than replacing it) or build a custom `x/authority` module.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Plan for SDK v0.54 upgrade as a prerequisite for M014 activation. The native x/poa module is the recommended path because it replaces (rather than wraps) `x/staking`, providing cleaner architecture. If the SDK v0.54 timeline slips, fall back to the Strangelove PoA module (audited by Hashlock, November 2024) as an interim solution with a planned migration to native x/poa when available. Begin integration testing with the Strangelove module immediately as a risk mitigation.

**Rationale:** The cosmos-poa-module.md document identifies both options and recommends Cosmos Labs native for new implementations. But the timeline dependency on SDK v0.54 is a hard external constraint outside Regen's control. Dual-track planning mitigates this risk.

**Status:** NOT_ADDRESSED -- The specs assume x/poa availability but do not address the version dependency or fallback plan.

**Implementation Dependencies:** Cosmos SDK v0.54 release (external dependency). CometBFT v0.39 compatibility. IBC-go v11 compatibility.

**Blocks:**
- M014 implementation technology choice
- Regen Ledger v6.0/v7.0 upgrade planning
- Stage 3 timeline feasibility

---

### OQ-IMPL-3: IBC Implications of PoA Transition

**Question:** What are the IBC implications of transitioning from PoS to PoA, and how are active IBC channels preserved?

**Priority:** P0 -- blocks implementation. Regen has active IBC channels. The 07-tendermint light client protocol that secures IBC connections assumes a PoS validator set. A sudden large change in validator power (as occurs during PoA transition) can break IBC connections if counterparty chains' light clients cannot track the validator set change.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Implement strict power change rate limiting during the transition (maximum 30% validator power change per block, per the Strangelove module's approach). Monitor IBC connection health continuously during the transition. Coordinate with counterparty chain relayers before the transition begins. Prepare a rollback plan if IBC channels break. Test the full transition on a testnet with active IBC connections before mainnet.

**Rationale:** The cosmos-poa-module.md document explicitly calls this out under "IBC Safety During Migration." Both the Strangelove and Cosmos Labs modules address this, but the migration plan must include concrete IBC health monitoring and rollback procedures.

**Status:** NOT_ADDRESSED -- The cosmos-poa-module.md mentions the risk but the mainnet migration plan (Phase 4.2) does not include specific IBC safety procedures for the validator set transition.

**Implementation Dependencies:** IBC connection inventory must be documented. Counterparty chain relayer operators must be contacted. Testnet must include active IBC connections.

**Blocks:**
- Stage 3 migration runbook (IBC safety section)
- Testnet IBC testing plan
- Counterparty chain coordination

---

### OQ-IMPL-4: Stability Tier Locks and Governance Voting Power

**Question:** How do M015 stability tier locks interact with governance voting power? Locked tokens are illiquid but economically active -- should they retain full governance voting rights?

**Priority:** P1 -- blocks mainnet. If stability-locked tokens cannot vote, a significant portion of the token supply could be disenfranchised. If they can vote, the stability tier creates a de facto governance power concentration among long-term holders who are also earning fixed returns.

**Recommended Resolution Owner:** Tokenomics WG

**Concrete Recommended Answer:** Stability-locked tokens retain full governance voting rights. The lock is an economic commitment, not a governance restriction. Holders who commit to the network's long-term stability should not be penalized by losing their voice in governance decisions that affect that stability. This aligns incentives: long-term holders have the strongest interest in good governance.

**Rationale:** Disenfranchising locked tokens would create a perverse incentive to avoid the stability tier, undermining M015's purpose. It would also concentrate governance power among active traders (who keep tokens liquid) rather than committed holders (who lock tokens).

**Status:** NEEDS_GOVERNANCE -- Neither M015 nor the governance specs address voting rights for stability-locked tokens. Reclassified from NOT_ADDRESSED because this decision affects governance power distribution and should be included in the Community Pool and Operations governance proposal (Proposal C).

**Implementation Dependencies:** The `x/rewards` module's stability tier must integrate with `x/gov` for voting delegation.

**Blocks:**
- `x/rewards` stability tier - governance integration
- Governance voting power calculation logic

---

### OQ-IMPL-5: x/feerouter Module and Non-REGEN Credit Pricing

**Question:** How does the fee router handle credits denominated in non-REGEN currencies (e.g., USDC, ATOM) for fee calculation purposes?

**Priority:** P1 -- blocks mainnet. The on-chain marketplace supports multi-denom trading. If a credit is priced in USDC, the TWAP calculation (OQ-M013-2) produces a USDC price, but the fee must be relatable to REGEN for the burn mechanism.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** For v0, require all fee calculations to reference a REGEN-denominated price. Credits priced in non-REGEN denoms use a REGEN/denom exchange rate (sourced from the on-chain DEX or a governance-set reference rate) to convert. The TWAP computation operates on REGEN-equivalent values. For v1, the hybrid denomination approach (OQ-M013-3) handles multi-denom natively.

**Rationale:** v0 simplification avoids the complexity of multi-denom fee computation while the hybrid approach is being developed. The REGEN-equivalent conversion is deterministic and verifiable.

**Status:** NOT_ADDRESSED -- The M013 spec discusses fee denomination (OQ-M013-3) but does not detail how non-REGEN credit pricing interacts with fee calculation.

**Implementation Dependencies:** Exchange rate oracle or governance-set rate parameter. On-chain DEX liquidity for REGEN pairs.

**Blocks:**
- `x/feerouter` multi-denom fee calculation
- TWAP computation for non-REGEN-priced credits

---

### OQ-IMPL-6: Emergency Validator Set Recovery

**Question:** If the PoA validator set drops below the BFT safety threshold (e.g., multiple validators go offline simultaneously), what is the emergency recovery procedure?

**Priority:** P1 -- blocks mainnet. M014 specifies emergency governance escalation when the set drops below `min_validators` (15), but does not detail the recovery mechanism. Under PoA, there is no open staking to attract replacement validators quickly.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Implement a tiered emergency response: (1) If active validators drop below 15 but above 10 (2/3 + 1 for BFT with 15), AGENT-004 triggers alerts and governance fast-tracks replacement validator onboarding from the approved candidate pool. (2) If active validators drop below 10, a pre-approved emergency validator list (maintained by governance) can be activated by a multisig without full governance proposal. (3) If the chain halts, the remaining validators coordinate an emergency restart with the pre-approved list. Maintain a standing pool of 3-5 pre-approved backup validators at all times.

**Rationale:** PoA systems are more vulnerable to validator loss than PoS because replacement validators cannot self-onboard. The emergency response must be faster than the standard governance process.

**Status:** NOT_ADDRESSED -- M014 mentions emergency governance escalation but does not specify the recovery procedure or backup validator pool.

**Implementation Dependencies:** Backup validator pre-approval governance process. Emergency multisig configuration. AGENT-004 alerting thresholds.

**Blocks:**
- M014 emergency recovery procedures
- Backup validator pool governance
- Chain halt recovery runbook

---

### OQ-IMPL-7: Fee Router Upgrade Path

**Question:** How is the `x/feerouter` module upgraded if fee calculation logic needs to change post-deployment?

**Priority:** P2 -- deferrable. This is an operational concern for post-launch maintenance.

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** The `x/feerouter` module follows standard Cosmos SDK module upgrade procedures. Fee rates and distribution shares are governance parameters (changeable via parameter change proposals without binary upgrades). Fee calculation logic changes (e.g., adding new transaction types, modifying TWAP computation) require a software upgrade proposal. The module should be designed with extensibility points for new transaction type handlers.

**Status:** NOT_ADDRESSED -- The specs do not discuss upgrade path for the fee router module.

**Implementation Dependencies:** None -- this is a design consideration for the module architecture.

**Blocks:**
- `x/feerouter` module architecture (extensibility design)

---

### OQ-IMPL-8: Cross-Mechanism State Consistency During Migration

**Question:** How is state consistency maintained across M012, M013, M014, and M015 during the multi-stage migration where mechanisms activate at different times?

**Priority:** P0 -- blocks implementation. The migration plan deploys mechanisms across four stages spanning 18+ months. During intermediate stages, some mechanisms are active while others are not. For example, M013 is active in Stage 2 but M012 (which consumes M013's burn pool) is not active until Stage 3. What happens to the accumulated burn pool during the gap?

**Recommended Resolution Owner:** Engineering

**Concrete Recommended Answer:** Design each mechanism to operate correctly in isolation during its stage. Specifically: (a) M013 accumulates burn pool funds during Stage 2; these funds are held in the BurnPool module account until M012 activates in Stage 3. (b) M013 routes to the Validator Fund during Stage 2; validators continue receiving PoS inflation AND fee-based compensation during the overlap, with inflation ramping down as specified in the Phase 4.2 migration plan. (c) Community Pool inflow begins in Stage 2 but M015 does not distribute rewards until Stage 4; accumulated funds remain in the Community Pool for governance-directed spending. Each mechanism's state machine starts in INACTIVE or TRANSITION state and progresses independently based on its own activation criteria.

**Rationale:** The acyclic dependency graph (Phase 4.2 principle 3) ensures disabling a later mechanism never breaks an earlier one. But the accumulation of funds in pools that do not yet have a consumer must be explicitly handled to prevent governance confusion (e.g., "there are 50,000 REGEN sitting in the burn pool -- can we redirect them?").

**Status:** NOT_ADDRESSED -- The Phase 4.2 migration plan describes the stage sequence but does not address inter-stage fund accumulation or state consistency.

**Implementation Dependencies:** All mechanism module accounts must be created in the correct stage. Governance communication must explain why funds are accumulating but not yet being consumed.

**Blocks:**
- Stage 2 deployment procedures (fund accumulation policy)
- Governance communication about inter-stage fund management
- Module account lifecycle management

---

## Summary Table

| ID | Module | Question Summary | Priority | Owner | Status | Blocks |
|----|--------|-----------------|----------|-------|--------|--------|
| OQ-M012-1 | M012 | Hard cap value | P0 | Community Governance | NEEDS_GOVERNANCE | Supply module, simulations, Proposal 3 |
| OQ-M012-2 | M012 | Ecological multiplier oracle | P2 | Engineering | RESOLVED | v1 ecological feature |
| OQ-M012-3 | M012 | Burn period length | P1 | Engineering | RESOLVED | Epoch scheduler, M013 accumulation |
| OQ-M012-4 | M012 | Permanent burn vs reserve | P0 | Tokenomics WG | RESOLVED | Burn execution architecture |
| OQ-M012-5 | M012 | Which multiplier phase | P1 | Tokenomics WG | RESOLVED | M012-M014-M015 integration |
| OQ-M013-1 | M013 | Fee distribution model | P0 | Community Governance | NEEDS_GOVERNANCE | All downstream economics |
| OQ-M013-2 | M013 | Credit value determination | P0 | Engineering | RESOLVED | Fee calculation logic |
| OQ-M013-3 | M013 | Fee denomination | P1 | Community Governance | NEEDS_GOVERNANCE | Multi-denom handling |
| OQ-M013-4 | M013 | Agent infra fund governance | P2 | Tokenomics WG | RESOLVED | Agent fund management |
| OQ-M013-5 | M013 | Burn pool existence/size | P0 | Community Governance | NEEDS_GOVERNANCE | M012 viability, deflation model |
| OQ-M014-1 | M014 | Validator set size | P1 | Tokenomics WG | RESOLVED | Compensation modeling |
| OQ-M014-2 | M014 | Performance bonus | P2 | Tokenomics WG | RESOLVED | Bonus logic, AGENT-004 |
| OQ-M014-3 | M014 | Seed set selection | P0 | Community Governance | NEEDS_GOVERNANCE | Stage 3, Proposal 2 |
| OQ-M014-4 | M014 | Activation timeline | P1 | Cross-functional | RESOLVED | Testnet/mainnet scheduling |
| OQ-M014-5 | M014 | Delegated REGEN impact | P1 | Cross-functional | RESOLVED | Delegator comms, Proposal 2 |
| OQ-M015-1 | M015 | 6% sustainability | P1 | Tokenomics WG | RESOLVED | Stability tier config |
| OQ-M015-2 | M015 | Facilitation identification | P2 | Engineering | RESOLVED | Facilitation credit logic |
| OQ-M015-3 | M015 | Community Pool split | P1 | Community Governance | NEEDS_GOVERNANCE | Distribution logic, Proposal 4 |
| OQ-M015-4 | M015 | Anti-gaming measures | P2 | Engineering | RESOLVED | Reward eligibility filters |
| OQ-GOV-POA-1 | GOV-POA | Per-process weights | P1 | Community Governance | NEEDS_GOVERNANCE | x/gov tally, Proposal 2 |
| OQ-GOV-POA-2 | GOV-POA | Parallel PoS/PoA duration | P1 | Cross-functional | RESOLVED | Phase 2 duration |
| OQ-GOV-POA-3 | GOV-POA | Existing validator treatment | P1 | Cross-functional | RESOLVED | Validator outreach |
| OQ-M001-1 | M001-ENH | Tally weight default | P2 | Tokenomics WG | RESOLVED | x/gov config |
| OQ-M001-2 | M001-ENH | Agent fast track | P2 | Tokenomics WG | RESOLVED | v1 fast track |
| OQ-M009-1 | M009 | Auto reputation feedback | P2 | Engineering | RESOLVED | M009-M010 integration |
| OQ-M009-2 | M009 | Partial milestones | P2 | Engineering | RESOLVED | v1 partial milestone |
| OQ-M009-3 | M009 | Stuck dispute resolution | P2 | Tokenomics WG | RESOLVED | Dispute escalation |
| OQ-M009-4 | M009 | Escrow-M013 interaction | P1 | Engineering | RESOLVED | M009-M013 fee integration |
| OQ-M011-1 | M011 | Curator holdings | P2 | Tokenomics WG | RESOLVED | Curator eligibility |
| OQ-M011-2 | M011 | Quality score storage | P2 | Engineering | RESOLVED | Score storage design |
| OQ-M011-3 | M011 | Curation fee source | P2 | Tokenomics WG | RESOLVED | Curation fee impl |
| OQ-M011-4 | M011 | Curator reputation | P2 | Engineering | RESOLVED | M010 extension |
| OQ-M011-5 | M011 | Basket quality scores | P2 | Tokenomics WG | RESOLVED | Basket scoring |
| OQ-BIO-1 | Bioregional | Claim verification | P1 | Cross-functional | NOT_ADDRESSED | Validator applications |
| OQ-BIO-2 | Bioregional | Constraint vs goal | P1 | Community Governance | NEEDS_GOVERNANCE | Composition enforcement |
| OQ-BIO-3 | Bioregional | Validator coordination | P2 | Cross-functional | NOT_ADDRESSED | Social infrastructure |
| OQ-BIO-4 | Bioregional | Non-technical support | P1 | Cross-functional | NOT_ADDRESSED | Data Steward recruitment |
| OQ-IMPL-1 | Cross-cutting | Zero fee revenue | P0 | Tokenomics WG | NOT_ADDRESSED | Validator compensation sustainability |
| OQ-IMPL-2 | Cross-cutting | SDK v0.54 readiness | P0 | Engineering | NOT_ADDRESSED | M014 tech choice |
| OQ-IMPL-3 | Cross-cutting | IBC safety | P0 | Engineering | NOT_ADDRESSED | Stage 3 migration |
| OQ-IMPL-4 | Cross-cutting | Stability locks + voting | P1 | Tokenomics WG | NEEDS_GOVERNANCE | Governance integration |
| OQ-IMPL-5 | Cross-cutting | Non-REGEN credit pricing | P1 | Engineering | NOT_ADDRESSED | Fee calculation |
| OQ-IMPL-6 | Cross-cutting | Emergency validator recovery | P1 | Engineering | NOT_ADDRESSED | Emergency procedures |
| OQ-IMPL-7 | Cross-cutting | Fee router upgrades | P2 | Engineering | NOT_ADDRESSED | Module architecture |
| OQ-IMPL-8 | Cross-cutting | Cross-mechanism state | P0 | Engineering | NOT_ADDRESSED | Stage 2+ deployment |

---

## Critical Path Analysis

### P0 Resolution Order

The 10 P0 items must be resolved in approximately the following order:

```
Phase A: Foundation Decisions (can be resolved in parallel)
  1. OQ-M012-4: Permanent burn vs reserve
     - Architectural decision, no upstream dependencies
     - Owner: Tokenomics WG (can resolve immediately)

  2. OQ-M013-2: Credit value determination
     - Engineering design decision, no upstream dependencies
     - Owner: Engineering (can resolve immediately)

  3. OQ-IMPL-2: SDK v0.54 readiness
     - External dependency assessment
     - Owner: Engineering (assess immediately, dual-track plan)

Phase B: Economic Parameters (sequential, governance-gated)
  4. OQ-M013-1: Fee distribution model
     - Root of all economic parameter chains
     - Must go to governance vote first
     - Owner: Community Governance (target: Q2 2026)

  5. OQ-M013-5: Burn pool existence/size
     - Depends on OQ-M013-1 (burn share is part of distribution)
     - Can be bundled with OQ-M013-1 governance vote
     - Owner: Community Governance

  6. OQ-M012-1: Hard cap value
     - Depends on burn pool size (determines time-to-cap)
     - Requires separate Layer 4 governance vote (67% supermajority)
     - Owner: Community Governance (target: Q2 2026)

Phase C: Operational Setup (after Phase B)
  7. OQ-IMPL-8: Cross-mechanism state consistency
     - Must be resolved before Stage 2 deployment (June 2026)
     - Defines fund accumulation policy during multi-stage rollout
     - Owner: Engineering

  8. OQ-M014-3: Seed set selection
     - Depends on all economic parameters being known
     - Requires validator outreach, application, scoring, governance vote
     - Owner: Community Governance (target: Q3 2026)

  9. OQ-IMPL-1: Zero fee revenue contingency
     - Design depends on knowing the fee distribution model
     - Must be resolved before testnet pilot (August 2026)
     - Owner: Tokenomics WG

 10. OQ-IMPL-3: IBC safety procedures
     - Must be resolved before Stage 3 migration
     - Requires IBC inventory, relayer coordination, testnet validation
     - Owner: Engineering (target: August 2026)
```

### Timeline Mapping

| Date | Milestone | P0 Items Due |
|------|-----------|-------------|
| April 2026 | Phase A complete | OQ-M012-4, OQ-M013-2, OQ-IMPL-2 |
| May-June 2026 | Governance votes | OQ-M013-1, OQ-M013-5, OQ-M012-1 |
| June 2026 | Stage 2 deployment begins | OQ-IMPL-8 |
| July-August 2026 | Validator applications | OQ-M014-3 |
| August 2026 | Testnet pilot begins | OQ-IMPL-1, OQ-IMPL-3 |
| Q4 2026 | Stage 3 mainnet | All P0 and P1 items resolved |

### P1 Items with Longest Lead Time

The following P1 items require the most calendar time and should be started immediately:

1. **OQ-BIO-4 (Non-technical validator support)** -- Requires identifying infrastructure partners, developing tooling, and testing with candidate organizations. Lead time: 3-6 months.
2. **OQ-M014-5 (Delegated REGEN impact)** -- Requires comprehensive communication campaign starting T-90 days before activation. Lead time: must begin by July 2026 for Q4 activation.
3. **OQ-GOV-POA-3 (Existing validator treatment)** -- Requires validator-by-validator outreach and application support. Lead time: 2-3 months of relationship work.

---

## Unresolved Gaps Summary

The following implicit questions were not addressed in any existing spec or resolution document. They represent genuine gaps that could cause implementation failures or governance crises if not resolved. Two items (OQ-IMPL-4 and OQ-BIO-2) have been reclassified as NEEDS_GOVERNANCE and are included in the governance proposal packaging below.

| Gap | Risk if Unresolved | Recommended Next Step |
|-----|--------------------|-----------------------|
| OQ-IMPL-1: Zero fee revenue | Validator exodus during downturn | Tokenomics WG designs contingency mechanism; must be resolved before testnet pilot (August 2026) |
| OQ-IMPL-2: SDK v0.54 dependency | Implementation blocked or on wrong technology | Engineering assesses timeline and begins Strangelove integration by April 2026 |
| OQ-IMPL-3: IBC safety | Active IBC channels break during migration | Engineering documents IBC inventory and contacts relayers by May 2026 |
| OQ-IMPL-5: Non-REGEN credit pricing | Fee calculation errors on multi-denom credits | Engineering designs conversion mechanism by May 2026 |
| OQ-IMPL-6: Emergency validator recovery | Chain halt with no recovery plan | Engineering designs tiered response by July 2026 |
| OQ-IMPL-7: Fee router upgrades | Inability to adapt fee logic post-launch | Engineering considers in module architecture design |
| OQ-IMPL-8: Cross-mechanism state | Fund accumulation confusion during multi-stage rollout | Engineering resolves before Stage 2 deployment |

> **Reclassified to NEEDS_GOVERNANCE:** OQ-IMPL-4 (stability locks + voting, Proposal C) and OQ-BIO-2 (bioregional representation constraint, Proposal B) require governance decisions and are tracked in the governance proposal packaging rather than this gaps table.

---

## Recommended Governance Proposal Packaging

The 9 NEEDS_GOVERNANCE items should be packaged into three governance proposals (consistent with the OQ resolution doc's recommendation):

### Proposal A: Economic Parameters (4 items)
- OQ-M012-1: Hard cap (Layer 4, 67% supermajority required)
- OQ-M013-1: Fee distribution model
- OQ-M013-3: Fee denomination
- OQ-M013-5: Burn pool size

**Note:** OQ-M012-1 requires Layer 4 supermajority while the others require Layer 3 majority. These may need to be separate on-chain proposals even though they are logically related.

### Proposal B: Validator and Governance Structure (3 items)
- OQ-M014-3: Seed set selection
- OQ-GOV-POA-1: Per-process governance weights
- OQ-BIO-2: Bioregional representation constraint

### Proposal C: Community Pool and Operations (2 items)
- OQ-M015-3: Community Pool auto/governance split
- OQ-IMPL-4: Stability tier voting rights

---

## References

- [Open Questions Resolution Analysis](./open-questions-resolution.md)
- [M012 Spec](../../mechanisms/m012-fixed-cap-dynamic-supply/SPEC.md)
- [M013 Spec](../../mechanisms/m013-value-based-fee-routing/SPEC.md)
- [M014 Spec](../../mechanisms/m014-authority-validator-governance/SPEC.md)
- [M015 Spec](../../mechanisms/m015-contribution-weighted-rewards/SPEC.md)
- [Bioregional Validator Framework](../bioregional-validators.md)
- [Validator Selection Rubric](./validator-selection-rubric.md)
- [Cosmos PoA Module Reference](../cosmos-poa-module.md)
- [Economic Reboot Proposals](./economic-reboot-proposals.md)
- [Mainnet Migration Plan](../../phase-4/4.2-mainnet-migration.md)
