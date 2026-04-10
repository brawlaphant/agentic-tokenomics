# Open Questions Resolution: Phase 2 Comprehensive Review

**Status:** Draft for WG Review
**Date:** 2026-03-24
**Author:** Tokenomics Working Group (AGENT-assisted analysis)
**Scope:** All 33 open questions from Phase 2 module specifications

---

## Overview

This document resolves all 33 open questions identified during Phase 2 of the Agentic Tokenomics design process. Each question receives structured analysis, a concrete recommendation, rationale, and a status designation:

- **RESOLVED** — The recommendation is ready for implementation without further governance action. (26 questions)
- **NEEDS_GOVERNANCE** — The recommendation requires a formal governance proposal or WG vote before implementation. (7 questions)

The goal is to eliminate ambiguity and provide implementers with clear parameter choices, while flagging politically sensitive or high-impact decisions for community input.

---

## M001-ENH: Governance Tally Weights

### OQ-M001-1: 60/40 Validator-to-Holder Weight Split

**Question:** Should the default governance tally weight be fixed at 60% validator / 40% token holder, or should it vary by proposal type?

**Analysis:**

The 60/40 split was introduced as a compromise between validator expertise and broad token-holder representation. In practice, different proposal types carry different risk profiles and require different forms of expertise. Software upgrades, for example, demand deep technical knowledge that validators possess disproportionately. Treasury disbursements, on the other hand, affect all stakeholders equally and benefit from broader representation.

A fixed ratio creates simplicity but sacrifices nuance. Allowing per-process variation lets the governance system match decision authority to the nature of the decision. The concern about complexity is valid but manageable: the number of process types is small (fewer than five), and the weights can be encoded in module parameters rather than requiring per-proposal configuration.

The risk of gaming through proposal miscategorization is mitigated by the proposal deposit mechanism and the categorization review during the deposit period. Validators themselves have an incentive to correctly categorize proposals since miscategorization could reduce their own influence on technical matters.

**Recommendation:** Adopt 60/40 as the default, but allow per-process variation: 70/30 for software upgrades (validator-heavy), 50/50 for treasury proposals (balanced), 60/40 for registry and parameter changes. See also OQ-GOV-POA-1 for the full per-process weight table.

**Rationale:** Matches decision authority to decision type without excessive complexity. The default provides a sensible fallback for uncategorized proposals.

**Status:** RESOLVED

---

### OQ-M001-2: Agent Score Influence on Governance Track

**Question:** Should agent reputation scores (from M010) influence which governance track a proposal follows — specifically, should high-scoring agents be able to trigger a validator-only fast track?

**Analysis:**

The agent reputation system (M010) produces scores that reflect an agent's history of accurate attestations, successful task completions, and community trust. Scores above 0.85 represent the top tier of agent performance and indicate sustained reliable behavior over multiple evaluation periods.

A validator-only fast track for high-confidence proposals could significantly reduce governance latency for non-controversial changes. The current governance cycle (7-day voting period minimum) creates bottlenecks for urgent but straightforward parameter adjustments. If a trusted agent identifies and proposes a change with high confidence, and validators can review and approve within 48 hours, the network becomes more responsive.

The safeguard is that the fast track only bypasses the full voting period, not the deposit or review mechanisms. Validators still vote independently. The 48-hour window provides enough time for validator review while being meaningfully faster than the standard cycle. The 0.85 threshold is deliberately high — it should represent roughly the top 10-15% of active agents.

There is a risk that this creates pressure to inflate agent scores, but since M010 scores are computed from on-chain behavior (not self-reported), manipulation requires sustained genuine contribution. The cost of gaming exceeds the benefit of slightly faster governance.

**Recommendation:** Yes, agent scores at or above 0.85 enable a validator-only fast track with a 48-hour voting window. The fast track applies only to parameter changes and registry updates, not to software upgrades or treasury proposals.

**Rationale:** Improves governance responsiveness for low-risk changes while preserving full deliberation for high-impact decisions. The high score threshold limits access to proven agents.

**Status:** RESOLVED

---

## M009: Escrow and Milestone Management

### OQ-M009-1: Automatic Reputation Feedback

**Question:** Should successful escrow completion automatically generate a reputation signal in M010, or should reputation updates require a separate explicit action?

**Analysis:**

Manual reputation signaling creates friction and inconsistency. If a milestone is completed, the escrow released, and both parties satisfied, requiring a separate reputation transaction adds cost and complexity with no informational benefit. The escrow outcome itself is the most reliable signal of performance.

The 30-day delay before the reputation signal takes effect serves two purposes. First, it provides a window for dispute initiation if problems emerge after delivery. Second, it prevents rapid reputation farming through high-frequency low-value escrows. A party that completes ten trivial escrows in a day does not receive an immediate reputation boost — the signals queue and apply gradually.

Negative signals (dispute outcomes, missed deadlines) should apply immediately since the harm from delayed negative feedback is greater than the harm from premature negative feedback. The asymmetry is intentional: trust should be slow to build and fast to lose.

The automatic mechanism also ensures completeness. In manual systems, positive feedback is systematically under-reported because satisfied parties have less motivation to act than dissatisfied ones. Automation corrects this bias.

**Recommendation:** Yes, implement automatic M010 reputation signal on escrow completion with a 30-day delay for positive signals. Negative signals (dispute loss, deadline miss) apply immediately.

**Rationale:** Reduces friction, corrects reporting bias, and maintains a cooling-off period for disputes.

**Status:** RESOLVED

---

### OQ-M009-2: Partial Milestones

**Question:** Should M009 support partial milestone completion and proportional escrow release in v0?

**Analysis:**

Partial milestones add significant complexity to the escrow module. They require defining what "partial" means for each milestone type, implementing proportional release calculations, handling disputes over completion percentages, and managing the accounting for partially released funds. Each of these is a non-trivial implementation challenge.

The v0 escrow module benefits from simplicity. Binary milestones (complete or not complete) are easier to implement, test, audit, and reason about. They also produce cleaner reputation signals — a milestone is either done or it is not.

That said, partial milestones are a legitimate need for larger, longer-duration agreements. A six-month development contract with a single binary milestone creates excessive risk for both parties. The solution is to encourage more granular milestone definition in v0 (many small milestones rather than few large ones) and add partial completion support in v1 when the module has been battle-tested.

The v1 implementation can learn from v0 usage patterns to design partial completion in a way that matches actual needs rather than theoretical ones.

**Recommendation:** No partial milestones in v0. Encourage granular milestone decomposition as a workaround. Add partial milestone support in v1 based on observed usage patterns.

**Rationale:** Keeps v0 simple and auditable. Real-world usage data will inform a better v1 design than speculative requirements.

**Status:** RESOLVED

---

### OQ-M009-3: Validator Override for Stuck Disputes

**Question:** Should validators have the ability to override or force-resolve disputes that are stuck (neither party responding)?

**Analysis:**

Stuck disputes represent a real operational risk. If neither party engages with the dispute process, escrowed funds remain locked indefinitely, which harms both parties and reduces system liquidity. A mechanism for resolution is necessary.

