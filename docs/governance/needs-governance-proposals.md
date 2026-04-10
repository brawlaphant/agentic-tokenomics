# Governance Proposals for NEEDS_GOVERNANCE Open Questions

**Status:** Draft for WG Review
**Date:** 2026-03-25
**Author:** Tokenomics Working Group (AGENT-assisted drafting)
**Scope:** 7 NEEDS_GOVERNANCE items from [Open Questions Resolution](./open-questions-resolution.md), packaged into 3 governance proposals

---

## Overview

The [Open Questions Resolution](./open-questions-resolution.md) triaged 33 open questions from Phase 2 module specifications. Twenty-six were resolved directly; seven require formal governance votes. This document packages those seven items into three governance proposals, each containing copy-paste-ready text, parameter tables, risk assessments, and voting guidance.

These proposals are **complementary** to the mechanism activation proposals in [Economic Reboot Proposals](./economic-reboot-proposals.md) (Proposals 1-5). The Economic Reboot Proposals activate M012-M015 on-chain. These proposals resolve the parameter ambiguities that the activation proposals depend on. They should be deliberated and voted on **before** the corresponding activation proposals are submitted.

### Source Documents

- [Open Questions Resolution](./open-questions-resolution.md) (NEEDS_GOVERNANCE analysis and recommendations)
- [Economic Reboot Proposals](./economic-reboot-proposals.md) (mechanism activation proposals)
- [M012 SPEC](../../mechanisms/m012-fixed-cap-dynamic-supply/SPEC.md)
- [M013 SPEC](../../mechanisms/m013-value-based-fee-routing/SPEC.md)
- [M014 SPEC](../../mechanisms/m014-authority-validator-governance/SPEC.md)
- [M015 SPEC](../../mechanisms/m015-contribution-weighted-rewards/SPEC.md)

### Proposal Grouping

| Proposal | Resolves | Affects |
|----------|----------|---------|
| Proposal A: Economic Parameters Resolution | OQ-M012-1, OQ-M013-1, OQ-M013-3, OQ-M013-5 | Proposals 1 and 3 from Economic Reboot |
| Proposal B: Validator Structure Resolution | OQ-M014-3, OQ-GOV-POA-1 | Proposal 2 from Economic Reboot |
| Proposal C: Community Pool Operations Resolution | OQ-M015-3 | Proposal 4 from Economic Reboot |

---

## PROPOSAL A: Economic Parameters Resolution

### 1. Title

**Resolve Economic Reboot Parameters: Hard Cap, Fee Distribution, and Burn Pool**

### 2. Type

Parameter Change Proposal (governance parameter resolution — sets binding values for subsequent mechanism activation proposals)

### 3. Deposit

500 REGEN

> Verify current `min_deposit` via `regen query gov params` before submission.

### 4. Description (Copy-Paste Ready for On-Chain Submission)

> This proposal resolves four interrelated economic parameters that are prerequisites for the Economic Reboot mechanism activation. Each parameter was identified during Phase 2 specification as requiring community governance input before implementation. The Tokenomics Working Group has analyzed the tradeoffs and presents recommended values, but the final decision rests with the governance community.
>
> **Parameter 1 -- REGEN Hard Cap (OQ-M012-1)**
>
> The M012 Fixed Cap Dynamic Supply mechanism requires a hard cap on total REGEN supply. The current circulating supply is approximately 224 million REGEN. We propose setting the hard cap at 221,000,000 REGEN -- approximately 1.3% below current supply. This creates immediate but manageable scarcity pressure: the network must burn roughly 3 million tokens through organic fee-driven burns before regrowth minting can begin. At moderate fee volumes, this gap closes within 12-18 months. Setting the cap below current supply signals genuine deflationary commitment and ties recovery to real network activity. The hard cap is a Layer 4 (Constitutional) parameter requiring a 67% supermajority to change in the future.
>
> **Parameter 2 -- Fee Distribution Model (OQ-M013-1 and OQ-M013-5)**
>
> The M013 Value-Based Fee Routing mechanism splits fee revenue across four pools. The original spec presented two models: Model A (30% burn, 40% validator, 25% community, 5% agent) and Model B (25-35% burn, 15-25% validator, 50-60% community, 0% agent). After extensive analysis, including the burn pool size reduction recommended in OQ-M013-5, we propose a revised distribution: 15% burn, 30% validator, 50% community, 5% agent.
>
> This distribution reflects a deliberate shift toward ecosystem development while preserving meaningful deflation. The 15% burn pool (reduced from the earlier 28% compromise) maintains deflationary credibility -- tokens are still permanently destroyed every epoch -- while redirecting 13 additional percentage points to contributor-facing allocations. The 50% community allocation makes the Community Pool the primary distribution channel, directly funding M015 activity rewards and governance-directed spending. The 30% validator allocation sustains a compensated, professional validator set. The 5% agent allocation bootstraps AI agent infrastructure.
>
> **Parameter 3 -- Fee Denomination (OQ-M013-3)**
>
> We propose a hybrid fee denomination model. Fees are collected in whatever token denomination the transaction uses (preserving user experience for participants who transact in USDC, ATOM, or other IBC tokens). The burn portion (15%) is auto-converted to REGEN at epoch boundaries via on-chain DEX or IBC swap before burning -- the burn mechanism requires REGEN. The remaining 85% (validator, community, agent portions) is distributed in the collected denomination. Batch conversion at epoch boundaries minimizes slippage compared to per-transaction conversion.
>
> **Parameter 4 -- Burn Pool Size (OQ-M013-5)**
>
> As described in Parameter 2, the burn pool is set at 15% of fee revenue -- reduced from the original 28-30% proposals. This reduction reflects the Working Group's judgment that direct ecosystem funding through the Community Pool (50%) provides stronger near-term value creation than aggressive token burning. The burn pool remains large enough to sustain the M012 deflationary mechanism and to demonstrate the network's commitment to supply management. The 15% rate is adjustable via standard parameter change proposal if the community wishes to shift the balance between deflation and direct funding.
>
> All four parameters are adjustable through subsequent governance proposals. We recommend a mandatory review vote at 6 months post-activation to evaluate whether the initial values are producing the desired outcomes.

