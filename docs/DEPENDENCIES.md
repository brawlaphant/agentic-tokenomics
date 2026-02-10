# Dependencies and External Repos

This repo is a coordination + specification registry for “agentic tokenomics” mechanisms. Where possible, **reference external repos instead of copying code**.

## Primary related repos

- `regen-network/koi-research` — research artifacts, datasets, extractors, ontologies, and tooling used to derive/validate signals that mechanisms may consume.
- `regen-network/agentic-tokenomics` (this repo) — mechanism specs, schemas, reference implementations, and contributor instructions.

## Referencing rules (keep PRs small + reviewable)

- Prefer **links + interface notes** over vendoring code.
- If a mechanism depends on an external dataset/tool:
  - Document it in the mechanism README under **“External inputs”** with:
    - repo link
    - path(s) / artifact name(s)
    - minimal expected shape (JSON schema or example)
    - update cadence (if known)
- If you must copy (rare): copy **only stable, versioned artifacts** (e.g., a schema file) and note the upstream commit/PR in the file header.

## Where to put cross-repo notes

- High-level dependency notes: `docs/DEPENDENCIES.md` (this file)
- Mechanism-specific dependency notes: `mechanisms/<id>/README.md` → “External inputs”