However, validator override introduces a subjective judgment that validators are not well-positioned to make. Validators are infrastructure operators, not arbitrators. Giving them override authority creates liability concerns, potential conflicts of interest (validators may have relationships with disputing parties), and an expectation of arbitration expertise that the validator role does not require.

The better mechanism is automatic escalation. Each dispute has a `resolution_deadline` parameter. If the deadline passes without resolution, the dispute automatically escalates to a governance proposal. This approach uses existing governance infrastructure, distributes the decision across the full governance body rather than individual validators, and creates a public record of the resolution.

For truly abandoned disputes (neither party engaging and no governance interest), a final fallback of proportional return (50/50 split of escrowed funds) after a governance-extended deadline provides closure without requiring any party to actively adjudicate.

**Recommendation:** No validator override. Rely on the existing `resolution_deadline` parameter for automatic escalation to governance. Add a 50/50 proportional return fallback for disputes that remain unresolved after governance escalation.

**Rationale:** Avoids creating an arbitration role that validators are not equipped for. Leverages existing governance infrastructure for dispute resolution.

**Status:** RESOLVED

---

### OQ-M009-4: Escrow and M013 Interaction

**Question:** How should platform fees from escrow transactions interact with the M013 fee distribution model?

**Analysis:**

Escrow transactions generate platform fees at two points: escrow creation (deposit) and escrow release (withdrawal). These fees are functionally identical to other transaction fees on the network and should follow the same distribution logic.

Creating a separate fee distribution path for escrow fees would fragment the fee revenue system, complicate accounting, and create arbitrage opportunities (users might route transactions through or around escrow to optimize fee treatment). Uniform treatment eliminates these issues.

The only consideration is timing. Escrow fees at creation time are straightforward. Fees at release time should be calculated on the released amount, not the original deposit, to account for any partial releases or dispute-related adjustments. This is a minor implementation detail, not a design question.

**Recommendation:** Platform fees from escrow transactions follow the standard M013 distribution model. Fees at release time are calculated on the actual released amount.

**Rationale:** Uniform fee treatment prevents fragmentation, simplifies accounting, and eliminates arbitrage opportunities.

**Status:** RESOLVED

---

## M011: Curation and Quality Scoring

### OQ-M011-1: Curator Token Holdings Requirement

**Question:** Should curators be required to hold a minimum amount of REGEN tokens, or is the curator bond sufficient alignment?

**Analysis:**

The curator bond already serves as a financial commitment mechanism. It ensures curators have skin in the game and can be slashed for malicious or negligent curation. Adding a separate holdings requirement on top of the bond creates a redundant alignment mechanism that raises the barrier to entry without proportional benefit.

High holdings requirements would exclude capable curators who have expertise but limited capital. The curation role should select for domain knowledge and analytical skill, not for wealth. The bond mechanism already filters out uncommitted participants — anyone willing to lock up the bond amount has demonstrated sufficient financial commitment.

There is an argument that token holdings align curators with long-term network value. However, the bond itself is denominated in REGEN and subject to slashing, which creates the same alignment. A curator whose bond is at risk from poor curation is already motivated to maintain network health.

If the community wants additional alignment, a more effective approach would be to require that bonds be staked (earning staking rewards) rather than idle, which aligns curators with validator set health without requiring additional capital.

**Recommendation:** No minimum holdings requirement. The curator bond is sufficient for alignment. Consider allowing bond staking in a future upgrade for additional alignment.

**Rationale:** Avoids wealth-based gatekeeping of the curator role. The bond mechanism already provides adequate financial commitment and slashing exposure.

**Status:** RESOLVED

---

### OQ-M011-2: Quality Scores On-Chain vs KOI

**Question:** Should detailed quality scores be stored fully on-chain, or should only a hash be anchored on-chain with full details in the KOI (Knowledge Object Index) layer?

**Analysis:**

Full on-chain storage of quality scores is technically feasible but economically wasteful. Quality scores include metadata, methodology references, confidence intervals, and supporting data that can consume significant block space. At scale (thousands of credits across hundreds of classes), the storage cost becomes material.

Anchoring a hash on-chain provides the same auditability guarantee as full storage. Anyone can verify that the off-chain data matches the on-chain hash. The hash is immutable and timestamped, so the integrity of the quality score is provable without storing the full data on the ledger.

The KOI layer is designed for exactly this type of structured data storage. It provides queryability, versioning, and rich metadata support that the blockchain layer intentionally does not optimize for. Storing quality scores in KOI leverages its strengths while using the chain for what the chain does best: immutable commitment.

The concern about KOI availability is valid — if KOI goes offline, the scores are temporarily inaccessible. However, the hash on-chain ensures that when KOI returns, the data can be verified. For critical path operations (like escrow release conditioned on quality scores), the relevant score can be included in the transaction as a witness, verified against the on-chain hash.

**Recommendation:** Anchor the quality score hash on-chain. Store full quality score details in the KOI layer. Include score witnesses in transactions that depend on quality scores for critical-path verification.

**Rationale:** Cost-effective storage strategy that preserves auditability. Leverages KOI's strengths for rich data while using the chain for commitment.

**Status:** RESOLVED

---

### OQ-M011-3: Curation Fee and M013 Interaction

**Question:** Should the curation fee be carved from the existing M013 trade fee, or should it be a separate fee?

**Analysis:**

Carving the curation fee from the M013 trade fee would reduce the revenue available for the core fee distribution (burn, validators, community, agents). Since M013 distribution ratios are already contentious (see OQ-M013-1), further reducing the pool would intensify disagreements and complicate an already difficult negotiation.

A separate curation fee is cleaner. It is paid by the party requesting curation (typically the credit issuer or marketplace listing entity) and represents payment for a specific service (quality assessment). This is fundamentally different from transaction fees, which are a cost of using the network. Conflating the two creates confusion about what each fee pays for.

The separate fee also allows market-driven pricing. Curators can compete on fee levels, and credit issuers can choose curators based on cost, quality, and reputation. This market dynamic is impossible if curation is funded from a fixed carve-out of trade fees.

Implementation-wise, a separate fee is simpler. It does not require modifying M013 parameters or recalculating distribution ratios. It is a direct payment from the requesting party to the curator, with a small protocol fee (going through M013) on the payment itself.

**Recommendation:** Implement curation as a separate fee paid by the requesting party, not carved from the M013 trade fee. The curation fee payment itself is subject to standard M013 protocol fees.

**Rationale:** Preserves M013 distribution integrity, enables market-driven curation pricing, and maintains clean separation of concerns between network fees and service fees.

**Status:** RESOLVED

---

### OQ-M011-4: Curator Reputation Tracking

**Question:** Should curator performance be tracked through the M010 reputation system?

**Analysis:**

Curators perform a specialized function that directly affects market quality. Their assessments influence trading decisions, escrow conditions, and credit class credibility. Tracking their performance through M010 provides a natural feedback loop: good curators build reputation, which attracts more curation requests, which generates more fee revenue.