### 5. Parameter Table

| Parameter | Current / Default | Proposed Value | On-Chain Representation | Change Rationale |
|-----------|------------------|---------------|------------------------|------------------|
| `hard_cap` | None (inflationary) | 221,000,000 REGEN | 221,000,000,000,000 uregen | Below-current-supply cap creates immediate scarcity; achievable gap via organic burns |
| `distribution.burn_share` | 0.30 (Model A default) | 0.15 | 1,500 bps | Reduced from 28-30% to fund ecosystem development; preserves deflationary mechanism |
| `distribution.validator_share` | 0.40 (Model A default) | 0.30 | 3,000 bps | Sustains professional validator set; ~$400-1,000/validator/month at moderate volumes |
| `distribution.community_share` | 0.25 (Model A default) | 0.50 | 5,000 bps | Primary distribution channel; funds M015 rewards and governance spending |
| `distribution.agent_share` | 0.05 (Model A default) | 0.05 | 500 bps | Bootstrap agent infrastructure; sufficient for estimated $282/month operational cost |
| `fee_denomination` | REGEN-only | Hybrid (collect in tx denom; auto-convert burn portion to REGEN) | Module config flag | Preserves UX; ensures burn always operates on REGEN |

> **Consistency note:** The distribution shares sum to 1.00 (15% + 30% + 50% + 5% = 100%). This resolves the tension between OQ-M013-1 and OQ-M013-5: the earlier compromise proposed {28% burn, 25% validator, 45% community, 2% agent}, but OQ-M013-5 recommended reducing burn to 15%. The freed-up allocation (13%) is redistributed to validator (+5%), community (+5%), and agent (+3%) pools.
>
> **Simulation validation:** The cadCAD simulation (`simulations/cadcad/`) uses Model A (30% burn) as its baseline but the parameter sweep covers burn_share from 0% to 35%, validating both configurations. At 15% burn, equilibrium supply rises to ~220.42M REGEN (vs ~219.85M at 30%). Validator sustainability is maintained — see `simulations/cadcad/equilibrium_analysis.md` section 1.3.1 for the derivation.

### 6. Risk Assessment Matrix

