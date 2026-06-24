# Spec → RFC → On-chain Proposal: Graduation Rules

This document defines the **lifecycle and promotion rules** for the three artifact types that govern the agentic-tokenomics → forum → on-chain pipeline. Together they form the deliberation chamber of the bicameral R&D pipeline.

Companion documents:
- [`_template.md`](./_template.md) — copyable RFC template
- [`schema/`](./schema/) — machine-readable governance review schema for agent + human reviewers

## The three artifact types

The agentic-tokenomics repo houses three distinct artifact types, with three distinct lifecycles. Each has a **purpose**, an **audience**, an **author scope** (who can write it), and a **decision scope** (what it commits the system to).

| | Spec | RFC | On-chain Proposal |
|---|---|---|---|
| **Purpose** | Explore a mechanism design — what could exist | Decide a question — what should exist | Execute a change — what now exists |
| **Audience** | Designers, contributors | Stewards, $REGEN stakers, partners | Validators, on-chain governance |
| **Author scope** | Anyone (humans + agents) | Humans + agents (with human sponsor) | Humans only (no agent-authored proposals to chain — for now) |
| **Decision scope** | Non-binding; specs can branch and proliferate | Binding within the repo; one ratified RFC per question | Binding on-chain; irreversible without superseding proposal |
| **Location** | `mechanisms/<id>/SPEC.md`, `phase-N/...` | `rfcs/RFC-XXXX.md` | `phase-4/proposals/RFC-XXXX-proposal.json` |
| **Lifecycle states** | draft → merged → superseded | draft → forum-posted → ratified → enacted (or parked / withdrawn) | drafted → submitted → voting → passed/rejected → enacted |
| **Versioning** | semver per spec; multiple drafts allowed | sequential RFC IDs; one canonical RFC per question; supersession rather than re-versioning | one proposal per attempt; if rejected, file a new one with new ID |

## Why three layers, not one?

Two failure modes this is designed to avoid:

1. **The "everything is a spec" problem.** When everything is a spec, nothing is decisive. Specs accumulate, branch, become impossible to navigate. There needs to be a graduation point where the system says "this is the canonical answer to this question, dispute it via supersession."

2. **The "specs go directly to chain" problem.** When specs become on-chain proposals without a deliberation step, the chain inherits the noise of design exploration. Validators get asked to vote on under-deliberated changes. Either they reject by default (chilling effect) or they rubber-stamp (loss of governance legitimacy).

The RFC layer is the **deliberation chamber**. It's where claims get sharpened, evidence gets challenged, and a human sponsor takes accountability before anything reaches chain.

## Promotion criteria

### Spec → RFC

A spec graduates to an RFC when **all** of the following are true:

1. **There is a decisive claim to be made.** The spec is no longer exploring "what could M013's fee split look like" — it is asserting "M013's fee split should be 15/30/50/5". If the work is still exploratory, it stays in `mechanisms/`.
2. **A human sponsor has volunteered.** No spec graduates without a named human accountable for shepherding the RFC to ratification or withdrawal.
3. **At least one decisive evidence item exists.** Per the RFC template's §2 (Sufficiency assertion), there must be at least one piece of evidence that, if invalidated, would invalidate the claim. Pure-design RFCs without empirical or simulation evidence are not allowed.
4. **The audience tier is declared, and the reference accessibility gate is passable.** If the RFC is targeted at `public`, every evidence URL must already resolve for a public reader, OR there must be a plan to elevate references before forum-posting.
5. **No live conflicting RFC.** If there's an unresolved RFC on the same question, that one must be ratified, withdrawn, or superseded first.

**Author scope:** anyone. **Decision authority:** the named sponsor (proposes promotion) + a steward confirms.

### RFC → forum-posted

An RFC graduates from `draft` to `forum-posted` when:

1. **The sponsor has prepared a forum post** with all references audience-accessible to the forum's reader population. (For commonwealth.im threads, that's a "public-or-commons" audience.)
2. **The discussion deadline is set** in the RFC's §5 — minimum 7 days, recommended 14, longer for high-stakes proposals.
3. **The RFC is linked from the forum post**, and the forum thread URL is added to the RFC.

**Author scope:** sponsor only. **Decision authority:** sponsor.

### RFC → ratified