The M010 system already supports categorical reputation tracking. Adding a "curator" category is a minor extension that leverages existing infrastructure. The inputs for curator reputation are well-defined: accuracy of quality predictions (did the credit perform as the score suggested?), timeliness of assessments, consistency across similar credits, and challenge outcomes (how often are their scores contested and overturned?).

Without reputation tracking, there is no systematic way to distinguish good curators from poor ones. The bond mechanism penalizes egregious failures but does not differentiate between adequate and excellent curation. Reputation fills this gap and enables quality-based curator selection.

The only concern is that reputation tracking might discourage curators from scoring marginal credits honestly (a low score might be contested, risking reputation even if accurate). This is mitigated by weighting challenge outcomes — a score that is contested but upheld should boost rather than harm reputation.

**Recommendation:** Yes, track curator performance via M010 with a dedicated "curator" category. Weight challenge outcomes so that upheld scores boost reputation. Include accuracy, timeliness, and consistency as scoring inputs.

**Rationale:** Creates a quality differentiation mechanism for curators that the bond system alone cannot provide. Leverages existing M010 infrastructure.

**Status:** RESOLVED

---

### OQ-M011-5: Basket Token Quality Scores

**Question:** When a basket token contains multiple credit classes, should the quality score apply to the basket as a whole or to each constituent credit?

**Analysis:**

Baskets are designed as diversified instruments. Their value proposition is aggregation — a basket of credits across multiple methodologies and geographies provides risk diversification that individual credits do not. The quality score should reflect this aggregated nature.

Scoring individual constituents and then computing a basket score creates several problems. First, it requires quality assessment of every constituent, which may not be feasible for large baskets. Second, it raises the question of how to aggregate (simple average? weighted? worst-case?), which introduces a design decision that different users would answer differently. Third, it creates an illusion of precision — a basket score derived from constituent scores implies a level of analytical depth that may not exist.

A basket-level score assesses the basket as an instrument: its diversification, the quality of its construction methodology, its historical performance, and its alignment with stated objectives. This is what a buyer of basket tokens actually needs to know. If a buyer wants constituent-level detail, they should purchase individual credits rather than baskets.

The basket-level score can reference constituent quality where known (as supporting evidence) without being mechanically derived from it. This gives curators analytical flexibility while providing users with the actionable information they need.

**Recommendation:** Quality scores apply to the basket as a whole, assessing construction methodology, diversification, and basket-level performance. Constituent scores may inform but do not mechanically determine the basket score.

**Rationale:** Matches the information needs of basket token buyers. Avoids intractable aggregation methodology debates. Preserves curator analytical flexibility.

**Status:** RESOLVED

---

## M012: Burn Mechanism and Supply Management

### OQ-M012-1: Hard Cap Value

**Question:** What should the REGEN token hard cap be?

**Analysis:**

The current circulating supply is approximately 224 million REGEN. Setting the hard cap below this figure creates immediate scarcity pressure — the network would need to burn tokens to reach a sustainable equilibrium, which reinforces the deflationary mechanism's credibility.

A hard cap of 221,000,000 REGEN (approximately 1.3% below current supply) is aggressive enough to signal commitment to deflation but not so aggressive that it creates a supply crisis. The gap between current supply and the cap defines the magnitude of initial burn pressure, and a 3 million token gap is achievable through normal fee-driven burns over 12-18 months without extraordinary measures.

Setting the cap above current supply (e.g., 250M) would undermine the deflationary narrative entirely, since no burns would be necessary to reach the cap. Setting it dramatically below (e.g., 200M) would require burning 24M tokens, which at current fee volumes could take years and might necessitate disruptive one-time burns.

The 221M figure balances credibility with achievability. However, this is a fundamentally political decision that affects every token holder's economic position. It cannot be resolved by technical analysis alone and requires a governance vote with full community participation.

**Recommendation:** Set the hard cap at 221,000,000 REGEN, creating immediate but manageable scarcity pressure. This requires a governance vote given its direct economic impact on all token holders.

**Rationale:** Below-current-supply cap demonstrates deflationary commitment. The 3M token gap is achievable through organic burns within 12-18 months.

**Status:** NEEDS_GOVERNANCE

---

### OQ-M012-2: Ecological Multiplier Oracle

**Question:** Should the ecological impact multiplier use an oracle, and if so, what data source?

**Analysis:**

The ecological multiplier adjusts burn rates based on the network's ecological impact — higher verified impact could reduce burn rates (rewarding productive use) or increase them (accelerating deflation during high-impact periods). The data source for this multiplier is critical to its integrity.

In v0, the multiplier should be disabled (set to 1.0x, neutral). The M008 attestation system that would feed the oracle is not yet mature enough to provide reliable aggregate data. Launching with a premature oracle creates a manipulation surface: if the multiplier can be influenced by strategic attestation behavior, it will be.

In v1, the M008 attestation aggregate data provides a natural oracle source. By v1, the attestation system will have months of operational data, established patterns, and known attack vectors. The multiplier can be calibrated against this historical data rather than speculative models.

The v1 oracle should compute the multiplier from a rolling 30-day aggregate of verified attestations, weighted by credit class diversity and geographic distribution. This makes the multiplier reflect genuine ecological breadth rather than volume in a single credit class.

**Recommendation:** Disable the ecological multiplier in v0 (set to 1.0x). In v1, implement using M008 attestation aggregate data with a 30-day rolling window, weighted by credit class diversity and geographic distribution.

**Rationale:** Avoids premature oracle risk in v0. Uses the most credible on-chain data source (M008 attestations) when sufficient history exists.

**Status:** RESOLVED

---

### OQ-M012-3: Burn Period Length

**Question:** Should burns occur per-block or on a periodic epoch basis, and if periodic, what length?

**Analysis:**

Per-block burns create continuous, fine-grained deflation but add computational overhead to every block. They also make burn analytics noisy — tracking burn rates requires aggregation over arbitrary windows, and small per-block amounts are difficult to communicate meaningfully to the community.

Epoch-based burns (accumulating fees and burning periodically) are cleaner operationally and communicatively. A weekly epoch (7 days) provides frequent enough burns to maintain deflationary pressure while being long enough to produce meaningful, reportable burn amounts.

Shorter epochs (daily) create too much overhead for marginal benefit. Longer epochs (monthly) delay the deflationary signal and accumulate larger burn pools that could become governance targets ("redirect this month's burn to X instead"). Weekly strikes the right balance.

The accumulated-then-burned model also provides a natural circuit breaker. If an anomalous week produces an unusually large burn pool (due to a spike in trading activity, for example), governance can intervene before the burn executes. Per-block burns offer no such opportunity.

**Recommendation:** 7-day (weekly) burn epochs. Fees accumulate in a burn pool during the epoch and are burned in a single transaction at epoch end.

**Rationale:** Weekly epochs balance burn frequency with operational clarity. Accumulated burns provide a natural governance circuit breaker for anomalous periods.

**Status:** RESOLVED

---

### OQ-M012-4: Permanent Burn vs Reserve

**Question:** Should burned tokens be permanently destroyed or moved to a reserve that could theoretically be reactivated?

**Analysis:**