| Risk | Likelihood | Impact | Combined | Mitigation |
|------|-----------|--------|----------|------------|
| 15% burn insufficient for meaningful deflation | Medium | Medium | **Medium** | At moderate fee volumes ($24K/month), 15% burn = $3,600/month = ~120,000 REGEN burned/month at $0.03. The 3M token gap to cap closes in ~25 months. If deflation is too slow, governance can increase burn_share. |
| 50% community allocation exceeds M015 absorption capacity | Low | Low | **Low** | Unabsorbed funds accumulate in Community Pool for governance-directed spending. No deficit risk -- excess is a feature. |
| Hybrid denomination introduces conversion slippage | Medium | Low | **Low-Medium** | Batch conversion at epoch boundaries (weekly) reduces per-unit slippage. Only 15% of fees require conversion. Limit orders or TWAP execution further mitigate. |
| Hard cap set too low, preventing necessary regrowth | Low | High | **Medium** | The cap is above any realistic near-term burning target. Regrowth activates automatically once supply falls below 221M. If cap is genuinely too low, Layer 4 governance (67% supermajority) can adjust. |
| Hard cap set too high, undermining scarcity signal | Low | Medium | **Low-Medium** | 221M is below current supply (~224M), creating immediate scarcity. No scenario exists where 221M fails to produce deflationary pressure at launch. |
| Validator compensation insufficient at 30% share | Medium | High | **Medium-High** | At moderate volumes ($24K/month), 30% = $7,200/month for 15-21 validators = $343-$480/validator/month. Marginal but supplemented by M015 activity rewards for contributing validators. Review at 6 months. |
| Community rejects compromise, prefers pure Model A or B | Medium | Medium | **Medium** | This proposal includes explicit YES/NO/ABSTAIN rationale (section 7). If rejected, the WG can submit refined parameters reflecting community feedback. |

### 7. Voting Recommendation

**Vote YES if:**
- You believe the network should prioritize ecosystem development (50% community allocation) while maintaining deflation (15% burn).
- You support a hard cap below current supply to create an immediate scarcity signal.
- You accept hybrid fee denomination as a pragmatic UX compromise.
- You are comfortable with parameter values being adjusted via future governance proposals after observing 6 months of data.

**Vote NO if:**
- You believe the burn percentage should be higher (>20%) to maximize deflationary pressure and token price appreciation.
- You believe the burn percentage should be zero, eliminating the burn pool entirely in favor of contributor funding.
- You believe the hard cap should be set at or above current supply (~224M) to allow regrowth from day one.
- You believe fees should be collected exclusively in REGEN to strengthen token demand.

**Vote ABSTAIN if:**
- You do not have a strong preference on the specific parameter values but support the Economic Reboot proceeding.
- You want to signal that the community should decide without your influence on this particular balance of priorities.

### 8. Dependencies

| Dependency | Direction | Description |
|-----------|-----------|-------------|
| Economic Reboot Proposal 1 (M013 Activation) | **Blocks** | Proposal 1 cannot set distribution parameters until this proposal resolves the fee distribution model |
| Economic Reboot Proposal 3 (M012 Activation) | **Blocks** | Proposal 3 cannot set `hard_cap` until this proposal resolves the cap value |
| Proposal B (this document) | None | Independent -- can be voted concurrently |
| Proposal C (this document) | None | Independent -- can be voted concurrently |
| PR #54 (Simulation Model) | **Informs** | Simulation should be re-run with decided parameters to confirm sustainability |

---

## PROPOSAL B: Validator Structure Resolution

### 1. Title

**Establish PoA Validator Seed Set and Governance Weight Structure**

### 2. Type

Text Proposal (establishes binding governance policy for validator selection process and per-process governance weights)

### 3. Deposit

500 REGEN

> Verify current `min_deposit` via `regen query gov params` before submission.

### 4. Description (Copy-Paste Ready for On-Chain Submission)

