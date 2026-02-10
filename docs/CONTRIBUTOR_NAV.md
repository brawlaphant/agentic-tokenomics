# Contributor Navigation

Goal: keep this repo **parsimonious and easy for humans + coding agents to review**.

## Where things go (authoritative)

- Mechanism specs + assets: `mechanisms/<mechanism-id>/...`
  - Example: `mechanisms/m010-reputation-signal/`
  - Keep mechanism docs + schemas + reference-impl + test vectors together.
- Cross-mechanism schemas: `schemas/`
- Deterministic validation / tooling: `scripts/`
- Contributor coordination + maps: `docs/`
- Narrative “phase” docs (high-level): `phase-1/`, `phase-2/`, `phase-3/`
  - Do **not** bury mechanism reference code inside phase docs.

## PR sizing rules (to keep reviews coherent)

**One subtree per PR.** Prefer PRs that touch only one of:
- `mechanisms/<id>/...`
- `schemas/...`
- `scripts/...`
- `docs/...`
- `phase-*/...`

If a change needs multiple subtrees, split it into multiple PRs and link them from a short tracking comment.

## “3-bullet PR description” standard

Every PR description should include:

- Lands in: `<folder(s)>`
- Changes: `<one sentence>`
- Validate: `<one command>` (or “docs-only”)

## How to reference `koi-research` (don’t copy)

When a mechanism consumes KOI outputs:
- Link to `koi-research` and the relevant path (extractor, ontology, dataset, or script).
- Record the expected output shape as:
  - a JSON schema (preferred), or
  - an example JSON blob in the mechanism README.

## koi-research structure (for contributors hopping repos)

In `regen-network/koi-research`, align changes to the existing top-level folders:
- `extractors/` — parsers/ingesters
- `ontologies/` — definitions (RDF/TTL, etc.)
- `experiments/` — exploratory work
- `tools/` and `scripts/` — ops + deterministic tooling
- `sources/` and `data/` — source material and derived datasets

Keep PRs there “one folder category” as well.
