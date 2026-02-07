# Phase 2: Mechanism Design & Specification

## Overview

Phase 2 translates Phase 1 discoveries into detailed technical specifications ready for implementation. This phase produces protocol-level designs, workflow definitions, and integration specifications.

## Sub-Phases

| Sub-Phase | Focus | Status |
|-----------|-------|--------|
| 2.1 | Token Utility Mechanism Specifications | ✅ Complete |
| 2.2 | Agentic Workflow Design | ✅ Complete |
| 2.3 | Governance Process Formalization | ✅ Complete |
| 2.4 | Agent Orchestration Architecture | ✅ Complete |
| 2.5 | Data Schema & Integration Specification | ✅ Complete |

## Key Outputs

### Token Mechanisms (5 Protocols)
| ID | Name | Type | Complexity |
|----|------|------|------------|
| M001-ENH | Credit Class Approval Voting | Enhancement | Medium |
| M008 | Data Attestation Bonding | New | High |
| M009 | Service Provision Escrow | New | High |
| M010 | Reputation/Legitimacy Signaling | New | Medium |
| M011 | Marketplace Curation | Enhancement | Medium |

### Agentic Workflows (12 Workflows)
| Agent | Workflows | Automation Level |
|-------|-----------|------------------|
| Registry Reviewer | WF-RR-01, WF-RR-02, WF-RR-03 | Layer 1-2 |
| Governance Analyst | WF-GA-01, WF-GA-02, WF-GA-03 | Layer 1 |
| Market Monitor | WF-MM-01, WF-MM-02, WF-MM-03 | Layer 1-2 |
| Validator Monitor | WF-VM-01, WF-VM-02, WF-VM-03 | Layer 1-3 |

### Governance Processes (5 Formalized)
| ID | Process | Layer |
|----|---------|-------|
| GOV-001 | Credit Class Allowlist | Layer 2-3 |
| GOV-002 | Currency Allow List | Layer 1-2 |
| GOV-003 | Software Upgrade | Layer 2-4 |
| GOV-004 | Community Pool Spend | Layer 3 |
| GOV-005 | Parameter Change | Layer 1-3 |

### Agent Architecture (4 Agents)
| Agent | Type | Primary MCP |
|-------|------|-------------|
| AGENT-001 | Registry Reviewer | KOI + Ledger |
| AGENT-002 | Governance Analyst | Ledger + KOI |
| AGENT-003 | Market Monitor | Ledger |
| AGENT-004 | Validator Monitor | Ledger |

## Outputs

1. [Token Utility Mechanisms](./2.1-token-utility-mechanisms.md)
2. [Agentic Workflows](./2.2-agentic-workflows.md)
3. [Governance Processes](./2.3-governance-processes.md)
4. [Agent Orchestration](./2.4-agent-orchestration.md)
5. [Data Schema & Integration](./2.5-data-schema-integration.md)

## Dependencies Identified

### Implementation Dependencies
- ElizaOS runtime deployment
- DAO DAO contract deployment (for M008, M009 arbitration)
- KOI MCP operational
- Ledger MCP operational

### On-Chain Dependencies
- x/ecocredit module extensions (M001-ENH)
- New x/attestation module (M008)
- New x/reputation module (M010)
- CosmWasm contracts (M009, M011)

## Ready for Phase 3

All specifications are ready for:
1. Smart contract development
2. Agent implementation in ElizaOS
3. Database migrations
4. API development
5. Integration testing