> This proposal establishes two foundational elements of the Proof-of-Authority governance transition: the process for selecting the initial validator seed set, and the per-process governance weight structure that determines how validator and token-holder votes are balanced across different proposal types.
>
> **Element 1 -- Seed Validator Set Selection Process (OQ-M014-3)**
>
> The initial PoA validator set bootstraps the authority governance model. We propose a three-step selection process:
>
> Step 1: Bootstrap from currently active Regen Network validators who meet PoA criteria. These validators have demonstrated sustained commitment through continued operation despite economic losses. Their on-chain track records (uptime, governance participation, block production) are publicly verifiable and provide the strongest available evidence of operational capability and mission alignment.
>
> Step 2: The Tokenomics Working Group evaluates candidates against the M014 composition requirements: minimum 5 Infrastructure Builders, minimum 5 Trusted ReFi Partners, and minimum 5 Ecological Data Stewards. Candidates not fitting existing categories may qualify under the flexible slots (positions 16-21). The WG publishes the recommended seed set with qualifying criteria documentation for each candidate.
>
> Step 3: The recommended seed set is submitted for a governance vote as a single slate. During the 14-day voting period, community members may raise objections to specific candidates via the governance forum. If substantive objections are raised and sustained, the WG may withdraw the proposal, address the objection, and resubmit with a revised slate.
>
> The composition requirements ensure diversity of expertise and perspective:
> - Minimum 5 Infrastructure Builders: active development of verification systems, dMRV tools, or registry infrastructure; demonstrable code contributions; operational history of 6 months or more.
> - Minimum 5 Trusted ReFi Partners: established ReFi organizations with public mission alignment; active Regen ecosystem participation; infrastructure meeting 99.5% uptime requirements.
> - Minimum 5 Ecological Data Stewards: organizations attesting to ecological data quality; participation in credit class development or verification; domain expertise in ecology, land management, or environmental science.
>
> **Element 2 -- Per-Process Governance Weights (OQ-GOV-POA-1)**
>
> Under PoA governance, different proposal types warrant different balances between validator expertise and broad token-holder representation. We propose the following per-process weight table:
>
> - Software Upgrades: 70% validator / 30% token holder. Upgrades require deep technical assessment and validators bear the operational burden. Strong validator authority on technical matters protects network stability.
> - Treasury Proposals: 50% validator / 50% token holder. The treasury belongs to the community. Equal weights ensure neither group can unilaterally direct spending.
> - Registry Changes: 60% validator / 40% token holder. Adding or modifying credit classes and methodologies requires moderate domain expertise. The default split provides balanced decision-making.
> - Parameter Changes: 60% validator / 40% token holder. Fee rates, epoch lengths, and thresholds are technical but have broad economic impact. Balanced weights reflect this dual nature.
> - Default (uncategorized): 60% validator / 40% token holder. Sensible fallback for proposals that do not fit other categories.
>
> These weights are encoded as governance module parameters and adjustable through future governance proposals. The categorization of each proposal is determined during the deposit period and reviewable by the governance community.
>
> Both elements are prerequisites for the M014 PoA Governance activation (Economic Reboot Proposal 2). The seed set selection process should begin immediately upon passage of this proposal, with a target of Q3 2026 for testnet deployment and Q4 2026 for mainnet activation, per the Economic Reboot Roadmap.

### 5. Parameter Table

| Parameter | Current Value | Proposed Value | On-Chain Representation | Change Rationale |
|-----------|-------------|---------------|------------------------|------------------|
| Seed set selection method | N/A (no PoA) | Bootstrap from current active validators + governance vote on slate | Text policy (enacted via M014 activation proposal) | Leverages existing track records; governance vote provides legitimacy |
| `governance_weight.software_upgrade` | 100% holder (PoS) | 70% validator / 30% holder | `[7000, 3000]` bps | Validators bear operational burden of upgrades; strong technical authority |
| `governance_weight.treasury` | 100% holder (PoS) | 50% validator / 50% holder | `[5000, 5000]` bps | Treasury is community-owned; equal representation |
| `governance_weight.registry` | 100% holder (PoS) | 60% validator / 40% holder | `[6000, 4000]` bps | Registry changes require domain expertise with broad stakeholder input |
| `governance_weight.parameter_change` | 100% holder (PoS) | 60% validator / 40% holder | `[6000, 4000]` bps | Technical parameters with economic impact; balanced authority |
| `governance_weight.default` | 100% holder (PoS) | 60% validator / 40% holder | `[6000, 4000]` bps | Sensible fallback for uncategorized proposals |
| `composition.min_infrastructure_builders` | N/A | 5 | uint32 | Ensures technical expertise in validator set |
| `composition.min_trusted_refi_partners` | N/A | 5 | uint32 | Ensures ReFi ecosystem representation |
| `composition.min_ecological_data_stewards` | N/A | 5 | uint32 | Ensures ecological domain expertise |
| Activation timeline | N/A | Q3 2026 testnet, Q4 2026 mainnet | Text policy | Aligned with Economic Reboot Roadmap and OQ-M014-4 |

### 6. Risk Assessment Matrix

| Risk | Likelihood | Impact | Combined | Mitigation |
|------|-----------|--------|----------|------------|
| Insufficient qualified applicants for 5/5/5 composition | Medium | High | **High** | Begin outreach 6 months before proposal. If fewer than 5 candidates exist for any category, accept 4/4/4 minimum for the initial set with a 6-month deadline to reach 5/5/5. Publish the shortfall and actively recruit. |
| Community rejects the proposed seed set | Low | High | **Medium** | Slate is published 2 weeks before on-chain submission for forum discussion. Individual objections can be addressed by substituting candidates. If wholesale rejection, the WG revises the slate based on community feedback. |
| Per-process weights give validators too much power | Medium | Medium | **Medium** | Even the strongest validator weight (70/30 for upgrades) requires validator supermajority; 30% holder voice still blocks extreme proposals. All weights adjustable via governance. Annual review recommended. |
| Per-process weights give validators too little power on technical matters | Low | Medium | **Low-Medium** | 70/30 is the strongest feasible weight without making holder votes decorative. Validators can always lobby holders; their technical arguments carry weight beyond the formal tally split. |
| Seed set becomes entrenched, resisting rotation | Medium | Medium | **Medium** | M014 enforces 12-month terms with mandatory re-application. Term accountability prevents indefinite incumbency. AGENT-004 monitors performance and flags underperformers. |
| Timeline slips, delaying entire Economic Reboot | Medium | Medium | **Medium** | Testnet and mainnet activations are gated on readiness criteria, not hard dates. A delayed but correct launch is preferable to a rushed one. Dependencies are sequenced so slippage propagates predictably. |

