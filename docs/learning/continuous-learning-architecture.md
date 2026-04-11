# Continuous Learning Architecture

## Overview

The Regen Network continuous learning system transforms fragmented ecosystem data into coherent, actionable knowledge through a multi-scale temporal architecture.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CONTINUOUS LEARNING ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  DATA SOURCES                PROCESSING               KNOWLEDGE OUTPUTS     │
│                                                                             │
│  ┌─────────────┐            ┌─────────────┐          ┌─────────────┐       │
│  │ Forum       │            │             │          │ Daily       │       │
│  │ Discussions │───────────▶│             │─────────▶│ Digests     │       │
│  └─────────────┘            │             │          └─────────────┘       │
│                             │             │                                │
│  ┌─────────────┐            │   KOI       │          ┌─────────────┐       │
│  │ GitHub      │───────────▶│   KNOWLEDGE │─────────▶│ Weekly      │       │
│  │ Activity    │            │   GRAPH     │          │ Summaries   │       │
│  └─────────────┘            │             │          └─────────────┘       │
│                             │             │                                │
│  ┌─────────────┐            │             │          ┌─────────────┐       │
│  │ Governance  │───────────▶│             │─────────▶│ Monthly     │       │
│  │ On-Chain    │            │             │          │ Reports     │       │
│  └─────────────┘            │             │          └─────────────┘       │
│                             │             │                                │
│  ┌─────────────┐            │             │          ┌─────────────┐       │
│  │ Market      │───────────▶│             │─────────▶│ Yearly      │       │
│  │ Data        │            │             │          │ Reviews     │       │
│  └─────────────┘            └─────────────┘          └─────────────┘       │
│                                                                             │
│                             ┌─────────────┐                                │
│                             │ AGENT       │                                │
│                             │ MEMORY      │◀─────── Persistent Context     │
│                             │             │                                │
│                             └─────────────┘                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Temporal Hierarchy

> "Daily to weekly to monthly to yearly—is not just an organizational convenience. It is a deliberate architecture for making sense of complexity at different scales."

### Scale Characteristics

| Scale | Focus | Content Type | Audience |
|-------|-------|--------------|----------|
| **Daily** | Immediate activity | Events, transactions, posts | Active contributors |
| **Weekly** | Short-term patterns | Trends, aggregations, highlights | Regular participants |
| **Monthly** | Medium-term themes | Analysis, comparisons, milestones | Stakeholders |
| **Yearly** | Long-term evolution | Strategic review, major shifts | Everyone |

---

## Knowledge Sources

### Primary Sources (KOI-Indexed)

| Source | Type | Update Frequency |
|--------|------|------------------|
| **Forum** | Discussion | Real-time |
| **GitHub** | Development | Real-time |
| **Ledger** | On-chain | Per block |
| **Notion** | Internal docs | As updated |
| **Discord/Telegram** | Chat | Continuous |
| **YouTube** | Media | Weekly |

---

## Agent Learning Integration

### How Agents Learn

1. **KOI MCP Queries**: Access tens of thousands of indexed documents
2. **Ledger MCP Queries**: Real-time on-chain state
3. **Digest Consumption**: Structured summaries for context
4. **Memory Persistence**: Cross-session knowledge retention

### Agent Memory Architecture

```yaml
agent_memory:
  short_term:
    type: "conversation_context"
    duration: "single_session"
    storage: "in_memory"

  working_memory:
    type: "task_context"
    duration: "task_completion"
    storage: "redis"

  long_term:
    type: "learned_patterns"
    duration: "persistent"
    storage: "postgresql_pgvector"

  semantic_memory:
    type: "knowledge_graph"
    duration: "permanent"
    storage: "apache_jena"
    access: "sparql_queries"
```

---

## Access Points

### Regen Heartbeat

**URL**: https://gaiaaiagent.github.io/regen-heartbeat/digests/

**Contents**:
- Daily digests (rolling 7 days)
- Weekly summaries (rolling 4 weeks)
- Monthly reports (rolling 12 months)
- Yearly reviews (permanent archive)

### KOI Search

**Access**: Via MCP tools or authenticated API

**Capabilities**:
- Semantic search across all sources
- Entity resolution and linking
- Historical pattern retrieval
- Graph-based exploration

---

## References

- [Regen Heartbeat README](https://gaiaaiagent.github.io/regen-heartbeat/digests/README)
- [KOI Master Implementation Guide](https://github.com/gaiaaiagent/koi-research/blob/main/docs/KOI_MASTER_IMPLEMENTATION_GUIDE.md)
- [HTTP Config Architecture v2](https://github.com/DarrenZal/koi-research/blob/regen-prod/docs/http-config-architecture-v2.md) — external draft in a personal fork (`DarrenZal/koi-research`); pending migration to the official `regen-network/koi-research` repository (see PR [#5](https://github.com/regen-network/agentic-tokenomics/pull/5))

---

*This document is part of the Regen Network Agentic Tokenomics framework.*