A reserve mechanism fundamentally undermines the deflationary credibility of the burn. If token holders know that burned tokens could theoretically be reactivated, the burn provides no credible supply reduction. Markets would price REGEN based on total supply (including reserve) rather than circulating supply, negating the economic benefit of burning.

Permanent burn is irreversible by design. This irreversibility is the source of its credibility. When the network burns 100,000 REGEN, every participant knows those tokens are gone forever. This certainty is what allows markets to reprice the remaining supply upward.

The concern about burning too aggressively is valid but is better addressed through burn rate parameters (the multiplier, the distribution percentages, the epoch length) than through reversibility. If the community determines that burn rates are too high, it can reduce the burn percentage through governance. This is a forward-looking adjustment, not a backward-looking reversal.

Reserve mechanisms also create governance attack surfaces. A reserve of millions of tokens becomes a target for proposals to "unlock the reserve for X worthy cause." Each such proposal, even if defeated, consumes governance attention and creates uncertainty.

**Recommendation:** Permanent burn. Burned tokens are irrecoverably destroyed. Burn rate adjustments are made through forward-looking parameter changes, not retroactive reserve access.

**Rationale:** Irreversibility is the foundation of deflationary credibility. Reserve mechanisms create governance attack surfaces and undermine market confidence in supply reduction.

**Status:** RESOLVED

---

### OQ-M012-5: Which Multiplier Phase

**Question:** Which burn rate multiplier should be active at launch, and how should multiplier phases progress?

**Analysis:**

The burn mechanism supports multiple multiplier phases, each optimizing for a different network objective: staking participation (reward stakers with lower effective burn), maximum deflation (highest possible burn rate), and supply stability (adaptive burn targeting a specific supply level).

At launch, the staking multiplier is most appropriate. The network needs to incentivize staking participation to secure the validator set under the emerging PoA model. A multiplier that reduces effective burn for staked tokens encourages delegation and strengthens network security during the critical early period.

After the staking ratio stabilizes (target: 60%+ of circulating supply staked), the network transitions to the maximum deflation phase. This phase applies the highest burn rates to aggressively reduce supply, capitalizing on the security provided by strong staking participation.

The stability phase activates once supply reaches a governance-defined target (e.g., 200M REGEN). In this phase, the multiplier adapts dynamically to maintain supply near the target, increasing burns when supply exceeds the target and reducing burns when supply approaches or drops below it.

Phase transitions should be triggered by on-chain conditions (staking ratio thresholds, supply levels) rather than calendar dates, ensuring that transitions reflect actual network state.

**Recommendation:** Phase-gated progression: staking multiplier at launch, maximum deflation multiplier when staking ratio exceeds 60%, stability multiplier when supply reaches governance-defined target. Transitions triggered by on-chain conditions.

**Rationale:** Each phase addresses the network's most pressing need at that stage. On-chain triggers ensure transitions reflect reality rather than arbitrary timelines.

**Status:** RESOLVED

---

## M013: Fee Distribution

### OQ-M013-1: Model A vs Model B

**Question:** Which fee distribution model should be adopted — Model A (higher burn) or Model B (higher community allocation)?

**Analysis:**

Model A prioritizes deflation through a higher burn percentage, while Model B prioritizes ecosystem development through a larger community allocation. Both models have strong advocates, and the debate reflects a genuine tension between capital appreciation (burn) and ecosystem growth (community funding).

The compromise position — 28% burn, 25% validator, 45% community, 2% agent — attempts to split the difference. The 28% burn is lower than Model A's proposal but still meaningful for deflation. The 45% community allocation is lower than Model B's proposal but represents the largest single allocation, signaling that ecosystem development is the primary use of fees. The 25% validator allocation maintains strong validator incentives. The 2% agent allocation seeds the agent infrastructure fund without consuming significant fee revenue.

This compromise will not fully satisfy either camp, which is arguably the sign of a reasonable middle ground. The key insight is that the distribution can be adjusted through governance as the network matures. Starting with a balanced allocation and adjusting based on observed outcomes is lower-risk than committing strongly to either extreme.

The decision affects every participant's economic incentives and cannot be resolved by technical analysis alone. A governance vote with robust community deliberation is essential.

**Recommendation:** Adopt the compromise distribution: 28% burn, 25% validator, 45% community, 2% agent infrastructure. Subject to governance vote and adjustable through subsequent governance proposals.

**Rationale:** Balanced allocation that does not over-index on any single objective. Adjustable through governance as network needs evolve.

**Status:** NEEDS_GOVERNANCE

---

### OQ-M013-2: Credit Value Determination

**Question:** How should the REGEN-denominated value of ecological credits be determined for fee calculation purposes?

**Analysis:**

Fee calculations require a reference price for ecological credits denominated in REGEN. This price must be resistant to manipulation (a single large trade should not dramatically alter fee calculations), timely (reflecting current market conditions), and available for all traded credit classes.

A 7-day Time-Weighted Average Price (TWAP) from the on-chain marketplace provides all three properties. The 7-day window smooths out volatility and manipulation attempts. The time-weighting ensures that sustained price levels have more influence than transient spikes. On-chain calculation means the price is verifiable and deterministic.

For credit classes that have not yet traded on the marketplace (new classes, rarely traded classes), a governance fallback is necessary. A governance-set reference price, updated quarterly or on-demand through governance proposals, provides a human-judgment backstop for assets that lack sufficient market data.

The TWAP calculation should use the same 7-day epoch as the burn mechanism (OQ-M012-3) for consistency. The TWAP is computed at epoch boundaries and applies to the subsequent epoch, ensuring that all participants operate with the same reference price throughout the epoch.

**Recommendation:** Use 7-day TWAP from the on-chain marketplace for actively traded credit classes. Governance-set reference prices for untraded or thinly traded classes, updated quarterly at minimum. TWAP computed at epoch boundaries aligned with the burn epoch.

**Rationale:** TWAP provides manipulation-resistant, timely pricing. Governance fallback covers edge cases. Epoch alignment simplifies implementation and reasoning.

**Status:** RESOLVED

---

### OQ-M013-3: Fee Denomination

**Question:** Should fees be collected and distributed in REGEN, in the transaction denomination, or in some hybrid approach?

**Analysis:**

Collecting fees exclusively in REGEN would require all transactors to hold REGEN, which creates friction for participants who primarily deal in other denominations (USDC, ATOM, etc.). This friction could suppress transaction volume, which reduces total fee revenue — a self-defeating outcome.

Collecting fees exclusively in the transaction denomination simplifies the user experience but creates complexity for distribution. The burn mechanism requires REGEN (you cannot burn USDC), and validators may prefer receiving fees in a stablecoin rather than a volatile asset.

The hybrid approach threads this needle. Fees are collected in whatever denomination the transaction uses, preserving user experience. The burn portion is auto-converted to REGEN via the on-chain DEX (or IBC swap) before burning, ensuring the burn mechanism always operates on REGEN. The remaining distribution (validators, community, agents) is distributed in the collected denomination, giving recipients the asset they would most naturally want.