### 7. Voting Recommendation

**Vote YES if:**
- You support bootstrapping the PoA validator set from current active validators who meet defined criteria, with a governance vote providing community legitimacy.
- You believe governance weights should vary by proposal type, giving validators stronger voice on technical matters and balanced voice on treasury and economic matters.
- You accept the 5/5/5 composition minimums as ensuring meaningful diversity across Infrastructure Builders, Trusted ReFi Partners, and Ecological Data Stewards.
- You support the Q3 2026 testnet / Q4 2026 mainnet timeline for PoA activation.

**Vote NO if:**
- You believe the seed set should be selected through a fully open application process rather than bootstrapping from existing validators, even if this delays the timeline.
- You believe all proposal types should use the same governance weight (e.g., fixed 60/40 or 50/50) for simplicity.
- You believe the composition categories are wrong or insufficient (e.g., missing a category for community advocates or regional representatives).
- You believe the PoA transition timeline is premature and more community socialization is needed.

**Vote ABSTAIN if:**
- You support the PoA transition in principle but do not have strong views on the specific selection process or weight structure.
- You want to signal that the community should decide these structural questions without your influence.

### 8. Dependencies

| Dependency | Direction | Description |
|-----------|-----------|-------------|
| Economic Reboot Proposal 2 (M014 Activation) | **Blocks** | Proposal 2 cannot define the seed set or weight structure until this proposal establishes the process and values |
| Proposal A (this document) | None | Independent -- can be voted concurrently |
| Proposal C (this document) | None | Independent -- can be voted concurrently |
| PR #45 (Validator Selection Rubric) | **Informs** | Rubric should be updated to reflect the decided selection process |

---

## PROPOSAL C: Community Pool Operations Resolution

### 1. Title

**Activate M015 Community Pool Distribution: 70/30 Automatic/Governance Split**

### 2. Type

Parameter Change Proposal (sets the Community Pool distribution split for M015 activation)

### 3. Deposit

500 REGEN

> Verify current `min_deposit` via `regen query gov params` before submission.

### 4. Description (Copy-Paste Ready for On-Chain Submission)

> This proposal establishes how the Community Pool -- funded by M013 value-based fee routing -- divides its inflow between automatic M015 contribution-weighted distribution and governance-directed spending. This split is the final prerequisite for activating the M015 Contribution-Weighted Rewards mechanism.
>
> **The Split: 70% Automatic / 30% Governance-Directed**
>
> We propose that 70% of each epoch's Community Pool inflow is automatically distributed via the M015 contribution-weighted reward system, and 30% remains available for governance-directed spending (grants, emergency funding, strategic initiatives, and other discretionary allocations).
>
> The 70% automatic allocation ensures operational predictability for contributors. Participants who buy credits, retire credits, facilitate transactions, and participate in governance can rely on consistent, formula-driven rewards funded by real fee revenue. This predictability is essential for attracting and retaining contributors who build their operations around expected income from network participation.
>
> The 30% governance-directed portion preserves meaningful community agency. At moderate fee volumes ($24K/month total fees, $12K/month to Community Pool at the 50% share proposed in Proposal A), the governance-directed portion yields approximately $3,600/month -- sufficient for development grants, marketing initiatives, emergency responses, and strategic partnerships. This amount grows proportionally with network activity.
>
> **Activation Sequence**
>
> Per the M015 specification, activation proceeds in two phases:
> 1. TRACKING phase (3 months): Activity scores are computed and published but no distributions occur. This calibration period validates the scoring formula, identifies gaming patterns, and provides baseline data for the community.
> 2. DISTRIBUTING phase: After successful completion of the TRACKING phase and verification that no anomalies were detected, automatic distribution begins. The first distribution includes retroactive credit for all activity recorded during the TRACKING phase.
>
> The 3-month TRACKING phase is a safety mechanism, not a delay. It ensures that when real REGEN begins flowing to contributors, the scoring system has been validated against actual network behavior rather than theoretical models.
>
> **Annual Review Mandate**
>
> This proposal includes a mandatory annual review. At 12 months post-activation, the Tokenomics Working Group will submit a follow-up proposal that either (a) confirms the 70/30 split, (b) proposes adjusted percentages based on observed outcomes, or (c) proposes a fundamentally different distribution structure if warranted. The annual review ensures that the split adapts to changing network conditions and community priorities.
>
> If Community Pool revenue grows significantly, the 30% governance portion may become more than sufficient, allowing the WG to propose increasing the automatic distribution to 75% or 80%. If revenue is lower than projected, the WG may propose reducing the automatic distribution to ensure governance retains meaningful discretionary capacity.
>
> **What This Means for Participants**
>
> - Active contributors (credit buyers, retirers, facilitators, governance voters) will receive automatic weekly rewards proportional to their activity scores, funded by 70% of Community Pool inflow.
> - Stability Tier holders (committed REGEN lockers) receive their 6% annual return from the 70% automatic allocation, capped at 30% of Community Pool inflow per the M015 spec.
> - Governance proposers can continue to submit Community Pool spend proposals drawing from the 30% governance-directed portion (and from any accumulated unspent balance).
> - All distribution is constrained by actual fee revenue -- no inflation, no deficit spending.

