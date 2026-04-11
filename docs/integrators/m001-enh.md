# Integrator guide: m001-enh Credit Class Approval Voting

**Mechanism:** Credit Class Approval Voting Enhancement
**Canonical spec:** [`mechanisms/m001-enh-credit-class-approval/SPEC.md`](../../mechanisms/m001-enh-credit-class-approval/SPEC.md)
**Reference implementation:** [`mechanisms/m001-enh-credit-class-approval/reference-impl/m001_score.js`](../../mechanisms/m001-enh-credit-class-approval/reference-impl/m001_score.js)

## 1. What this mechanism does

m001-enh scores proposals for new credit classes on a 0-1000 composite and returns one of three recommendations: **APPROVE**, **CONDITIONAL**, or **REJECT**. It's an *advisory* mechanism in v0 — the score is a decision aid for the Tokenomics Working Group, not an enforcement action. An integrator uses m001-enh to compute the score for a proposal *before* it goes to governance, so voters see a consistent, reproducible recommendation alongside the proposal text.

Four weighted factors drive the composite:

- `methodology_quality` × 0.4 — rigor of the methodology across additionality, baseline, MRV, permanence
- `admin_reputation` × 0.3 — the admin's M010 reputation score (default 500 when no history)
- `novelty` × 0.2 — `(1 - max_similarity_vs_existing_classes) × 1000`
- `completeness` × 0.1 — application checklist score (250 per required item)

## 2. What you give it

```js
import { computeM001Score } from "./m001_score.js";

const result = computeM001Score({
  proposal: {
    proposal_id: "prop-example-001",
    credit_type: "C",                                      // C / KSH / BT / MBS / USS
    methodology_iri: "koi://methodology/soil-carbon-v4",
    admin_address: "regen1qx7yt3e4wk5gk2xnhzrvn0cf",
  },
  factors: {
    // Scoring inputs — each 0..1000, defaults to 0 except
    // admin_reputation which defaults to 500 when not provided.
    methodology_quality: 850,
    admin_reputation: 720,
    novelty: 600,
    completeness: 1000,

    // Confidence flags — boolean. Together they produce a
    // confidence score (count_true / 4) * 1000.
    reputation_available: true,      // M010 record exists for this admin
    methodology_resolvable: true,    // IRI resolves to parseable doc
    sufficient_classes: true,        // >= 3 existing classes for similarity math
    history_available: true,         // prior proposals exist for this admin
  },
});
```

**Schemas:** [`m001_proposal.schema.json`](../../mechanisms/m001-enh-credit-class-approval/schemas/m001_proposal.schema.json) and [`m001_agent_score.schema.json`](../../mechanisms/m001-enh-credit-class-approval/schemas/m001_agent_score.schema.json).

## 3. What you get back

```js
{
  score: 776,                    // 0..1000 composite, clamped
  confidence: 1000,              // 0..1000, derived from availability flags
  recommendation: "APPROVE",     // APPROVE | CONDITIONAL | REJECT
  factors: {                     // clamped factor values echoed back
    methodology_quality: 850,
    admin_reputation: 720,
    novelty: 600,
    completeness: 1000,
  },
}
```

**Recommendation rules** (strict inequalities):

| Condition | Recommendation |
|---|---|
| `score >= 700` | `APPROVE` |
| `score < 300` AND `confidence > 900` | `REJECT` |
| otherwise | `CONDITIONAL` |

The dual condition on REJECT is deliberate — the agent must have high confidence AND a low score to auto-reject. Low score with low confidence falls through to CONDITIONAL so the proposal gets human review instead of being killed by a bad data draw.

## 4. Common error modes

### 4a. Unknown admin, no M010 history

If the admin has never submitted a proposal before, `admin_reputation` defaults to **500** (neutral, not zero). This means a first-time admin's score is the same as a mid-reputation repeat admin for the 0.3 reputation slice. Set `reputation_available: false` so the confidence dips to at most 750 (3/4) and the recommendation path avoids the REJECT branch.

### 4b. Methodology IRI doesn't resolve

Set `methodology_resolvable: false`. This drops confidence by 250 points. If the score is also low, the CONDITIONAL branch fires (guarding against auto-rejecting an unresolvable IRI — the agent should flag it for human triage, not kill it).

### 4c. Too few existing classes for similarity math

If the registry has fewer than 3 existing classes, novelty cannot be meaningfully computed. Pass `novelty: 1000` (most-novel-by-default) and `sufficient_classes: false` so the confidence drops and the agent's score cannot auto-reject the proposal.

### 4d. Score out of range

Inputs are clamped to `[0, 1000]` before weighting. Passing `methodology_quality: 1500` produces the same output as `methodology_quality: 1000`. Passing a negative value clamps to 0. If you need to distinguish "invalid" from "bottom of range", validate upstream — the scoring function treats them identically by design.

### 4e. Boundary score exactly at a threshold

- A score of exactly **700** fires APPROVE (predicate is `>=`).
- A score of exactly **300** fires CONDITIONAL, not REJECT (predicate is strict `<`).

These boundaries are pinned by [`vector_v0_boundary_approve`](../../mechanisms/m001-enh-credit-class-approval/reference-impl/test_vectors/vector_v0_boundary_approve.input.json) and [`vector_v0_boundary_conditional_300`](../../mechanisms/m001-enh-credit-class-approval/reference-impl/test_vectors/vector_v0_boundary_conditional_300.input.json) so a future refactor cannot silently move them.

## 5. Runnable example

The reference implementation ships with 7 test vectors (5 proposals in the sample vector + 6 edge case vectors for boundary conditions). The self-test discovers every vector in `test_vectors/` automatically:

```bash
node mechanisms/m001-enh-credit-class-approval/reference-impl/m001_score.js
```

Expected output:

```
m001_score self-test: PASS (11 proposals across 7 vectors)
```

Each vector file is a pair: `vector_v0_<name>.input.json` (what you pass into `computeM001Score`) and `vector_v0_<name>.expected.json` (what you get back). Drop a new pair into the directory and the self-test picks it up with no code changes.

---

Canonical spec: [`mechanisms/m001-enh-credit-class-approval/SPEC.md`](../../mechanisms/m001-enh-credit-class-approval/SPEC.md) §5.
