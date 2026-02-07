# WG Packet — Unit 1 (agentic-tokenomics): m010 Reputation Signal (v0 advisory)

## What shipped
- **m010** is the first numbered mechanism spec: a **reputation / legitimacy signal** derived from **stake-weighted endorsements** with **time decay**.
- Scope is **v0 advisory only**: it defines how to compute and publish a score, but does **not** enforce gating, fees, or on-chain privileges.

## Where it lives
- `mechanisms/m010-reputation-signal/SPEC.md` (canonical spec)
- `mechanisms/m010-reputation-signal/README.md` (summary)

## What it enables
- Downstream agents (e.g., Heartbeat `signal-agent`) can reference this spec to:
  - identify candidate subjects (CreditClass/Project/Verifier/Methodology/Address)
  - compute KPIs about evidence coverage + latency
  - publish repeatable, reviewable “reputation signal” sections

## What is explicitly NOT included (yet)
- No on-chain enforcement or contract deployment in v0
- No required category registry / category semantics beyond what’s in SPEC
- No “reputation-weighted voting” implemented—only a signal spec

## Next mechanism candidate (not implemented)
- **m011 Reputation-Weighted Attention** (ordering/surfacing only, no enforcement)

As-of: 2026-02-04