### 5. Parameter Table

| Parameter | Current Value | Proposed Value | On-Chain Representation | Change Rationale |
|-----------|-------------|---------------|------------------------|------------------|
| `community_pool.auto_distribution_share` | N/A (no M015) | 0.70 (70%) | 7,000 bps | Operational predictability for contributors; majority of pool serves its stated purpose |
| `community_pool.governance_directed_share` | 1.00 (100% governance) | 0.30 (30%) | 3,000 bps | Preserves meaningful discretionary capacity; ~$3,600/month at moderate volumes |
| `m015.tracking_period` | N/A | 90 days (3 months) | 7,776,000 seconds | Calibration before live distribution; validates scoring, detects gaming |
| `m015.retroactive_tracking_credit` | N/A | true | boolean | Activity during TRACKING phase earns credit paid in first DISTRIBUTING epoch |
| `annual_review.required` | N/A | true | Governance policy (text) | Mandatory 12-month review ensures split adapts to observed outcomes |

### 6. Risk Assessment Matrix

| Risk | Likelihood | Impact | Combined | Mitigation |
|------|-----------|--------|----------|------------|
| 70% automatic allocation is too high, starving governance of discretionary funds | Low | Medium | **Low-Medium** | At moderate volumes, 30% governance = ~$3,600/month. If insufficient, a parameter change proposal can reduce automatic allocation. Annual review catches this within 12 months. |
| 70% automatic allocation is too low, insufficient rewards to attract contributors | Medium | Medium | **Medium** | Activity rewards scale with fee revenue, not just the split percentage. If rewards are too small, the signal is that network activity must grow. The 70/30 split maximizes the automatic portion without eliminating governance flexibility. |
| TRACKING phase reveals gaming patterns that invalidate the scoring model | Low | High | **Medium** | This is exactly why the TRACKING phase exists. If gaming is detected, the WG proposes scoring adjustments before DISTRIBUTING phase begins. Activation is gated on "no anomalies detected," not on a calendar date. |
| Contributors frustrated by 3-month delay before receiving rewards | Medium | Low | **Low-Medium** | Retroactive credit for TRACKING phase activity ensures no contribution goes unrewarded. Clear communication that the delay is a safety mechanism, not bureaucratic inertia. |
| Annual review is ignored or becomes perfunctory | Medium | Low | **Low** | The review is a governance proposal -- it requires deposit, voting period, and quorum. Even a perfunctory "confirm existing parameters" proposal generates community attention and an opportunity to raise concerns. |
| Community Pool inflow lower than projected, making both portions inadequate | Medium | Medium | **Medium** | Revenue constraint is a feature. If total Community Pool inflow is $2K/month, both the automatic and governance portions are small. This is an accurate signal that network activity needs to grow, not a flaw in the split. |

### 7. Voting Recommendation

**Vote YES if:**
- You believe automatic, formula-driven distribution of the majority of Community Pool funds creates the operational predictability necessary to attract and retain contributors.
- You accept that 30% governance-directed is sufficient discretionary capacity for grants, emergency funding, and strategic initiatives.
- You support the 3-month TRACKING phase as a prudent safety mechanism before live distribution begins.
- You support mandatory annual review to ensure the split adapts to changing conditions.

