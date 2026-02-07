# Regen Network Agentic Tokenomics & Governance System

A comprehensive framework for building an 65-75% automated governance and tokenomics system for Regen Network, integrating AI agents with on-chain infrastructure.

## Project Overview

This repository contains the design specifications and implementation plans for an agentic tokenomics system that:

- **Automates** routine governance and registry operations
- **Augments** human decision-making with AI-powered analysis
- **Ensures** transparency through KOI knowledge infrastructure
- **Maintains** human oversight for critical decisions

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                    PRESENTATION LAYER                           │
│         Discord │ Twitter │ Telegram │ Web UI │ CLI             │
├─────────────────────────────────────────────────────────────────┤
│                    ELIZAOS RUNTIME                              │
│  Registry Reviewer │ Governance Analyst │ Market │ Validator    │
├─────────────────────────────────────────────────────────────────┤
│                    MCP INTEGRATION LAYER                        │
│         KOI MCP │ Ledger MCP │ TX Builder MCP                   │
├─────────────────────────────────────────────────────────────────┤
│                    DATA LAYER                                   │
│    PostgreSQL (Agent State) │ Jena (Knowledge) │ Regen Ledger   │
└─────────────────────────────────────────────────────────────────┘
```

## Governance Layers

| Layer | Delegation | Automation | Examples |
|-------|------------|------------|----------|
| Layer 1 | Fully Automated | 100% | Price monitoring, data validation |
| Layer 2 | Agentic + Oversight | 85%+ | Credit class screening, routine proposals |
| Layer 3 | Human-in-Loop | 50% | Large grants, contested decisions |
| Layer 4 | Constitutional | 0% | Protocol upgrades, governance changes |

## Phase Structure

### Phase 1: Discovery & Analysis ✅
- Stakeholder & value flow mapping
- Tokenomic mechanisms inventory
- Agentic service identification
- Governance layer architecture
- System architecture synthesis

### Phase 2: Mechanism Design & Specification ✅
- Token utility mechanism specifications (5 protocols)
- Agentic workflow design (12 workflows)
- Governance process formalization (5 processes)
- Agent orchestration architecture (4 agents)
- Data schema & integration specification

### Phase 3: Implementation & Testing (Planned)
- Smart contract development
- Agent implementation
- Integration testing
- Security audits

### Phase 4: Deployment & Migration (Planned)
- Testnet deployment
- Mainnet migration
- Community onboarding

### Phase 5: Operations & Evolution (Planned)
- Monitoring & maintenance
- Continuous improvement
- Community governance handoff

## Key Specifications

## Mechanisms

- [m010 Reputation Signal (v0 advisory)](mechanisms/m010-reputation-signal/SPEC.md)


### Token Mechanisms
- **M001-ENH**: Credit Class Approval Voting Enhancement
- **M008**: Data Attestation Bonding
- **M009**: Service Provision Escrow
- **M010**: Reputation/Legitimacy Signaling
- **M011**: Marketplace Curation & Quality Signals

### Mechanisms (canonical specs)
- [m010 — Reputation Signal (v0 advisory)](mechanisms/m010-reputation-signal/SPEC.md)

### Agent Personas
- **AGENT-001**: Registry Reviewer
- **AGENT-002**: Governance Analyst
- **AGENT-003**: Market Monitor
- **AGENT-004**: Validator Monitor

## Technology Stack

- **Agent Runtime**: ElizaOS
- **Knowledge Graph**: Apache Jena + KOI Framework
- **Blockchain**: Regen Ledger (Cosmos SDK)
- **Smart Contracts**: CosmWasm + DAO DAO
- **Database**: PostgreSQL (pgvector)
- **Cache/Queue**: Redis Streams
- **Embeddings**: BGE (1024-dim)

## On-Chain Context

Based on Regen Network mainnet (as of analysis):
- 13 credit classes
- 58+ registered projects
- 78+ credit batches
- 61+ governance proposals (historical)
- ~3.2M REGEN in community pool
- ~224M REGEN total supply

## Repository Structure

```
regen-agentic-tokenomics/
├── README.md
├── mechanisms
│   └── m010-reputation-signal
│       ├── README.md
│       └── SPEC.md
├── phase-1/
│   ├── README.md
├── mechanisms
│   └── m010-reputation-signal
│       ├── README.md
│       └── SPEC.md
│   ├── 1.1-stakeholder-value-flow.md
│   ├── 1.2-tokenomic-mechanisms.md
│   ├── 1.3-agentic-services.md
│   ├── 1.4-governance-architecture.md
│   └── 1.5-system-architecture.md
├── phase-2/
│   ├── README.md
├── mechanisms
│   └── m010-reputation-signal
│       ├── README.md
│       └── SPEC.md
│   ├── 2.1-token-utility-mechanisms.md
│   ├── 2.2-agentic-workflows.md
│   ├── 2.3-governance-processes.md
│   ├── 2.4-agent-orchestration.md
│   └── 2.5-data-schema-integration.md
├── phase-3/
├── phase-4/
└── phase-5/
```

## Contributing

This project is developed in collaboration with Regen Network community. Contributions, feedback, and discussion are welcome.

## License

[To be determined - suggest Apache 2.0 or MIT for code, CC-BY-SA for documentation]

---

Generated with assistance from Claude Code, grounded in Regen Network's KOI and Ledger MCP infrastructure.