The auto-conversion introduces slippage risk for the burn portion. This can be mitigated by using the epoch-based burn accumulation (OQ-M012-3): conversion happens once per epoch in a single batch, which is more efficient than per-transaction conversion and allows the use of limit orders or TWAP-based execution.

This is a design decision with significant implementation implications and user experience impact. It should be validated through a governance vote to ensure community alignment on the tradeoffs.

**Recommendation:** Hybrid approach — collect fees in transaction denomination, auto-convert the burn portion to REGEN at epoch boundaries, distribute the remainder in the collected denomination. Batch conversion at epoch boundaries to minimize slippage.

**Rationale:** Preserves user experience while ensuring the burn mechanism operates in REGEN. Batch conversion reduces slippage and operational complexity.

**Status:** NEEDS_GOVERNANCE

---

### OQ-M013-4: Agent Infrastructure Fund Governance

**Question:** How should the agent infrastructure fund (2% of fee distribution) be governed?

**Analysis:**

The agent infrastructure fund is small in absolute terms (2% of fees) but important symbolically — it represents the network's commitment to AI agent development. Its governance must be both efficient (small amounts should not require heavyweight governance) and accountable (public funds must be tracked).

A separate module account provides clean accounting. The fund accumulates in its own account, and disbursements are tracked separately from the Community Pool. This separation makes it easy to audit how agent infrastructure funds are being used.

In Stage 1 (pre-PoA maturity), a multisig of 3-of-5 trusted community members can manage disbursements efficiently. The multisig members should include at least one validator, one agent developer, and one community representative. Disbursement decisions can be made quickly without full governance proposals for amounts below a threshold (e.g., 10,000 REGEN).

In Stage 2+ (post-PoA maturity), the fund transitions to full governance control. Disbursements require governance proposals, and the multisig is dissolved. This transition mirrors the broader network maturation from trusted bootstrapping to decentralized governance.

**Recommendation:** Separate module account for the agent infrastructure fund. Multisig governance (3-of-5) in Stage 1 with expedited disbursement for amounts under 10,000 REGEN. Full governance control in Stage 2+.

**Rationale:** Staged governance matches the fund's maturity to the network's maturity. Separate accounting ensures transparency. Multisig provides efficiency during bootstrapping.

**Status:** RESOLVED

---

### OQ-M013-5: Burn Pool Existence and Size

**Question:** Should a burn pool exist, and if so, at what percentage of fee distribution?

**Analysis:**

The burn pool question intersects with the broader tension between capital formation (burning tokens to increase scarcity) and contributor compensation (directing funds to people building the ecosystem). Both are legitimate priorities, and the original burn pool proposal at 28-35% reflected a strong deflationary stance.

A reduced burn pool of 15% represents a meaningful compromise. It maintains the deflationary mechanism's existence and credibility — tokens are still being permanently burned — while redirecting 13-20% (compared to original proposals) toward contributors. This redirection can fund development grants, curation rewards, attestation incentives, and other activities that directly grow the ecosystem.

The argument for a higher burn pool (stronger deflation, higher token price) assumes that token price appreciation is the primary mechanism for attracting contributors. The argument for a lower burn pool (more direct funding) assumes that direct compensation is more effective. In practice, both mechanisms matter, and a 15% burn pool maintains both without over-indexing on either.

This is one of the most consequential economic decisions for the network. The burn percentage directly determines the long-term supply trajectory and the funding available for ecosystem development. It requires robust community deliberation and a governance vote.

**Recommendation:** Yes, maintain a burn pool, reduced to 15% of fee distribution. Redirect the difference (relative to original proposals) to contributor-facing allocations within the community pool. Subject to governance vote.

**Rationale:** Preserves deflationary credibility while shifting emphasis toward direct ecosystem funding. A compromise between capital formation and contributor-first philosophies.

**Status:** NEEDS_GOVERNANCE

---

## M014: Proof-of-Authority Validator Set

### OQ-M014-1: Validator Set Size

**Question:** How large should the initial PoA validator set be?

**Analysis:**

The validator set size balances decentralization against coordination efficiency. A larger set provides more decentralization and fault tolerance but requires more coordination overhead and dilutes individual validator rewards. A smaller set is more efficient but concentrates authority.

Starting at 15 validators provides sufficient Byzantine fault tolerance (tolerating up to 4 Byzantine validators under standard BFT assumptions) while keeping the set small enough for effective coordination. Fifteen validators can realistically participate in synchronous governance discussions, respond to emergencies collectively, and maintain personal accountability.

Growing to 21 organically (as qualified applicants emerge) provides a clear expansion path without committing to a fixed timeline. The "qualified applicants emerge" criterion is important — expanding the set with unqualified validators to hit a number target would degrade network quality. Each new validator should meet the PoA criteria established in M014.

The growth from 15 to 21 should be governed by a standing governance proposal mechanism: when a new applicant meets criteria, a streamlined proposal admits them. This avoids both the bottleneck of full governance proposals for each validator and the risk of unvetted additions.

**Recommendation:** Start at 15 validators. Grow to 21 organically as qualified applicants emerge. Use a streamlined governance proposal mechanism for validator set expansion.

**Rationale:** Fifteen provides adequate BFT tolerance and coordination efficiency. Organic growth ensures quality is maintained. Streamlined proposals reduce admission friction.

**Status:** RESOLVED

---

### OQ-M014-2: Performance Bonus

**Question:** Should a performance bonus from the validator fund be allocated, and if so, at what percentage?

**Analysis:**

Validator performance varies. Some validators maintain near-perfect uptime, participate actively in governance, and contribute to network development. Others meet minimum requirements but do not exceed them. A performance bonus rewards the former and incentivizes all validators to improve.

A 10% allocation from the validator fund (i.e., 10% of the 25% validator allocation from M013) is meaningful enough to motivate performance improvement without being so large that it creates destructive competition. At 10%, the bonus supplements rather than dominates validator economics.

The performance metrics should be objective and on-chain where possible: uptime percentage, governance participation rate, attestation accuracy (if the validator participates in M008), and block production consistency. Subjective metrics (community contribution, ecosystem development) are harder to measure fairly and should be excluded from the automatic bonus calculation, though they can be recognized through separate community programs.

The bonus should be distributed at the end of each burn epoch (weekly), using the same epoch boundaries as other periodic mechanisms for consistency. This provides regular, timely feedback on performance.

**Recommendation:** Yes, maintain the 10% performance bonus from the validator fund. Base it on objective, on-chain metrics: uptime, governance participation, and block production consistency. Distribute weekly at epoch boundaries.

**Rationale:** Simple, meaningful incentive that rewards operational excellence. Objective metrics prevent gaming and favoritism. Weekly distribution provides timely feedback.

**Status:** RESOLVED

---

### OQ-M014-3: Initial Trusted Partners

**Question:** How should the initial set of PoA validators be selected?

**Analysis:**