**Vote NO if:**
- You believe governance should retain majority control (>50%) of Community Pool funds, with M015 automatic distribution as a minority allocation.
- You believe the TRACKING phase is unnecessary and distributions should begin immediately upon M015 activation.
- You believe the split should be determined after observing M013 fee revenue for 6+ months, not before activation.
- You believe Community Pool funds should be exclusively governance-directed, with no automatic distribution mechanism.

**Vote ABSTAIN if:**
- You support M015 activation in principle but do not have a strong view on the specific split percentage.
- You want to signal that the community should determine the balance between operational predictability and governance flexibility without your influence.

### 8. Dependencies

| Dependency | Direction | Description |
|-----------|-----------|-------------|
| Proposal A (this document) | **Soft dependency** | Proposal A's community_share (50%) determines the absolute size of the Community Pool. Proposal C's 70/30 split applies to whatever inflow the Community Pool receives. Proposal C can pass before Proposal A, but the dollar amounts in the rationale assume Proposal A's 50% community share. |
| Economic Reboot Proposal 1 (M013 Activation) | **Required** | M013 must be active for Community Pool to receive fee revenue. No revenue = no distribution regardless of split. |
| Economic Reboot Proposal 4 (M015 Activation) | **Blocks** | Proposal 4 cannot set the auto/governance split until this proposal resolves the percentage. |
| Proposal B (this document) | None | Independent -- can be voted concurrently |

---

## Cross-Proposal Parameter Consistency Check

The three proposals in this document and the five proposals in the Economic Reboot Proposals document form a coherent parameter set. The following table verifies consistency:

| Parameter | Set By | Value | Consumed By | Consistent? |
|-----------|--------|-------|-------------|-------------|
| Hard cap | Proposal A | 221,000,000 REGEN | Proposal 3 (M012 activation) | Yes -- Proposal 3 uses this value |
| Burn share | Proposal A | 15% | Proposal 1 (M013 activation), Proposal 3 (M012 burn input) | Yes -- reduced from Proposal 1's preliminary 28% |
| Validator share | Proposal A | 30% | Proposal 1 (M013 activation), Proposal 2 (M014 compensation) | Yes -- increased from Proposal A's earlier 25% |
| Community share | Proposal A | 50% | Proposal 1 (M013 activation), Proposal 4 (M015 funding) | Yes -- increased from Proposal 1's preliminary 45% |
| Agent share | Proposal A | 5% | Proposal 1 (M013 activation), Proposal 5 (agent infra) | Yes -- increased from Proposal 1's preliminary 2% |
| Fee denomination | Proposal A | Hybrid | Proposal 1 (M013 activation) | Yes -- replaces Proposal 1's preliminary REGEN-only |
| Seed set process | Proposal B | Bootstrap + governance vote | Proposal 2 (M014 activation) | Yes -- Proposal 2 uses the seed set |
| Governance weights | Proposal B | Per-process (70/30 to 50/50) | Proposal 2 (M014 activation) | Yes -- encoded in x/authority module |
| Auto/governance split | Proposal C | 70/30 | Proposal 4 (M015 activation) | Yes -- Proposal 4 uses this split |

> **Note on Proposal 1 parameter updates:** The Economic Reboot Proposals document (Proposal 1) proposed a preliminary distribution of {28% burn, 25% validator, 45% community, 2% agent}. Proposal A in this document supersedes those preliminary values with {15% burn, 30% validator, 50% community, 5% agent}. If Proposal A passes, Proposal 1's parameter table must be updated to reflect the decided values before on-chain submission.

---

## Implementation Sequence

```
PHASE 1: Governance Votes (Q2 2026)
  Submit Proposals A, B, and C for community deliberation.
  All three can be submitted and voted on concurrently.
  Minimum 2 weeks of forum discussion before on-chain submission.

PHASE 2: Parameter Integration (immediately after votes)
  Update Economic Reboot Proposals 1-4 with decided parameter values.
  Re-run simulation (PR #54) with decided parameters.
  Confirm sustainability analysis holds.

PHASE 3: Mechanism Activation (Q2-Q4 2026, per Economic Reboot timeline)
  Submit Economic Reboot Proposals 1-5 sequentially per dependency chain.
  Decided parameters from Proposals A, B, C are embedded in activation proposals.
```

---

## Cosmos SDK Governance Parameter Change JSON

For implementers preparing on-chain submissions, the following JSON templates encode the parameter changes from each proposal.

### Proposal A -- Parameter Changes

