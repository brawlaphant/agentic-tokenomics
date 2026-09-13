# RFCs

This directory holds Request-for-Comment documents for agentic-tokenomics — the **deliberation chamber** of the agentic-tokenomics → forum → on-chain pipeline.

## What's an RFC here?

An RFC is a decisive claim about the system, supported by evidence and methodology, with a named human sponsor accountable for shepherding it through deliberation. RFCs sit between **specs** (exploratory designs in `mechanisms/`, `phase-1..5/`) and **on-chain proposals** (executable transactions in `phase-4/proposals/`).

Three artifact types, three purposes:

| | Spec | RFC | On-chain Proposal |
|---|---|---|---|
| **Purpose** | Explore — what could exist | Decide — what should exist | Execute — what now exists |
| **Audience** | Designers, contributors | Stewards, $REGEN stakers, partners | Validators, on-chain governance |
| **Author scope** | Anyone (humans + agents) | Humans + agents (with human sponsor) | Humans only |
| **Location** | `mechanisms/`, `phase-N/` | `rfcs/RFC-XXXX.md` | `phase-4/proposals/RFC-XXXX-proposal.json` |

The why and the full lifecycle are in [`GRADUATION_RULES.md`](./GRADUATION_RULES.md).

## How to author an RFC

1. **Read** [`GRADUATION_RULES.md`](./GRADUATION_RULES.md) and confirm a sponsor is willing.
2. **Copy** [`_template.md`](./_template.md) → `RFC-XXXX.md` (next sequential ID).
3. **Fill in** §0 Header, §1 Claim, §2 Evidence, §3 Methodology — the four load-bearing sections.
4. **Self-review** against the [governance review schema](./schema/) before requesting human review. If your draft has any `blocker`-severity failures, fix them first; this concentrates reviewer attention on substantive judgment, not mechanical hygiene.
5. **Open a PR.** A reviewer (agent or human) emits a structured judgment per the schema. Sponsor addresses findings, iterates.
6. **Graduate** through the lifecycle (`draft` → `forum-posted` → `ratified` → `enacted`) per the rules.

## Index

| ID | Title | Status | Sponsor | Last update |
|---|---|---|---|---|
| RFC-0001 | Propose M013 Fee Distribution Split 15/30/50/5 | draft | sponsor required | 2026-09-13 |

## Why this exists

Today the code chamber of agentic-tokenomics works — agents ship PRs, CI gates merges, mechanism specs accumulate. The deliberation chamber barely exists in the repo. Without a structured deliberation step, two failure modes appear:

1. **Specs accumulate without becoming decisive.** The repo becomes a great library of design exploration with no clear path from "we considered X" to "we decided X."
2. **Specs go directly to chain without deliberation.** Validators get asked to vote on under-deliberated changes; either they reject by default (chilling effect) or rubber-stamp (loss of governance legitimacy).

The RFC layer closes both gaps. It's where claims get sharpened, evidence gets challenged, and a human sponsor takes accountability before anything reaches chain.

## Status

This is `v0.1` of the RFC process. Expect the template, graduation rules, and schema to evolve through 1.0 as the first RFCs are run end-to-end. Schema versions should themselves be governed by an RFC once the system is mature enough to bootstrap on itself.
