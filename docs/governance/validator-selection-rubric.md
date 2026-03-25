# Validator Selection Rubric

> Operationalizes PR #34 (Bioregional Validator Framework) into a concrete, scoreable rubric for the Regen Network authority validator set.

**Status:** Draft
**Spec dependency:** M014 (phase-2/2.6) — Curated Authority Validator Set
**PR origin:** Gregory's PR #34 — Bioregional Validator Framework
**Last updated:** 2026-03-24

> **Architecture note**: This rubric operationalizes M014's validator selection process. M014 defines the _mechanism_ (set size, lifecycle states, compensation model); this rubric defines the _scoring framework_ used to evaluate applicants. The rubric's parameters (set size 15-21, category minimums 5/5/5, term length 12 months) are drawn from M014 §Validator Composition and §Governance Parameters. If M014 parameters change via governance, this rubric must be updated to match.

---

## Overview

The bioregional validator framework (PR #34) establishes that Regen Network validators represent ecosystems and communities, not just servers. Validators are stewards of bioregional trust -- their legitimacy derives from ecological knowledge, institutional commitment, and geographic rootedness as much as from technical uptime.

This rubric translates that vision into a repeatable, transparent scoring framework that validator governance can apply consistently across all three validator categories. Every applicant is scored on a 1000-point scale against category-specific criteria. The rubric is designed to be machine-assistable (AGENT-004 can compute preliminary scores from on-chain and KOI data) while preserving human judgment for qualitative dimensions like mission alignment and bioregional engagement.

### Design Principles

1. **Transparency.** Every score component is defined, weighted, and published. Applicants can self-assess before applying.
2. **Bioregional primacy.** Geographic and ecological diversity are first-class criteria, not tiebreakers.
3. **Category fidelity.** Infrastructure Builders, Trusted ReFi Partners, and Ecological Data Stewards are evaluated on what makes each role distinct.
4. **Anti-entrenchment.** Term limits, rotation schedules, and rebalancing prevent validator capture.
5. **Conflict hygiene.** Undisclosed conflicts are disqualifying -- full stop.

### Validator Set Composition (from M014)

| Parameter | Value |
|---|---|
| Total validators | 15--21 curated |
| Term length | 12 months |
| Categories | `infrastructure_builder`, `refi_partner`, `data_steward` |
| Minimum per category | 5 |
| Performance baseline | 99.5% uptime, governance participation, ecosystem contribution |
| Application mechanism | `MsgApplyValidator` with `evidence_iri` |

---

## Scoring Framework

**Total score: 1000 points.**
**Minimum eligibility threshold: 600 points.**

Each validator category has seven criteria weighted to reflect the priorities of that role. The criteria names are shared across categories, but their definitions and point allocations differ.

---

### Infrastructure Builders (minimum 5 seats)

Infrastructure Builders operate the physical and software infrastructure that keeps the chain running. They are evaluated primarily on technical reliability and code contributions, with meaningful weight given to mission alignment and bioregional context.

| # | Criterion | Max Points | Weight |
|---|---|---|---|
| IB-1 | Technical Infrastructure | 200 | 20% |
| IB-2 | Code Contributions | 200 | 20% |
| IB-3 | Mission Alignment | 150 | 15% |
| IB-4 | Institutional Stability | 150 | 15% |
| IB-5 | Geographic Diversity | 100 | 10% |
| IB-6 | Bioregional Engagement | 100 | 10% |
| IB-7 | Conflict of Interest Disclosure | 100 | 10% |
| | **Total** | **1000** | **100%** |

#### IB-1: Technical Infrastructure (0--200)

Evaluates the applicant's operational capability to run a reliable, secure validator node.

| Score Range | Description |
|---|---|
| 160--200 | Proven node operator with 12+ months of continuous mainnet operation on Regen or comparable Cosmos SDK chain. Documented 99.9%+ uptime. HSM or equivalent key management. Tested disaster recovery plan with documented recovery time objective (RTO) under 4 hours. Sentry node architecture. Monitoring and alerting in place. |
| 120--159 | Experienced operator with 6--12 months mainnet history. 99.5%+ uptime. Hardware key management. Disaster recovery plan exists but not fully tested. Basic monitoring. |
| 80--119 | Testnet experience or limited mainnet history (under 6 months). Reasonable infrastructure described but limited track record. Software key management with documented rotation policy. |
| 40--79 | Minimal operational history. Infrastructure plan provided but untested. Key management practices unclear or basic. |
| 0--39 | No demonstrated node operation experience. Infrastructure description vague or absent. |

**Evidence required:** Node operation history (chain explorer links, uptime reports), infrastructure architecture diagram, key management policy document, disaster recovery runbook.

#### IB-2: Code Contributions (0--200)

Evaluates contributions to the Regen Ledger codebase and broader ecosystem tooling.

| Score Range | Description |
|---|---|
| 160--200 | Core contributor to `regen-ledger` or major ecosystem repository. 50+ merged PRs or equivalent. Authored or maintained critical modules. Active in code review. Developed and maintains ecosystem tools (explorers, wallets, indexers, SDKs). Comprehensive documentation contributions. |
| 120--159 | Regular contributor with 20--49 merged PRs. Meaningful feature work or bug fixes. Maintains at least one ecosystem tool. Documentation contributions. |
| 80--119 | Occasional contributor with 5--19 merged PRs. Bug fixes, test improvements, or documentation. Some tooling work. |
| 40--79 | Minor contributions (1--4 PRs). Issue reports. Small documentation fixes. |
| 0--39 | No code contributions to Regen ecosystem. Contributions to adjacent ecosystems (Cosmos SDK, IBC) may earn partial credit here. |

**Evidence required:** GitHub profile links, PR history, repository links for maintained tools, documentation contributions.

#### IB-3: Mission Alignment (0--150)

Evaluates the organization's commitment to regenerative ecological outcomes.

| Score Range | Description |
|---|---|
| 120--150 | Organization's primary mission is ecological regeneration, climate action, or environmental stewardship. Team members have backgrounds in ecology, sustainability, or regenerative economics. Published commitment to Regen Network's mission. Active participation in ReFi ecosystem governance. |
| 80--119 | Organization has ecological or sustainability goals alongside other objectives. Some team members with relevant backgrounds. Demonstrated interest in regenerative outcomes. |
| 40--79 | General-purpose infrastructure provider with stated interest in sustainability. Limited ecological background but genuine engagement. |
| 0--39 | No demonstrated mission alignment. Purely commercial motivation without ecological framing. |

**Evidence required:** Organizational mission statement, team bios, public statements or publications on ecological mission, governance participation history.

#### IB-4: Institutional Stability (0--150)

Evaluates the organization's ability to sustain validator operations over a 12-month term and beyond.

| Score Range | Description |
|---|---|
| 120--150 | 5+ years in operation. Diversified funding sources (grants, revenue, endowment). Team of 10+ with dedicated infrastructure staff. Established legal entity in a clear jurisdiction. Published financial reports or equivalent transparency. |
| 80--119 | 2--5 years in operation. Stable funding for 12+ months. Team of 5--9. Legal entity established. |
| 40--79 | 1--2 years in operation. Funding secured for current term. Team of 2--4. Legal structure in place or in process. |
| 0--39 | Under 1 year in operation. Funding uncertain. Solo operator or very small team. No formal legal structure. |

**Evidence required:** Organizational history, funding disclosures, team roster, legal entity documentation.

#### IB-5: Geographic Diversity (0--100)

Evaluates the applicant's contribution to geographic and bioregional diversity within the validator set.

| Score Range | Description |
|---|---|
| 80--100 | Located in or primarily serving a bioregion not currently represented in the active validator set. Infrastructure physically located in an underrepresented region. |
| 50--79 | Located in a bioregion with 1--2 existing validators. Provides geographic diversity within an already-represented continent or biome. |
| 25--49 | Located in a well-represented region but brings some geographic nuance (e.g., different country within a represented continent). |
| 0--24 | Located in same metropolitan area or narrow region as 3+ existing validators. No meaningful geographic diversity contribution. |

**Evidence required:** Primary operational location, infrastructure hosting locations, bioregional affiliation statement.

**Scoring note:** This criterion is relative to the current validator set composition. AGENT-004 recalculates geographic diversity scores whenever the set changes.

#### IB-6: Bioregional Engagement (0--100)

Evaluates the applicant's relationships with local ecological communities, indigenous groups, land stewards, and place-based organizations.

| Score Range | Description |
|---|---|
| 80--100 | Deep, documented partnerships with local ecological organizations, indigenous communities, or land management bodies. Active in bioregional governance or planning. Supports local ecological data collection or monitoring. Example: partnership with a watershed council, indigenous land trust, or regional conservation district. |
| 50--79 | Some local partnerships or community relationships. Participates in regional ecological initiatives. Emerging bioregional connections. |
| 25--49 | Awareness of bioregional context. Initial outreach to local organizations. Plans for engagement described but not yet realized. |
| 0--24 | No bioregional engagement. Operates as a geographically detached infrastructure provider. |

**Evidence required:** Partnership agreements or letters of support, community engagement reports, participation records in bioregional initiatives.

#### IB-7: Conflict of Interest Disclosure (0--100)

Evaluates transparency and independence. See [Conflict of Interest Rules](#conflict-of-interest-rules) for disqualifying conditions.

| Score Range | Description |
|---|---|
| 80--100 | Complete, proactive disclosure of all financial relationships, shared ownership, and potential conflicts. No material conflicts identified. Independent governance structure. |
| 50--79 | Adequate disclosure with minor gaps. Disclosed conflicts are manageable and mitigated. |
| 25--49 | Incomplete disclosure. Some conflicts identified but mitigation plan is weak. Requires follow-up. |
| 0--24 | Significant undisclosed conflicts discovered during review. Inadequate transparency. **Scores below 50 are disqualifying.** |

**Evidence required:** Conflict of interest disclosure form (template provided), ownership structure diagram, financial relationship disclosures.

---

### Trusted ReFi Partners (minimum 5 seats)

Trusted ReFi Partners are organizations actively building the regenerative finance ecosystem -- originating credits, operating marketplaces, facilitating ecological transactions, and connecting capital to regeneration. They are evaluated primarily on ecosystem activity and mission alignment.

| # | Criterion | Max Points | Weight |
|---|---|---|---|
| RP-1 | Ecosystem Activity | 200 | 20% |
| RP-2 | Mission Alignment | 200 | 20% |
| RP-3 | Community Engagement | 150 | 15% |
| RP-4 | Institutional Stability | 150 | 15% |
| RP-5 | Geographic Diversity | 100 | 10% |
| RP-6 | Bioregional Engagement | 100 | 10% |
| RP-7 | Conflict of Interest Disclosure | 100 | 10% |
| | **Total** | **1000** | **100%** |

#### RP-1: Ecosystem Activity (0--200)

Evaluates the applicant's active participation in regenerative finance transactions on Regen Network or interoperable chains.

| Score Range | Description |
|---|---|
| 160--200 | Major credit originator, marketplace operator, or platform facilitator with 12+ months of activity. High transaction volume (top quartile of ecosystem participants). Multiple credit classes or project types. Demonstrated facilitation of ecological outcomes at scale. |
| 120--159 | Active participant with 6--12 months of ecosystem activity. Moderate transaction volume. Involvement in multiple credit classes or projects. |
| 80--119 | Emerging participant with some on-chain activity. Early-stage credit origination or marketplace activity. 1--2 credit classes or projects. |
| 40--79 | Minimal on-chain activity. Plans for ecosystem participation described but largely unrealized. Testnet activity or pilot projects. |
| 0--39 | No demonstrated ecosystem activity on Regen Network or interoperable chains. |

**Evidence required:** On-chain transaction history (wallet addresses), credit origination records, marketplace activity reports, platform usage metrics.

#### RP-2: Mission Alignment (0--200)

Evaluates the organization's core commitment to regenerative finance and ecological outcomes.

| Score Range | Description |
|---|---|
| 160--200 | Organization exists to advance regenerative finance. Public track record of funding or facilitating ecological regeneration. Measurable ecological outcomes attributed to the organization's work. Leadership team with deep ReFi expertise. Published impact reports. |
| 120--159 | Strong ReFi focus with demonstrated ecological outcomes. Some measurable impact. Team with relevant expertise. |
| 80--119 | ReFi is a significant part of the organization's work alongside other activities. Some ecological outcomes documented. |
| 40--79 | General financial services or technology organization with ReFi as an emerging initiative. Limited ecological track record. |
| 0--39 | No demonstrated commitment to regenerative finance or ecological outcomes. |

**Evidence required:** Organizational mission and impact reports, ecological outcome documentation, team credentials, public track record.

#### RP-3: Community Engagement (0--150)

Evaluates participation in Regen Network governance, community building, and public discourse.

| Score Range | Description |
|---|---|
| 120--150 | Active governance participant: votes on proposals, submits proposals, participates in working groups. Regular forum contributor. Organizes community events, workshops, or educational content. Mentors new ecosystem participants. |
| 80--119 | Regular governance voter. Occasional forum contributor. Participates in community calls or events. Some community building activity. |
| 40--79 | Votes on major governance proposals. Occasional community participation. Passive but present. |
| 0--39 | Minimal or no governance participation. No community engagement beyond basic operations. |

**Evidence required:** Governance voting record, forum activity (username), event participation or organization, community building initiatives.

#### RP-4: Institutional Stability (0--150)

Same rubric structure as IB-4. Evaluates the organization's ability to sustain operations over a 12-month term.

| Score Range | Description |
|---|---|
| 120--150 | 5+ years in operation. Diversified revenue (credit sales, platform fees, grants). Team of 10+ with dedicated ReFi operations staff. Established legal entity. Financial transparency. |
| 80--119 | 2--5 years in operation. Stable funding. Team of 5--9. Legal entity established. |
| 40--79 | 1--2 years in operation. Funding secured for term. Team of 2--4. Legal structure in place. |
| 0--39 | Under 1 year. Funding uncertain. Very small team. No formal legal structure. |

**Evidence required:** Same as IB-4.

#### RP-5: Geographic Diversity (0--100)

Same rubric structure as IB-5. Evaluates contribution to bioregional diversity, with emphasis on the regions where the applicant facilitates ecological credit activity.

**Scoring note:** For ReFi Partners, "location" considers both organizational headquarters and the bioregions where credit activity occurs. A partner headquartered in Europe but facilitating credit origination in the Amazon basin scores based on both locations.

#### RP-6: Bioregional Engagement (0--100)

Evaluates place-based ecological programs and relationships with communities where regeneration occurs.

| Score Range | Description |
|---|---|
| 80--100 | Operates place-based ecological programs with measurable outcomes. Deep relationships with local communities, indigenous groups, or smallholder cooperatives in the bioregions where credits originate. Supports local capacity building. Example: the Berkshire Sweet Gold Cooperative model -- embedded in a specific landscape with direct community benefit. |
| 50--79 | Some place-based programming. Relationships with project developers in bioregions. Emerging community connections. |
| 25--49 | Awareness of bioregional context. Plans for place-based engagement. Initial relationships forming. |
| 0--24 | No place-based engagement. Operates as a purely intermediary platform without bioregional rootedness. |

**Evidence required:** Program descriptions, community partnership documentation, local impact assessments, letters of support from bioregional organizations.

#### RP-7: Conflict of Interest Disclosure (0--100)

Same rubric structure as IB-7. Particular attention to relationships with credit class admins and project developers, given the ReFi Partner's marketplace role.

---

### Ecological Data Stewards (minimum 5 seats)

Ecological Data Stewards bring scientific rigor and domain expertise to the validator set. They verify ecological claims, contribute to MRV (Measurement, Reporting, and Verification) methodologies, and ensure the integrity of on-chain ecological data. They are evaluated primarily on scientific credentials and data quality work.

| # | Criterion | Max Points | Weight |
|---|---|---|---|
| DS-1 | Scientific Credentials | 200 | 20% |
| DS-2 | Data Quality Track Record | 200 | 20% |
| DS-3 | Mission Alignment | 150 | 15% |
| DS-4 | Institutional Stability | 150 | 15% |
| DS-5 | Geographic Diversity | 100 | 10% |
| DS-6 | Bioregional Engagement | 100 | 10% |
| DS-7 | Conflict of Interest Disclosure | 100 | 10% |
| | **Total** | **1000** | **100%** |

#### DS-1: Scientific Credentials (0--200)

Evaluates domain expertise in ecology, land management, remote sensing, environmental science, or related fields.

| Score Range | Description |
|---|---|
| 160--200 | Organization includes researchers with advanced degrees (PhD or equivalent) in ecology, environmental science, or related fields. Peer-reviewed publications on ecological measurement, monitoring, or verification. Developed or contributed to recognized MRV methodologies. Active in scientific standards bodies (e.g., IPCC, Verra methodology development, Gold Standard TAC). |
| 120--159 | Team includes members with graduate-level expertise. Some publications or methodology contributions. Participation in scientific review processes. |
| 80--119 | Team includes members with relevant professional credentials (e.g., certified ecologists, GIS specialists, remote sensing analysts). Applied ecological expertise without extensive academic publication record. |
| 40--79 | General environmental knowledge. Some technical skill in data management or analysis. Limited formal ecological credentials. |
| 0--39 | No demonstrated ecological or scientific expertise. |

**Evidence required:** Team CVs/bios with credentials, publication lists, methodology contributions, standards body participation, professional certifications.

#### DS-2: Data Quality Track Record (0--200)

Evaluates the applicant's history of ecological data verification, attestation, and MRV contributions.

| Score Range | Description |
|---|---|
| 160--200 | Extensive verification work: 50+ ecological data attestations or verification reports. Contributions to Regen Network's data module or MRV frameworks. Demonstrated ability to identify data quality issues. Maintains or contributes to ecological data standards. Published data quality methodologies. |
| 120--159 | Significant verification history: 20--49 attestations or reports. Contributions to data standards or MRV frameworks. Some published methodologies. |
| 80--119 | Moderate verification experience: 5--19 attestations or reports. Familiarity with Regen data module. Participation in MRV pilot programs. |
| 40--79 | Limited verification experience. General data quality skills applicable to ecological data. 1--4 attestations or participation in verification training. |
| 0--39 | No demonstrated ecological data verification experience. |

**Evidence required:** Attestation history (on-chain references), verification reports, MRV methodology contributions, data quality publications, participation in verification programs.

#### DS-3: Mission Alignment (0--150)

Evaluates the organization's ecological mission and commitment to data integrity.

| Score Range | Description |
|---|---|
| 120--150 | Organization's primary mission is ecological research, conservation, or environmental monitoring. Strong commitment to open data and scientific integrity. Active participation in ecological data commons. Example: the Amazon Sacred Headwaters Alliance model -- mission is inseparable from ecological stewardship. |
| 80--119 | Ecological mission is central alongside other activities. Commitment to data quality and transparency. |
| 40--79 | Environmental data is one of several organizational activities. General commitment to scientific rigor. |
| 0--39 | No demonstrated ecological mission. Data work is purely commercial without ecological framing. |

**Evidence required:** Organizational mission statement, ecological program descriptions, data sharing policies, participation in data commons.

#### DS-4: Institutional Stability (0--150)

Same rubric structure as IB-4, with attention to research funding stability.

| Score Range | Description |
|---|---|
| 120--150 | 5+ years in operation. Diversified funding (research grants, government contracts, foundation support, earned revenue). Team of 10+ with dedicated research/data staff. Established legal entity. Track record of long-term research programs. |
| 80--119 | 2--5 years in operation. Multi-year funding secured. Team of 5--9. Legal entity established. |
| 40--79 | 1--2 years in operation. Current funding stable. Team of 2--4. Legal structure in place. |
| 0--39 | Under 1 year. Funding uncertain or project-based only. Very small team. No formal legal structure. |

**Evidence required:** Same as IB-4, plus research funding documentation.

#### DS-5: Geographic Diversity (0--100)

Same rubric structure as IB-5, with emphasis on unique ecosystem and biome representation.

**Scoring note:** For Data Stewards, "location" prioritizes the ecosystems and biomes where the organization conducts ecological monitoring and research. A data steward specializing in mangrove ecosystems scores differently from one specializing in temperate forests, even if both are headquartered in the same city.

#### DS-6: Bioregional Engagement (0--100)

Evaluates local ecological knowledge, community science programs, and relationships with bioregional communities.

| Score Range | Description |
|---|---|
| 80--100 | Deep integration with local ecological knowledge systems. Community science programs that involve local residents in data collection. Partnerships with indigenous knowledge holders. Contributes to local ecological management decisions. Example: ReFi Mediterranean model -- scientific work embedded in a specific bioregional context with community participation. |
| 50--79 | Some community science activity. Relationships with local ecological organizations. Data shared with bioregional stakeholders. |
| 25--49 | Awareness of local ecological context. Plans for community engagement in data collection. Initial outreach to bioregional organizations. |
| 0--24 | No bioregional engagement in data work. Operates as a remote data analysis organization without place-based relationships. |

**Evidence required:** Community science program descriptions, indigenous knowledge partnerships, local engagement documentation, data sharing agreements with bioregional organizations.

#### DS-7: Conflict of Interest Disclosure (0--100)

Same rubric structure as IB-7. Particular attention to independence from credit issuers, project developers, and entities whose ecological claims the Data Steward may be asked to verify.

| Score Range | Description |
|---|---|
| 80--100 | Complete disclosure. Organizationally independent from credit issuers and project developers. No financial relationships that could compromise verification objectivity. Clear separation between data stewardship and commercial interests. |
| 50--79 | Adequate disclosure. Minor relationships with credit ecosystem participants, but effective mitigation (e.g., recusal policies, independent review boards). |
| 25--49 | Incomplete disclosure. Financial relationships with credit issuers or developers that raise objectivity concerns. Mitigation plan required. |
| 0--24 | Significant undisclosed relationships with entities whose data the steward would verify. **Scores below 50 are disqualifying.** |

---

## Minimum Thresholds

All applicants must meet the following minimum thresholds to be eligible for a validator seat.

> **Dual-gate eligibility**: Applicants must pass ALL THREE gates below. These are independent — passing one does not waive the others. An applicant scoring 750/1000 overall but 40/200 on a single criterion is **not eligible** (fails Gate 2). An applicant scoring 650/1000 with all per-criterion minimums met but 45/100 on Conflict of Interest is **not eligible** (fails Gate 3).

### Gate 1: Overall Threshold

- **Minimum total score: 600/1000.**
- Applicants scoring below 600 are not eligible regardless of category composition needs.

### Gate 2: Per-Criterion Minimums

No single criterion may fall below **25% of its maximum point value**:

| Criterion Max | Minimum Required |
|---|---|
| 200 | 50 |
| 150 | 37.5 (round to 38) |
| 100 | 25 |

An applicant with a total score of 750 but a Technical Infrastructure score of 40/200 is **not eligible** because 40 < 50 (25% of 200).

### Gate 3: Conflict of Interest Hard Floor

- **Conflict of Interest Disclosure must score at least 50/100.**
- Any score below 50 on the Conflict of Interest criterion is an automatic disqualification, regardless of overall score.
- Material undisclosed conflicts discovered at any point are grounds for immediate disqualification or removal.

---

## Conflict of Interest Rules

These rules apply to all three validator categories. Violations are disqualifying.

### Mandatory Disclosures

All applicants must disclose:

1. **Financial relationships** with credit class admins, project developers, marketplace operators, or other active authority validators.
2. **Shared beneficial ownership** above 10% with any other entity in the Regen Network ecosystem.
3. **Board memberships, advisory roles, or consulting relationships** with entities that originate, verify, or trade ecological credits on Regen Network.
4. **Pending or anticipated financial relationships** that could create conflicts during the 12-month term.

### Structural Prohibitions

The following conditions are disqualifying:

1. **Common control.** An applicant cannot be owned or controlled (directly or indirectly) by an entity that owns or controls another active authority validator. "Control" means holding more than 50% of voting rights or the ability to appoint a majority of the governing body.
2. **Dual role prohibition.** An applicant cannot simultaneously serve as a validator AND operate as an arbiter in M008/M009 dispute resolution processes. Organizations performing both roles must choose one or maintain strict organizational separation with independent governance.
3. **Shared beneficial ownership concentration.** If an individual or entity holds more than 10% beneficial ownership in two or more validator applicants or active validators, all affected entities must disclose this relationship and the governance committee must evaluate whether the concentration undermines set independence.

### Disclosure Failures

- **Pre-approval discovery:** Immediate disqualification from the current application cycle. The applicant may reapply in the next cycle with complete disclosures.
- **Post-approval discovery:** Grounds for removal proceedings. The governance committee initiates a removal proposal within 14 days of discovery. The validator is suspended from block production pending resolution.
- **Intentional concealment:** Permanent bar from the authority validator set. Intentional concealment is determined by governance committee vote (2/3 majority of active validators).

### Safe Harbor

Applicants who proactively disclose conflicts and propose reasonable mitigation measures (e.g., recusal policies, independent oversight, structural separation) will not be penalized for the existence of the conflict itself. The rubric rewards transparency. Scoring reflects the quality of disclosure and mitigation, not the mere existence of relationships.

---

## Rotation and Rebalancing Schedule

### Annual Term Renewal

- Standard terms are **12 months**, starting from the governance proposal approval date.
- **Streamlined re-application** for incumbents scoring 700 or above on their most recent evaluation: abbreviated application form, AGENT-004 auto-populates on-chain performance data, 7-day community review (instead of 14 days).
- Incumbents scoring 600--699 must submit a full application.
- Incumbents scoring below 600 at mid-term review are not eligible for streamlined renewal.

### Mid-Term Review (6 Months)

- At the 6-month mark, AGENT-004 generates a performance report for each active validator.
- The report includes: uptime, governance participation rate, ecosystem contribution metrics, and any flagged incidents.
- **Automatic review trigger:** If a validator's composite on-chain performance score drops below the equivalent of 600/1000 on the original rubric, a formal review is initiated.
- The governance committee may issue: (a) a warning with a 30-day improvement period, (b) a conditional continuation with specific requirements, or (c) a removal proposal.

### Composition Rebalancing

- If any category drops to its **minimum of 5** validators (due to removal, resignation, or term expiration), the governance committee activates a **priority queue** for that category.
- Priority queue: applications for the underrepresented category are reviewed on an accelerated 7-day timeline.
- If the set drops below 15 total validators, emergency applications are accepted on a rolling basis until the minimum is restored.

### Geographic Diversity Review

- **Quarterly check** by AGENT-004: computes geographic concentration metrics for the active set.
- **Flag threshold:** If more than 3 validators have their primary operations in the same geographic region (defined as UN M49 sub-region), AGENT-004 flags the concentration for governance committee attention.
- The flag does not trigger automatic action but is factored into subsequent application scoring (Geographic Diversity criterion becomes relatively more valuable for underrepresented regions).

### Term Limits

- **Maximum 2 consecutive terms** without a 1-term gap.
- After serving 2 consecutive 12-month terms (24 months), a validator must sit out for at least 1 full term (12 months) before reapplying.
- This prevents entrenchment while allowing experienced validators to return after a cooling-off period.
- Exception: if a category would drop below its minimum of 5, the term limit is suspended for that category until a replacement is seated.

---

## Scoring Process

### Step 1: Application Submission

The applicant submits a transaction via `MsgApplyValidator` containing:

- **Validator category:** `infrastructure_builder`, `refi_partner`, or `data_steward`
- **evidence_iri:** An IRI (Internationalized Resource Identifier) pointing to the application package stored on-chain or in a content-addressed store. The package includes all evidence documents referenced in the rubric.
- **Operator details:** Node configuration, key management attestation, contact information.
- **Conflict of interest disclosure form:** Completed template covering all mandatory disclosures.

### Step 2: Automated Preliminary Scoring (Layer 1)

AGENT-004 computes a preliminary score based on verifiable on-chain and KOI (Knowledge, Oracle, and Indexer) data:

- **On-chain metrics:** Transaction history, governance voting record, attestation count, credit origination volume, uptime history (if previously active).
- **Code contributions:** GitHub API integration for commit history, PR counts, repository contributions.
- **Geographic data:** Self-reported location cross-referenced with node IP geolocation and organizational registration data.

AGENT-004 produces a preliminary score for each criterion where automated evidence is available. Criteria that require qualitative judgment (e.g., Mission Alignment, Bioregional Engagement) receive a placeholder score of "pending human review" rather than an automated estimate.

### Step 3: Community Discussion (14 days)

- The application and AGENT-004's preliminary score are published to the governance forum.
- Community members have 14 days to comment, ask questions, and flag concerns.
- The applicant is expected to respond to substantive questions during this period.
- Community sentiment is not formally scored but informs the governance committee's review.

### Step 4: Governance Committee Review (Layer 3)

The validator governance committee (composed of existing active validators) reviews each application:

- Evaluates qualitative criteria that AGENT-004 cannot score.
- Adjusts AGENT-004's automated scores where committee members have additional context.
- Considers community discussion feedback.
- Applies minimum threshold checks.
- Produces a final score with written justification for each criterion.

Scoring is conducted independently by each committee member, then averaged. Outlier scores (more than 2 standard deviations from the mean on any criterion) are discussed before final averaging.

### Step 5: Final Approval

- Applications meeting all thresholds are submitted as governance proposals.
- Active validators vote to approve or reject.
- Approval requires a simple majority of active validators.
- Approved validators are added to the active set in the next epoch.

---

## Worked Examples

The following examples illustrate how the rubric applies to realistic (but fictional) applicants. Scores are assigned by a hypothetical governance committee.

### Example 1: Infrastructure Builder -- Cascadia Node Collective

**Background:** A worker-owned cooperative in the Pacific Northwest that has operated Cosmos SDK validator nodes for 3 years. Deeply embedded in the bioregion, with partnerships with local watershed councils and the Cascadia Bioregion movement.

| Criterion | Score | Rationale |
|---|---|---|
| IB-1: Technical Infrastructure | 175/200 | 3 years continuous mainnet operation on Osmosis and Regen testnet. 99.95% uptime. HSM key management. Tested DR plan with 2-hour RTO. Sentry nodes in 3 locations. |
| IB-2: Code Contributions | 95/200 | 12 merged PRs to regen-ledger (mostly test improvements and documentation). Maintains a community block explorer. No core module work. |
| IB-3: Mission Alignment | 130/150 | Worker-owned cooperative with ecological mission in founding charter. Team includes two ecologists and a sustainability economist. Active in ReFi DAO governance. |
| IB-4: Institutional Stability | 105/150 | 3 years in operation. Funded by staking rewards and a Gitcoin grant. Team of 7. Registered cooperative in Oregon. Funding concentrated in staking rewards -- some risk if rewards change. |
| IB-5: Geographic Diversity | 85/100 | Pacific Northwest (Cascadia bioregion) not currently represented in active set. Nodes hosted in Portland and Vancouver BC. |
| IB-6: Bioregional Engagement | 90/100 | Formal partnership with Willamette Watershed Council. Hosts data for local salmon monitoring program. Active in Cascadia Bioregion movement governance. |
| IB-7: Conflict of Interest | 85/100 | Complete disclosure. No conflicts identified. Cooperative governance structure ensures independence. |
| **Total** | **765/1000** | **Eligible.** Strong technical and bioregional profile. Code contributions are the weakest area but above minimum threshold (95 > 50). |

### Example 2: Trusted ReFi Partner -- Cerrado Carbon Collective

**Background:** A Brazilian organization that facilitates carbon credit origination from cerrado biome restoration projects. Works directly with smallholder farming cooperatives. 18 months in operation.

| Criterion | Score | Rationale |
|---|---|---|
| RP-1: Ecosystem Activity | 145/200 | Active credit originator for 12 months. 15 credit batches originated across 3 credit classes. Moderate transaction volume (second quartile). Growing trajectory. |
| RP-2: Mission Alignment | 170/200 | Organization exists solely to channel finance to cerrado restoration. Published impact report documenting 2,400 hectares under restoration management. Team includes agroforestry experts and community organizers. |
| RP-3: Community Engagement | 85/150 | Votes on most governance proposals. Occasional forum posts. Organized one community workshop in Sao Paulo. Limited English-language engagement (language barrier, not disinterest). |
| RP-4: Institutional Stability | 55/150 | 18 months in operation. Funded by credit sales revenue and one foundation grant. Team of 4. Brazilian LLC established. Funding somewhat dependent on credit market conditions. |
| RP-5: Geographic Diversity | 95/100 | Cerrado biome (Brazil) not currently represented. Addresses significant gap in South American coverage. |
| RP-6: Bioregional Engagement | 95/100 | Works directly with 12 smallholder farming cooperatives in Goias state. Community-led restoration planning. Local employment and capacity building programs. |
| RP-7: Conflict of Interest | 70/100 | Adequate disclosure. Organization originates credits that would be validated on the chain it helps secure -- inherent tension disclosed with mitigation (recusal from credit class governance votes for classes they originate). |
| **Total** | **715/1000** | **Eligible.** Exceptional bioregional engagement and mission alignment. Institutional stability is the weakest area (55 > 38 minimum) -- committee notes this for mid-term monitoring. |

### Example 3: Ecological Data Steward -- Alpine Monitoring Institute

**Background:** A Swiss research institute specializing in alpine ecosystem monitoring. 15 years of operation. Extensive publication record. New to blockchain and Regen Network specifically.

| Criterion | Score | Rationale |
|---|---|---|
| DS-1: Scientific Credentials | 190/200 | Team includes 8 PhDs in ecology, glaciology, and remote sensing. 200+ peer-reviewed publications. Developed two methodologies adopted by Verra. Active in IPCC Working Group II. |
| DS-2: Data Quality Track Record | 60/200 | No on-chain attestation history. However, 15 years of ecological monitoring data with rigorous quality control. 3 pilot attestations on Regen testnet during application period. Score reflects absence of Regen-specific track record despite exceptional general data quality credentials. |
| DS-3: Mission Alignment | 140/150 | Institute's sole mission is alpine ecosystem research and conservation. Open data policy for all published datasets. Active contributor to European ecological data commons. |
| DS-4: Institutional Stability | 145/150 | 15 years in operation. Funded by Swiss National Science Foundation, EU Horizon grants, and cantonal government. Team of 35. Established foundation under Swiss law. |
| DS-5: Geographic Diversity | 70/100 | European Alps. One existing validator in broader European region but none in alpine ecosystems specifically. Brings unique biome representation. |
| DS-6: Bioregional Engagement | 75/100 | Extensive partnerships with Swiss Alpine Club, local canton environmental offices, and mountain farming communities. Community science program engages 200+ volunteer observers. Some engagement is scientific rather than governance-oriented. |
| DS-7: Conflict of Interest | 95/100 | Complete disclosure. Fully independent from credit issuers. No commercial ecological credit relationships. Research funding comes from public sources. |
| **Total** | **775/1000** | **Eligible.** Exceptional scientific credentials and institutional stability. Data Quality Track Record is the concern area (60 > 50 minimum) -- strong general credentials but limited on-chain history. Committee recommends close mid-term review of attestation activity. |

---

## Summary

### Criteria by Category

| Criterion | Infrastructure Builders | Trusted ReFi Partners | Ecological Data Stewards |
|---|---|---|---|
| Primary Criterion 1 | Technical Infrastructure (200) | Ecosystem Activity (200) | Scientific Credentials (200) |
| Primary Criterion 2 | Code Contributions (200) | Mission Alignment (200) | Data Quality Track Record (200) |
| Mission Alignment | 150 | -- (see above) | 150 |
| Institutional Stability | 150 | 150 | 150 |
| Geographic Diversity | 100 | 100 | 100 |
| Bioregional Engagement | 100 | 100 | 100 |
| Conflict of Interest Disclosure | 100 | 100 | 100 |
| Community Engagement | -- | 150 | -- |
| **Total** | **1000** | **1000** | **1000** |

### Threshold Summary

| Threshold | Value | Consequence |
|---|---|---|
| Overall minimum | 600/1000 | Below = ineligible |
| Per-criterion minimum | 25% of max | Below on any criterion = ineligible |
| Conflict of Interest hard floor | 50/100 | Below = automatic disqualification |
| Streamlined renewal threshold | 700/1000 | Above = abbreviated re-application |
| Mid-term review trigger | 600/1000 equivalent | Below = formal review initiated |
| Maximum consecutive terms | 2 | Must sit out 1 term after 2 consecutive |

### Process Summary

| Step | Duration | Actor |
|---|---|---|
| 1. Application via MsgApplyValidator | -- | Applicant |
| 2. AGENT-004 preliminary scoring | Automated | AGENT-004 (Layer 1) |
| 3. Community discussion | 14 days | Community |
| 4. Governance committee review | 7 days | Validator committee (Layer 3) |
| 5. Governance proposal vote | Per chain governance params | Active validators |

---

*This rubric is a living document. Amendments are proposed via standard governance proposals and require a simple majority of active validators to adopt. The rubric version is tracked in the governance module metadata.*