The initial validator set bootstraps the PoA system. The selection process must balance speed (the network needs to launch) with legitimacy (the initial set shapes the network's character). A purely top-down selection risks accusations of centralization, while a purely bottom-up process could take months.

The pragmatic approach is to start with the current active validators who already meet PoA criteria. These validators have demonstrated commitment to the network through sustained operation, and their track records are publicly verifiable. They represent the path of least resistance for bootstrapping.

However, not all current validators will meet PoA criteria (which may include ecological mission alignment, organizational requirements, or operational standards beyond basic uptime). The selection must be filtered through the criteria, not grandfathered automatically.

A governance vote on the seed set provides legitimacy. The vote does not assess individual validators in isolation but approves the set as a whole, with the option to object to specific inclusions. This approach combines efficiency (one vote for the whole set) with accountability (any validator can be challenged).

This decision has significant implications for who controls the network during its formative period and requires community input.

**Recommendation:** Bootstrap the initial validator set from current active validators who meet PoA criteria. Submit the proposed seed set for a governance vote. Allow individual objections during the voting period.

**Rationale:** Leverages existing operational track records while providing community legitimacy through governance approval.

**Status:** NEEDS_GOVERNANCE

---

### OQ-M014-4: Timeline

**Question:** What is the target timeline for PoA activation?

**Analysis:**

The timeline must account for testnet validation, mainnet preparation, validator onboarding, and community education. Rushing the timeline risks deploying an undertested system; delaying it prolongs the uncertainty about the network's governance structure.

Q3 2026 for testnet pilot provides approximately 3-4 months from now (March 2026) for specification finalization, implementation, and initial testing. This is ambitious but achievable given the current state of the specifications. The testnet pilot should run for a minimum of 8 weeks to expose operational issues.

Q4 2026 for mainnet activation aligns with the Economic Reboot Roadmap and provides time to incorporate testnet learnings. The activation should be gated on testnet success criteria (no critical bugs, validator set stability, governance mechanism validation) rather than a fixed date.

The parallel PoS/PoA period (see OQ-GOV-POA-2) begins at mainnet activation and provides a safety net during the transition. This means the effective "hard deadline" for full PoA is 6-12 months after mainnet activation (Q2-Q4 2027), which provides ample time for course correction.

**Recommendation:** Q3 2026 testnet pilot (minimum 8-week duration). Q4 2026 mainnet activation, gated on testnet success criteria. Timeline aligned with Economic Reboot Roadmap.

**Rationale:** Ambitious but achievable timeline with built-in gates. Alignment with Economic Reboot Roadmap ensures coordination with other network initiatives.

**Status:** RESOLVED

---

### OQ-M014-5: Impact on Delegated REGEN

**Question:** How should the transition to PoA affect existing delegated REGEN?

**Analysis:**

The transition from PoS to PoA fundamentally changes the role of delegation. Under PoS, delegation is a consensus mechanism — delegated tokens contribute to network security. Under PoA, validators are authorized by reputation and criteria, not by stake weight. Delegation may continue for reward distribution purposes but loses its consensus function.

This change has significant implications for delegators. They need sufficient notice to make informed decisions about their staked tokens. The 90-day advance notice period provides three months for delegators to evaluate their options, consult with their validators, and plan any changes to their delegation strategy.

The mandatory 21-day unbonding period remains in effect during the transition. This is the existing Cosmos SDK parameter and should not be shortened (which would reduce security during the vulnerable transition period) or lengthened (which would unfairly restrict delegator flexibility during a period of change).

Communication must be comprehensive. On-chain governance announcements reach active governance participants, but many delegators are passive holders who do not monitor governance. Email notifications (where registered), social media announcements, validator-mediated communication, and ecosystem partner announcements should all be employed. The goal is to ensure that no delegator is surprised by the transition.

**Recommendation:** 90-day advance notice before PoA activation. Mandatory 21-day unbonding period unchanged. Communication via all available channels: on-chain governance, social media, validator announcements, ecosystem partner notifications.

**Rationale:** Respects delegators' economic interests with ample notice and unchanged unbonding terms. Multi-channel communication minimizes the risk of delegators being caught unaware.

**Status:** RESOLVED

---

## M015: Facilitation Fee Distribution

### OQ-M015-1: 6% Sustainability

**Question:** Is the 6% facilitation fee sustainable across the projected range of committed capital?

**Analysis:**

The 6% facilitation fee generates revenue proportional to the committed capital it facilitates. At $50,000 per month in fee revenue (which corresponds to approximately $10M in annual facilitated transactions at a 6% rate), the fee supports a meaningful facilitation infrastructure. The question is whether this scales.

At $3M committed capital with $50K/month fee revenue, the 6% rate is sustainable for a lean facilitation operation (2-3 full-time equivalents plus infrastructure). Above $3M committed, the 6% generates surplus that can fund expanded services. Below $3M, the fee may not cover operational costs, requiring supplementary funding from the community pool.

The $3M threshold is achievable based on current network activity projections. The Regen Marketplace handles significant credit transaction volume, and a 6% facilitation fee on even a fraction of this volume exceeds the $3M threshold.

The rate should be a governance-adjustable parameter rather than a hard-coded constant. If network activity grows significantly, the rate could be reduced (e.g., to 4%) while maintaining the same absolute revenue. If activity is lower than projected, the rate could be temporarily increased or supplemented. Starting at 6% and adjusting based on observed outcomes is the most pragmatic approach.

**Recommendation:** 6% is sustainable up to approximately $3M committed capital at $50K/month fee revenue. Start at 6% with governance ability to adjust. Monitor quarterly and propose adjustments if revenue falls below operational cost thresholds or significantly exceeds them.

**Rationale:** 6% provides a viable starting point with built-in governance flexibility. Quarterly monitoring ensures the rate stays appropriate as network activity evolves.

**Status:** RESOLVED

---

### OQ-M015-2: Facilitation Identification

**Question:** How should facilitation activity be identified on-chain for fee distribution purposes?

**Analysis:**

Facilitation fees must be attributed to the facilitating entity to be distributed correctly. This requires a reliable on-chain identification mechanism that is both inclusive (captures all legitimate facilitation) and resistant to false claims.

Transaction metadata via the memo field is the simplest approach. Facilitators include their identifier in the transaction memo, and the fee distribution module reads this metadata. The Cosmos SDK memo field is already available and does not require module modifications to use.

The limitation of memo-based identification is that it relies on the transacting party to include the memo. A registered dApp address fallback addresses this: if a transaction originates from a registered facilitator address (registered through a governance proposal or a facilitation registry), the fee is automatically attributed even without a memo. This covers the case where a dApp facilitates transactions programmatically and cannot rely on users to include memos.

The combination of memo-based and address-based identification provides comprehensive coverage. The address registry is the authoritative source; the memo is a convenience mechanism for ad-hoc facilitation that does not originate from a registered address.

**Recommendation:** Primary identification via transaction metadata (memo field) with the facilitator's registered identifier. Fallback identification via registered dApp address matching. Address registration through a facilitation registry maintained by governance.

**Rationale:** Dual identification mechanism ensures comprehensive coverage. Memo-based identification is flexible; address-based identification is automatic. Registry provides an authoritative source of truth.

**Status:** RESOLVED

---

### OQ-M015-3: Community Pool Split

**Question:** What percentage of the Community Pool should be automatically distributed via M015 vs. directed by governance?

**Analysis:**

The Community Pool serves two functions: funding ongoing network operations (which benefits from predictable, automatic distribution) and funding discretionary initiatives (which benefits from governance deliberation). The split between these functions determines how much operational certainty the network provides versus how much flexibility governance retains.

A 70/30 split (70% automatic M015 distribution, 30% governance-directed) prioritizes operational continuity. The 70% automatic allocation ensures that facilitation rewards, contributor compensation, and other M015-eligible activities receive reliable funding without requiring repeated governance proposals. This predictability is essential for attracting and retaining contributors who need income stability.

The 30% governance-directed portion preserves community agency over a meaningful portion of the pool. This is sufficient for grants, emergency funding, strategic initiatives, and other discretionary spending. It is not so large that the governance process becomes a bottleneck for routine operations.

The split could reasonably be 60/40 or 80/20 — there is no analytically "correct" answer. The 70/30 split reflects a judgment that operational predictability is slightly more important than governance flexibility at this stage of network development, while preserving enough governance-directed funds to be meaningful.

This split directly affects community expectations and contributor compensation predictability. It requires a governance vote to ensure community buy-in.

**Recommendation:** 70% automatic M015 distribution, 30% governance-directed. Both percentages adjustable through governance proposals. Review annually.

**Rationale:** Prioritizes operational predictability for contributor retention while preserving meaningful governance discretion. Annual review ensures the split adapts to changing needs.

**Status:** NEEDS_GOVERNANCE

---

### OQ-M015-4: Anti-Gaming Measures

**Question:** What mechanisms should prevent gaming of the facilitation fee distribution?

**Analysis:**

Gaming the facilitation fee system involves creating artificial transactions to claim facilitation fees without providing genuine facilitation value. The primary defense is that M013 fees are levied on all transactions — a gamer must pay transaction fees to claim facilitation fees, which limits the profit margin of gaming to the difference between the transaction cost and the facilitation reward.

However, if facilitation rewards exceed transaction costs, gaming becomes profitable. Additional measures are needed to close this gap.

A minimum transaction value of 1 REGEN ensures that dust transactions (which cost minimal fees but could claim facilitation rewards) are not eligible for facilitation attribution. This is a simple, effective floor that eliminates the most trivial gaming vector.

Address correlation analysis by AGENT-003 (the network's anti-gaming agent) provides a more sophisticated defense. AGENT-003 monitors transaction patterns for signs of wash trading: rapid round-trips between related addresses, unusual transaction patterns, and volume spikes from new addresses. When AGENT-003 flags suspicious activity, the facilitation attribution is suspended pending review.

The combination of economic disincentives (M013 fees), minimum transaction values, and AI-assisted monitoring creates a layered defense. No single measure is sufficient, but together they make gaming unprofitable and detectable.

**Recommendation:** M013 transaction fees as primary economic disincentive. Minimum 1 REGEN transaction value for facilitation eligibility. AGENT-003 address correlation analysis for pattern-based detection of wash trading. Flagged transactions have facilitation attribution suspended pending review.

**Rationale:** Layered defense combining economic disincentives, simple rules, and AI-assisted monitoring. Each layer addresses a different attack vector.

**Status:** RESOLVED

---

## GOV-POA: Governance and Proof-of-Authority Transition

### OQ-GOV-POA-1: Per-Process Weights

**Question:** Should governance tally weights vary by process type, and if so, what should the weights be for each type?

**Analysis:**

This question extends OQ-M001-1 with specific per-process weight recommendations. The principle established in OQ-M001-1 (weights should match decision type) requires concrete numbers for each process category.

Software upgrades (70/30 validator/holder) reflect that upgrades require technical assessment of code changes, security implications, and operational impact. Validators bear the operational burden of upgrades and have the deepest understanding of their implications. A 70/30 split gives validators strong but not absolute authority over technical decisions.

Treasury proposals (50/50) affect all stakeholders equally. The treasury belongs to the community, and decisions about its use should reflect broad community preferences rather than validator technical judgment. Equal weights ensure that neither validators nor holders can unilaterally direct treasury spending.

Registry changes (60/40) — adding or modifying credit classes, methodologies, and other registry entries — require moderate domain expertise. The default 60/40 split provides a slight validator advantage without marginalizing holder input.

Parameter changes (60/40) — adjusting module parameters like fee rates, epoch lengths, and thresholds — are technical but have broad economic impact. The default 60/40 split balances these considerations.

The per-process weight table is a governance-level decision that affects power distribution across the network. It requires community deliberation and a governance vote.

**Recommendation:** Per-process weight table: Software upgrades 70/30, Treasury proposals 50/50, Registry changes 60/40, Parameter changes 60/40. Default 60/40 for uncategorized proposals. Adjustable through governance.

**Rationale:** Matches decision authority to decision type. Gives validators stronger voice on technical matters, balanced voice on economic matters, and equal voice on treasury matters.

**Status:** NEEDS_GOVERNANCE

---

### OQ-GOV-POA-2: Parallel PoS/PoA Duration

**Question:** How long should the PoS and PoA governance systems run in parallel during the transition?

**Analysis:**

The parallel period serves as a safety net. If PoA governance exhibits unexpected failures — validator collusion, governance capture, insufficient participation — the PoS system provides a fallback. The parallel period must be long enough to stress-test PoA governance through multiple governance cycles and at least one contentious proposal.

A 6-month minimum ensures that PoA governance experiences at least 24 weekly epochs, multiple governance proposals, and likely at least one contentious vote. Six months also spans seasonal variation in network activity, ensuring that the PoA system is tested under different load conditions.

A 12-month maximum prevents the parallel period from becoming a permanent state. Running two governance systems indefinitely creates confusion about authority, increases operational overhead, and delays the benefits of full PoA transition. The 12-month cap forces a decision: either PoA is working and PoS is deprecated, or PoA is not working and the transition is reconsidered.

Within the 6-12 month window, the governance community decides when to end the parallel period based on observed PoA performance. A governance proposal to end the parallel period requires passage under both the PoS and PoA systems, ensuring mutual consent.

**Recommendation:** 6-month minimum, 12-month maximum parallel operation. Transition to full PoA requires a governance proposal passing under both PoS and PoA systems. If PoA criteria are not met after 12 months, a mandatory review and extension or reversion proposal is triggered.

**Rationale:** Minimum provides sufficient stress-testing; maximum prevents indefinite parallelism. Dual-system passage requirement ensures mutual consent for the transition.

**Status:** RESOLVED

---

### OQ-GOV-POA-3: Existing Validator Treatment

**Question:** How should existing PoS validators be treated in the PoA transition?

**Analysis:**

Existing validators have invested time, capital, and effort in operating Regen Network infrastructure. The transition to PoA should respect this investment while not automatically grandfathering validators who may not meet PoA criteria.

Under PoA, the validator role changes from "anyone who stakes enough" to "authorized parties who meet specific criteria." Existing validators who do not meet PoA criteria should not be excluded from the network — they transition to the token holder track and participate in governance as holders rather than validators.

Operational experience from running a PoS validator is genuinely valuable for PoA applications. It demonstrates technical capability, commitment to the network, and familiarity with Cosmos SDK infrastructure. PoA applications should explicitly value this experience, creating a natural advantage for existing validators who choose to apply.

However, operational experience alone should not guarantee PoA inclusion. PoA criteria include ecological mission alignment, organizational accountability, and other factors that go beyond technical operation. An existing validator who meets all criteria has a strong application; one who does not should be encouraged to address the gaps rather than be automatically included.

The messaging around this transition is critical. Existing validators should not feel rejected or devalued. They should understand that their experience is an asset in the PoA application process and that the holder track provides meaningful governance participation.

**Recommendation:** Existing PoS validators transition to the token holder track in PoA governance. Operational experience is explicitly valued in PoA validator applications. Existing validators who meet PoA criteria are encouraged to apply and have a natural advantage.

**Rationale:** Respects existing validators' contributions without creating automatic grandfathering that could compromise PoA criteria. Clear path from PoS experience to PoA application.

**Status:** RESOLVED

---

## Summary Table

| ID | Module | Question Summary | Status |
|----|--------|-----------------|--------|
| OQ-M001-1 | M001-ENH | 60/40 validator-to-holder weight split | RESOLVED |
| OQ-M001-2 | M001-ENH | Agent score influence on governance track | RESOLVED |
| OQ-M009-1 | M009 | Auto reputation feedback | RESOLVED |
| OQ-M009-2 | M009 | Partial milestones | RESOLVED |
| OQ-M009-3 | M009 | Validator override for stuck disputes | RESOLVED |
| OQ-M009-4 | M009 | Escrow + M013 interaction | RESOLVED |
| OQ-M011-1 | M011 | Curator holdings requirement | RESOLVED |
| OQ-M011-2 | M011 | Quality scores on-chain vs KOI | RESOLVED |
| OQ-M011-3 | M011 | Curation fee + M013 | RESOLVED |
| OQ-M011-4 | M011 | Curator reputation tracking | RESOLVED |
| OQ-M011-5 | M011 | Basket token quality scores | RESOLVED |
| OQ-M012-1 | M012 | Hard cap value | NEEDS_GOVERNANCE |
| OQ-M012-2 | M012 | Ecological multiplier oracle | RESOLVED |
| OQ-M012-3 | M012 | Period length | RESOLVED |
| OQ-M012-4 | M012 | Permanent burn vs reserve | RESOLVED |
| OQ-M012-5 | M012 | Which multiplier phase | RESOLVED |
| OQ-M013-1 | M013 | Model A vs B fee distribution | NEEDS_GOVERNANCE |
| OQ-M013-2 | M013 | Credit value determination | RESOLVED |
| OQ-M013-3 | M013 | Fee denomination | NEEDS_GOVERNANCE |
| OQ-M013-4 | M013 | Agent infra fund governance | RESOLVED |
| OQ-M013-5 | M013 | Burn pool existence and size | NEEDS_GOVERNANCE |
| OQ-M014-1 | M014 | Validator set size | RESOLVED |
| OQ-M014-2 | M014 | Performance bonus | RESOLVED |
| OQ-M014-3 | M014 | Initial trusted partners | NEEDS_GOVERNANCE |
| OQ-M014-4 | M014 | Timeline | RESOLVED |
| OQ-M014-5 | M014 | Delegated REGEN impact | RESOLVED |
| OQ-M015-1 | M015 | 6% sustainability | RESOLVED |
| OQ-M015-2 | M015 | Facilitation identification | RESOLVED |
| OQ-M015-3 | M015 | Community Pool split | NEEDS_GOVERNANCE |
| OQ-M015-4 | M015 | Anti-gaming measures | RESOLVED |
| OQ-GOV-POA-1 | GOV-POA | Per-process weights | NEEDS_GOVERNANCE |
| OQ-GOV-POA-2 | GOV-POA | Parallel PoS/PoA duration | RESOLVED |
| OQ-GOV-POA-3 | GOV-POA | Existing validator treatment | RESOLVED |

**Totals:** 26 RESOLVED, 7 NEEDS_GOVERNANCE

---

## Cross-Reference: OQ Assumptions in Other PRs

Several PRs in this repository assume specific OQ resolutions as baselines. If the WG resolves any NEEDS_GOVERNANCE item differently than assumed here, the following PRs will need parameter updates:

| OQ | Assumed Value | Assumed In | Impact if Changed |
|----|--------------|-----------|-------------------|
| OQ-M012-1 | 221M hard cap | PR #54 (simulation), PR #49 (governance proposals) | Recalculate equilibrium analysis; update proposal text |
| OQ-M013-1 | {28/25/45/2} compromise | PR #49 (governance proposals) | Update proposal parameters; re-run simulation |
| OQ-M013-1 | {30/40/25/5} Model A | PR #54 (simulation baseline) | Simulation covers full range via sweep; conclusions robust |
| OQ-M013-3 | Hybrid denomination | PR #49 (Proposal 1 text) | Update fee collection implementation architecture |
| OQ-M013-5 | Burn exists (15-30%) | PR #54 (simulation), PR #49 | If burn eliminated, supply model changes fundamentally |
| OQ-M014-3 | Bootstrap from current validators | PR #45 (selection rubric) | Update seed set selection process |
| OQ-M015-3 | 70/30 auto/governance | PR #49 (Proposal 4), PR #54 | Update reward distribution parameters |
| OQ-GOV-POA-1 | Per-process variation | PR #45 (rubric), PR #49 | Update tally logic in proposals |

> **Action item**: When any NEEDS_GOVERNANCE item is resolved via community vote, update the affected PRs to reflect the actual governance decision. The simulation (PR #54) should be re-run with the decided parameters to confirm sustainability.

---

## Next Steps for the Working Group

1. **Governance Proposals (7 items):** The seven NEEDS_GOVERNANCE items should be packaged into governance proposals for community deliberation. We recommend grouping them into three proposals:
   - **Economic Parameters Proposal:** OQ-M012-1 (hard cap), OQ-M013-1 (fee distribution model), OQ-M013-3 (fee denomination), OQ-M013-5 (burn pool size)
   - **Validator and Governance Structure Proposal:** OQ-M014-3 (initial trusted partners), OQ-GOV-POA-1 (per-process weights)
   - **Community Pool and Operations Proposal:** OQ-M015-3 (Community Pool split)

2. **Implementation Specifications:** The 22 RESOLVED items should be incorporated into their respective module specifications as concrete parameters. Implementation teams can proceed with these values without waiting for governance action on the remaining 9 items.

3. **Testnet Validation:** Per OQ-M014-4, target Q3 2026 for testnet deployment. The testnet should exercise all RESOLVED parameters and use placeholder values for NEEDS_GOVERNANCE items until governance votes conclude.

4. **Community Education:** Before the governance votes, publish accessible summaries of each NEEDS_GOVERNANCE item with the tradeoffs involved. The goal is informed voting, not advocacy for specific outcomes.

5. **Quarterly Review Cycle:** After mainnet activation, review all parameter values quarterly. The RESOLVED items are not immutable — they can be adjusted through governance if operational experience reveals issues.