```json
{
  "@type": "/cosmos.params.v1beta1.ParameterChangeProposal",
  "title": "Resolve Economic Reboot Parameters: Hard Cap, Fee Distribution, and Burn Pool",
  "description": "Sets binding parameter values for M012 hard cap, M013 fee distribution shares, M013 fee denomination mode, and M013 burn pool size. See governance forum post for full analysis.",
  "changes": [
    {
      "subspace": "supply",
      "key": "HardCap",
      "value": "\"221000000000000\""
    },
    {
      "subspace": "feerouter",
      "key": "BurnShare",
      "value": "\"0.150000000000000000\""
    },
    {
      "subspace": "feerouter",
      "key": "ValidatorShare",
      "value": "\"0.300000000000000000\""
    },
    {
      "subspace": "feerouter",
      "key": "CommunityShare",
      "value": "\"0.500000000000000000\""
    },
    {
      "subspace": "feerouter",
      "key": "AgentShare",
      "value": "\"0.050000000000000000\""
    },
    {
      "subspace": "feerouter",
      "key": "FeeDenominationMode",
      "value": "\"hybrid\""
    }
  ],
  "deposit": "500000000uregen"
}
```

### Proposal B -- Text Proposal

```json
{
  "@type": "/cosmos.gov.v1beta1.TextProposal",
  "title": "Establish PoA Validator Seed Set and Governance Weight Structure",
  "description": "Establishes the selection process for the initial PoA validator seed set (bootstrap from current active validators, governance vote on slate) and the per-process governance weight structure (Software Upgrades 70/30, Treasury 50/50, Registry 60/40, Parameter 60/40, Default 60/40). See governance forum post for full analysis.",
  "deposit": "500000000uregen"
}
```

> Note: Per-process governance weights will be encoded as module parameters when the `x/authority` module is deployed via the M014 Software Upgrade Proposal. The text proposal establishes the binding policy; the software upgrade implements it.

### Proposal C -- Parameter Changes

```json
{
  "@type": "/cosmos.params.v1beta1.ParameterChangeProposal",
  "title": "Activate M015 Community Pool Distribution: 70/30 Automatic/Governance Split",
  "description": "Sets the Community Pool distribution split: 70% automatic M015 contribution-weighted distribution, 30% governance-directed. Includes 90-day tracking period before live distribution. Mandatory annual review. See governance forum post for full analysis.",
  "changes": [
    {
      "subspace": "rewards",
      "key": "AutoDistributionShare",
      "value": "\"0.700000000000000000\""
    },
    {
      "subspace": "rewards",
      "key": "GovernanceDirectedShare",
      "value": "\"0.300000000000000000\""
    },
    {
      "subspace": "rewards",
      "key": "TrackingPeriodSeconds",
      "value": "\"7776000\""
    },
    {
      "subspace": "rewards",
      "key": "RetroactiveTrackingCredit",
      "value": "\"true\""
    }
  ],
  "deposit": "500000000uregen"
}
```

---

## Summary

| Proposal | Title | Type | OQs Resolved | Deposit | Can Vote Concurrently? |
|----------|-------|------|-------------|---------|----------------------|
| A | Resolve Economic Reboot Parameters | Parameter Change | OQ-M012-1, OQ-M013-1, OQ-M013-3, OQ-M013-5 | 500 REGEN | Yes (with B, C) |
| B | Establish PoA Validator Seed Set and Governance Weight Structure | Text Proposal | OQ-M014-3, OQ-GOV-POA-1 | 500 REGEN | Yes (with A, C) |
| C | Activate M015 Community Pool Distribution | Parameter Change | OQ-M015-3 | 500 REGEN | Yes (with A, B) |

**Total OQs resolved:** 7 distinct OQs across 3 proposals (OQ-M013-1 and OQ-M013-5 are resolved jointly in Proposal A)

**Total NEEDS_GOVERNANCE items addressed:** 7 of 7

| OQ ID | Question | Resolved In |
|-------|----------|-------------|
| OQ-M012-1 | Hard cap value | Proposal A |
| OQ-M013-1 | Fee distribution model | Proposal A |
| OQ-M013-3 | Fee denomination | Proposal A |
| OQ-M013-5 | Burn pool size | Proposal A |
| OQ-M014-3 | Initial trusted partners | Proposal B |
| OQ-GOV-POA-1 | Per-process governance weights | Proposal B |
| OQ-M015-3 | Community Pool auto/governance split | Proposal C |


---

*This document provides governance-ready proposal drafts for the 7 NEEDS_GOVERNANCE open questions identified during the Phase 2 comprehensive review. All recommendations represent the Tokenomics Working Group's best current analysis and should be refined through community deliberation before on-chain submission.*

*Prepared for the Regen Network Tokenomics Working Group, March 2026.*
