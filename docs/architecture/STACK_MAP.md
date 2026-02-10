# Stack Map

This diagram is a **navigation aid** for humans and agents. It shows **where work belongs** and **how repositories reference each other** without copying large chunks across repos.

## Diagram

```mermaid
%% Regen Network AI Stack Map (Unit 2)
%% Rendered by GitHub Mermaid. Keep this file small and link-heavy.
flowchart TB
  subgraph AT["agentic-tokenomics (registry of mechanisms)"]
    AT_docs["docs/ (navigation, contributor rules)"]
    AT_mech["mechanisms/ (m010, ...)"]
    AT_schemas["schemas/ (canonical shapes)"]
    AT_scripts["scripts/ (verify, generators)"]
  end

  subgraph KOI["koi-research (research + data plumbing)"]
    KOI_src["sources/ (upstream inputs)"]
    KOI_ext["extractors/ (parsers, ETL)"]
    KOI_onto["ontologies/ (schemas/semantics)"]
    KOI_tools["tools/ + scripts/ (ops utilities)"]
  end

  subgraph CLAUDE["regen-ai-claude (agent skills + interfaces)"]
    CLAUDE_skills["skills/ (Claude configs, prompts)"]
    CLAUDE_mcp["MCP adapter interface (docs + stubs)"]
  end

  subgraph HB["regen-heartbeat (signal emission + digests)"]
    HB_agent["signal-agent (m010 digests)"]
    HB_replay["replay/stub runners (offline review)"]
    HB_validate["validators (no-fabrication, schema checks)"]
  end

  subgraph DEV["Devnet path (placeholder)"]
    DEV_harness["devnet harness (tests + acceptance)"]
    DEV_contracts["contracts (future)"]
  end

  KOI_src --> KOI_ext --> KOI_onto
  KOI_tools --> KOI_ext

  AT_schemas --> HB_validate
  AT_mech --> HB_agent
  AT_scripts --> HB_replay
  AT_docs --> CLAUDE_skills

  CLAUDE_mcp --> HB_agent
  KOI_ext --> HB_agent

  HB_agent --> HB_validate --> HB_replay
  HB_replay --> AT_scripts

  AT_mech -. "spec drives" .-> DEV_contracts
  HB_agent -. "acceptance signals" .-> DEV_harness
  DEV_harness -. "results inform" .-> AT_docs
```

## How to use this

- If you are changing **mechanism specs/schemas/reference implementations**, work in **agentic-tokenomics** under `mechanisms/` and `schemas/`.
- If you are changing **data ingestion / extractors / ontologies**, work in **koi-research** under `extractors/` and `ontologies/`.
- If you are changing **agent configuration and tool interfaces**, work in **regen-ai-claude** (skills + MCP adapter docs/stubs).
- If you are changing **signal generation / replay / validation**, work in **regen-heartbeat**.

## Review strategy

To keep PRs reviewable:
- Prefer **one subtree per PR** (e.g., `docs/` only, or `mechanisms/m010-...` only).
- Link to related repos/docs rather than duplicating content.

## Repo links

- agentic-tokenomics: https://github.com/regen-network/agentic-tokenomics
- koi-research: https://github.com/regen-network/koi-research
- regen-ai-claude: https://github.com/gaiaaiagent/regen-ai-claude
- regen-heartbeat: https://github.com/gaiaaiagent/regen-heartbeat
