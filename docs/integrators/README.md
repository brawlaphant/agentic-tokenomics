# Integrator how-to guides

This folder holds **integrator-facing** how-to guides for each mechanism. An integrator is any downstream consumer — an agent, an indexer, a frontend, a report writer, or a smart contract — that needs to call into or read state from one of the `M*` mechanisms without reading the full SPEC.md end-to-end.

The guides are complementary to:

- **`mechanisms/<id>/SPEC.md`** — formal protocol specification. Read this if you're *changing* a mechanism.
- **`mechanisms/<id>/README.md`** — short overview with a link to the SPEC and the reference implementation. Read this if you want to *understand* a mechanism quickly.
- **`mechanisms/<id>/reference-impl/`** — deterministic reference implementation with test vectors. Run this if you want to *verify* your own implementation matches the canonical output.
- **`docs/MECHANISM_CONSUMERS.md`** — map of mechanism ID → known consumers. Read this if you want to know *who already depends* on a mechanism.

**These integrator guides answer a different question:** given I am starting from zero, how do I wire my system to read from or call into this mechanism today, and what do I have to worry about?

## Index

| Guide | Mechanism | Mechanism type | Status |
|---|---|---|---|
| [m001-enh.md](m001-enh.md) | Credit Class Approval Voting | Scoring (0-1000 composite, 3-way recommendation) | ✅ Written |
| [m012.md](m012.md) | Fixed Cap Dynamic Supply | Supply dynamics (BigInt arithmetic, phase-gated multipliers) | ✅ Written |
| [m014.md](m014.md) | Authority Validator Governance | Validator performance (re-normalized weighted score) | ✅ Written |
| m008.md | Data Attestation Bonding | Scoring | ⏳ TODO — follow m001-enh template |
| m009.md | Service Provision Escrow | Dual-guard scoring (score AND confidence) | ⏳ TODO — follow m001-enh template |
| m010.md | Reputation Signal | Stake-weighted endorsement + challenge lifecycle | ⏳ TODO |
| m011.md | Marketplace Curation | 7-factor quality scoring + collections | ⏳ TODO |
| m013.md | Value-Based Fee Routing | Fee computation + pool distribution | ⏳ TODO — follow m012 template |
| m015.md | Contribution-Weighted Rewards | Stability + activity tiers | ⏳ TODO |

## Guide structure

Every integrator guide follows the same five-section structure so a reader can skip to the part they need without reading the whole thing.

### 1. What this mechanism does (one paragraph)

Plain-English. No jargon a new contributor wouldn't already know. Link to the SPEC for detail.

### 2. What you give it (inputs)

The minimum viable call. JSON schemas with one realistic example. If the mechanism has modes (happy path / error path / dispute path), show one example per mode.

### 3. What you get back (outputs)

The deterministic output shape. Every field documented with units, ranges, and meaning. If the output has computed fields (composite scores, confidence, recommendations), explain the math in one line and link to the reference impl for detail.

### 4. Common error modes (and what to do)

Actual failure modes an integrator will hit. Not "bad input" in the abstract — specific things like "what happens when the attester has no prior reputation" or "what happens when the batch denom is from a different class".

### 5. Runnable example

A one-paragraph walkthrough of the reference-impl self-test for this mechanism, ending with the exact command to run it. If the mechanism ships with a generator script, mention it.

## Writing a new guide

1. Copy the structure from [m001-enh.md](m001-enh.md) (scoring mechanism), [m012.md](m012.md) (supply dynamics), or [m014.md](m014.md) (validator performance).
2. Replace the content with the specifics of your mechanism, keeping the section headers identical.
3. Every code example must come from the mechanism's reference implementation or test vectors — never invent numbers.
4. Link to the SPEC.md once at the top and once at the bottom; do not duplicate its prose.
5. Keep the whole guide under 300 lines of rendered Markdown. If you can't, the mechanism is telling you the spec is the right place for that detail.

## Scope boundary — what does NOT belong here

- **Internal state machine derivations** — those live in the SPEC.
- **Governance parameter rationales** — those live in `docs/governance/`.
- **Agent workflow implementations** — those live in `agent-00X-*/src/`.
- **Contract implementation details** — those live in `contracts/<id>/src/` and its inline tests.

The integrator guide is the thin outward-facing layer. If your guide starts to grow into those areas, move the detail to its canonical home and link to it from the guide.