An RFC graduates from `forum-posted` to `ratified` when:

1. **The discussion deadline has passed.**
2. **All key counter-positions raised in §5 have been addressed** — either incorporated into the RFC's claim/methodology, or explicitly noted as "considered and rejected because <reason>".
3. **All open questions in §8 are either resolved or explicitly carried as known limitations.**
4. **The decisive evidence in §2 has not been invalidated** during the discussion period.
5. **A ratifier (a steward distinct from the sponsor) signs off.** The ratifier's job is to confirm the deliberation was sufficient — not to re-litigate the claim. Two-eyes principle.

**Author scope:** sponsor + ratifier. **Decision authority:** ratifier.

### RFC → enacted

An RFC graduates from `ratified` to `enacted` when:

1. **An on-chain proposal generated from the RFC has passed** (or, for off-chain ratified RFCs that don't require chain action, when the corresponding code/process change is merged and live).
2. **The RFC's §7 (On-chain enactment) is filled out** with the actual proposal ID, voting result, and post-enactment notes.
3. **The lineage in §4 is updated** to reflect the realized outcome.

**Author scope:** sponsor or proposer. **Decision authority:** automatic once on-chain proposal passes.

### RFC → on-chain proposal

A `ratified` RFC produces an on-chain proposal when:

1. **A human proposer (typically the sponsor) generates the proposal artifact** from the RFC — using deterministic generation rules from §7 wherever possible.
2. **Pre-flight checks listed in §7 have all passed.** (Unit tests, integration tests, simulation reruns, etc.)
3. **The proposal references the RFC** by ID and KOI IRI in its on-chain `description` field.
4. **The proposer commits the deposit** and submits via `regen tx gov submit-proposal`.

**Critical rule:** **agents do not submit on-chain proposals.** They can draft the JSON, run pre-flight checks, and surface the proposal for human review — but the `submit-proposal` transaction must be signed by a human key. This is the accountability boundary.

## Demotion / parking / withdrawal

Some RFCs don't progress. The lifecycle accommodates this:

- **Parked.** The sponsor can move an RFC from `draft` or `forum-posted` to `parked` if it's blocked on an external dependency (e.g., waiting on M012 enactment before M013 makes sense). Parked RFCs retain their ID and can be revived later by the sponsor; the parking reason is logged in §6.
- **Withdrawn.** The sponsor can withdraw at any state before `enacted`. Withdrawn RFCs are not deleted; their ID is permanently retired. A new RFC on the same question gets a new ID.
- **Superseded.** A new RFC can supersede a `ratified` or `enacted` RFC. The new RFC must explicitly cite the superseded one in its header and explain the delta. The old RFC's status moves to `superseded`. (For `enacted` RFCs, supersession requires a new on-chain proposal.)

## Author roles: agents vs humans

This is where the accountability chain lives. Mapping each role to who can occupy it:

| Role | Codex agent | ElizaOS agent | Human contributor | Human steward |
|---|---|---|---|---|
| Spec author | yes | yes | yes | yes |
| RFC author | yes (with sponsor) | yes (with sponsor) | yes | yes |
| RFC sponsor | no | no | yes | yes |
| RFC ratifier | no | no | no | yes |
| Forum poster | drafts only — sponsor posts | drafts only — sponsor posts | yes | yes |
| On-chain proposer | no | no | yes | yes |

The pattern: **agents accelerate the cheap parts (drafting, evidence-gathering, JSON generation, pre-flight checks); humans hold the accountable parts (sponsorship, ratification, on-chain submission).**

## How this integrates with the rest of the system

- **KOI** indexes specs, RFCs, simulation runs, forum threads, and on-chain proposals under stable IRIs. The `koi:rfc:RFC-XXXX` namespace becomes the canonical reference for RFC artifacts. Cross-document lineage (per §4 of the template) is queryable as graph traversal.

- **Claims engine** uses the same 4-layer schema (claim, evidence, methodology, status) for ecological credits as the RFC template uses for governance. Same shape, different domain. This means tooling built for one chamber works on the other.

- **RegenOS** is the coordinator. It watches:
  - new RFCs (notifies stewards of incoming sponsorship requests)
  - RFCs at `forum-posted` reaching their discussion deadline (prompts ratifier)
  - RFCs at `ratified` (prompts proposer to generate on-chain proposal)
  - RFCs at `enacted` (closes the loop, updates lineage)

- **Reference accessibility gate** is the publication gate for any RFC graduating from `draft` to `forum-posted`. The audience tier in the RFC header drives the gate's strictness.

- **Codex agent** plugs in at the spec layer and at the RFC drafting layer. Its PRs that touch governance-relevant surface (specs, simulations, contract behavior) are flagged for RFC twin generation; its PRs that don't (CI, lint, tests, docs) merge directly. This is the filter that converts agent throughput from "merge volume" into "candidate proposals worth elevating."

## Governance Review Schema

The graduation criteria above are designed so an agent can mechanically judge whether an RFC meets them. The canonical review schema lives in [`schema/rfc-review-schema.yaml`](./schema/rfc-review-schema.yaml), with the corresponding judgment output shape in [`schema/rfc-review-judgment.yaml`](./schema/rfc-review-judgment.yaml). See [`schema/README.md`](./schema/README.md) for how agents and human reviewers use them.

The schema is structured as:

- **`always` rules** — apply to any RFC at any state ≥ `draft`. Cover header completeness, claim decisiveness, evidence sufficiency, methodology lineage, append-only status log.
- **Gate-specific rules** — `spec-to-rfc`, `rfc-to-forum-posted`, `rfc-to-ratified`, `rfc-to-enacted`. Each gate's rules map directly to the corresponding §Promotion criteria section above.
- **Severities** — `blocker` (prevents graduation), `warning` (must be acknowledged but not blocking), `info` (advisory).

A reviewer (agent or human) emits a structured judgment with verdict (`pass | needs-revision | fail`), per-rule findings with citations, and a `next_action` directive. This converts "is this RFC ready?" from a vibes-check into a structured checklist with citations, so reviewer attention concentrates on the 3–4 judgment-heavy rules per gate rather than mechanical hygiene.

**What the schema deliberately does NOT do:**

- It does not decide ratification. The schema can verify that ratification *criteria are met* but the ratifier (a human steward) still signs the §6 log row. This preserves the two-eyes principle.
- It does not resolve substantive disputes. If two reviewers disagree on whether a counter-position has been "addressed", the schema will flag both findings; humans arbitrate. The schema is a checklist, not a judge of philosophical adequacy.
- It does not handle on-chain submission. Schema rules verify post-hoc that the proposer was human, but never authorize submission.

## Smallest possible first cut

If the goal is to ship the smallest viable version of this:

1. ✅ Create `rfcs/` directory with `_template.md`, `GRADUATION_RULES.md`, and `schema/` (this commit).
2. Pick **one** existing live question — e.g., the M013 fee split (28/25/45/2 vs. 15/30/50/5) — and write **RFC-0001** for it. This forces the template through real use and surfaces what's missing.
3. Run the full lifecycle once (draft → forum-posted → ratified → enacted) before scaling the pattern to the agent stack.
4. Then, and only then, retrofit the Codex agent's PR pipeline with the spec/RFC filter.

The risk of designing the whole pipeline before running one RFC end-to-end is that the template will be wrong in ways that aren't visible until lived experience reveals them.

## Open questions for these rules

- **Who are the stewards / ratifiers?** This document assumes RND PBC members + designated community stewards. Needs explicit roster and rotation rules.
- **How do we handle RFCs that span multiple repos?** (e.g., a fee-split change touches agentic-tokenomics + regen-ledger + a frontend). Proposed: RFC lives in agentic-tokenomics if it's primarily a tokenomics decision; if multi-domain, sponsor decides repo and cross-references.
- **Dispute escalation.** What happens if the sponsor and ratifier disagree on whether deliberation was sufficient? Proposed: third steward arbitrates; if no resolution, RFC parks.
- **Relationship to existing forum governance.** Today, $REGEN governance has its own forum norms. This RFC track needs to either replace or sit alongside that. Recommendation: sit alongside initially, and let lived use determine whether it absorbs the existing track.
- **Relationship to DUNA / Regen Commons governance.** That's a parallel deliberation chamber for the Commons; this is for tokenomics. They likely converge over time but don't need to start unified.

## Status

This is `v0.1` of the graduation rules. Expect refinement through 1.0 as RFC-0001 surfaces gaps in real use.
