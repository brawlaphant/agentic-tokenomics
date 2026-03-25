# AGENT-002 Discord Integration Specification

## Overview

This specification defines the ElizaOS plugin for integrating AGENT-002 (Governance Analyst) with Discord, bringing governance intelligence directly to the Regen Network community server. The integration addresses systemic participation deficits identified in the October 2025 governance audit (65% voting power concentration among top validators, 25% average voter turnout) and builds on 18+ months of community Proof of Authority discussions across forum.regen.network involving Will Szal, Gregory Landua, Max Semenchuk, brawlaphant, James Bettauer, and others.

The Discord bot surfaces real-time proposal analysis, voting progress, quorum risk alerts, and economic reboot status updates to the community, lowering the barrier to informed governance participation.

### Design Principles

1. **Informational only** — Layer 1 automation per the 4-layer governance model. The agent provides analysis, never recommendations on how to vote.
2. **Transparency** — Every message discloses the AI agent identity and cites data sources (Ledger MCP, KOI MCP).
3. **Low noise** — Rate-limited, deduplicated, and channel-appropriate posting to avoid alert fatigue.
4. **Graceful degradation** — The bot remains functional with reduced capabilities if upstream MCP servers are unavailable.

### Relationship to Existing Specs

| Spec | Relationship |
|------|-------------|
| Phase 2.2 (Agentic Workflows) | Implements WF-GA-01, WF-GA-02, WF-GA-03 output delivery via Discord |
| Phase 2.6 (Economic Reboot) | Surfaces M012/M013/M014/M015 state transitions and metrics |
| Phase 3.2 (Agent Implementation) | Extends `governanceAnalystCharacter` with Discord-specific plugin |
| Phase 2.4 (Agent Orchestration) | Integrates with ElizaOS runtime and @regen/plugin-social |

---

## Discord Bot Architecture

### Bot Identity

| Property | Value |
|----------|-------|
| Bot Name | Regen Governance Analyst |
| Bot Username | `RegenGovAnalyst` |
| Agent ID | AGENT-002 |
| ElizaOS Plugin | `@regen/plugin-discord-governance` |
| Parent Plugin | `@regen/plugin-social` (extends) |
| Runtime | ElizaOS with `@elizaos/client-discord` |

### Channel Configuration

| Channel | Type | Bot Permissions | Purpose |
|---------|------|----------------|---------|
| `#governance-alerts` | Read-only (bot-only posts) | Send Messages, Embed Links, Add Reactions | Automated governance event notifications |
| `#proposal-discussion` | Interactive | Send Messages, Embed Links, Add Reactions, Read Message History | Community Q&A about governance |
| `#economic-reboot-status` | Read-only (bot-only posts) | Send Messages, Embed Links, Add Reactions | Economic reboot metrics and phase tracking |

### Required Discord Permissions (Intent Flags)

```
Permissions Integer: 274878024768
- Send Messages (0x800)
- Embed Links (0x4000)
- Add Reactions (0x40)
- Read Message History (0x10000)
- Use External Emojis (0x40000)
- View Channels (0x400)

Gateway Intents:
- GUILDS
- GUILD_MESSAGES
- MESSAGE_CONTENT (for command parsing and NL queries)
```

---

## Channel Specifications

### #governance-alerts (Read-Only, Bot-Only)

This channel receives automated posts triggered by AGENT-002 workflows. No community members can post; they react or follow up in `#proposal-discussion`.

#### Message Type 1: New Proposal Detected (WF-GA-01)

**Trigger**: `ProposalSubmitted` event on Regen Ledger
**Scheduled message SLA**: Post within 2 hours of proposal submission (per WF-GA-01 response time target)

> **SLA note**: Scheduled/automated messages (this section) have a 2-hour SLA measured from blockchain event to Discord post. Interactive command responses (§ `#proposal-discussion`) have a separate <5s (cached) / <30s (fresh) SLA measured from user command to bot reply. These are different guarantees with different failure modes.

```json
{
  "embeds": [
    {
      "title": "New Proposal #73: Enable Dynamic Supply Module (M012)",
      "url": "https://regen.network/governance/proposals/73",
      "color": 3447003,
      "description": "A new governance proposal has been submitted and is now in voting period.",
      "fields": [
        {
          "name": "Category",
          "value": "Parameter Change",
          "inline": true
        },
        {
          "name": "Proposer",
          "value": "`regen1qx8...f4m2`",
          "inline": true
        },
        {
          "name": "Deposit Status",
          "value": "5,000 REGEN (Met)",
          "inline": true
        },
        {
          "name": "TL;DR",
          "value": "Activates the M012 fixed-cap dynamic supply mechanism, replacing the current inflationary x/mint module with algorithmic mint/burn cycles tied to ecological activity. Sets hard cap at 221M REGEN.",
          "inline": false
        },
        {
          "name": "Impact Assessment",
          "value": "**Economic**: High — fundamentally changes supply model\n**Technical**: Medium — requires ledger upgrade\n**Governance**: High — constitutional-level change (Layer 4)",
          "inline": false
        },
        {
          "name": "Community Sentiment",
          "value": "Positive (based on 23 forum posts, 89% supportive)",
          "inline": true
        },
        {
          "name": "Historical Precedent",
          "value": "No direct precedent on Regen; analogous to Cosmos Hub Prop #848 (inflation reduction)",
          "inline": true
        },
        {
          "name": "Voting Period",
          "value": "Ends <t:1711900800:R>",
          "inline": true
        },
        {
          "name": "Vote Now",
          "value": "[Keplr](https://wallet.keplr.app/chains/regen/proposals/73) | [CLI](https://docs.regen.network/ledger/governance)",
          "inline": false
        }
      ],
      "footer": {
        "text": "Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP | This is analysis, not a voting recommendation.",
        "icon_url": "https://regen.network/assets/agent-icon.png"
      },
      "timestamp": "2026-03-24T14:30:00.000Z"
    }
  ],
  "components": [
    {
      "type": 1,
      "components": [
        {
          "type": 2,
          "style": 5,
          "label": "Full Analysis",
          "url": "https://regen.network/governance/proposals/73/analysis"
        },
        {
          "type": 2,
          "style": 5,
          "label": "Forum Discussion",
          "url": "https://forum.regen.network/t/proposal-73-discussion"
        },
        {
          "type": 2,
          "style": 5,
          "label": "Vote on Keplr",
          "url": "https://wallet.keplr.app/chains/regen/proposals/73"
        }
      ]
    }
  ]
}
```

#### Message Type 2: Voting Progress Update (WF-GA-02)

**Trigger**: `VoteCast` event OR periodic schedule (every 6h normal, 1h final 48h, 15m final 6h)
**Cooldown**: 30 minutes minimum between updates for the same proposal

```json
{
  "embeds": [
    {
      "title": "Voting Update: Proposal #73 — Enable Dynamic Supply Module",
      "url": "https://regen.network/governance/proposals/73",
      "color": 16776960,
      "fields": [
        {
          "name": "Current Tally",
          "value": "```\nYes     ████████████████░░░░  72.4%  (38.1M REGEN)\nNo      ██░░░░░░░░░░░░░░░░░░   8.2%  ( 4.3M REGEN)\nAbstain ███░░░░░░░░░░░░░░░░░  15.1%  ( 7.9M REGEN)\nVeto    █░░░░░░░░░░░░░░░░░░░   4.3%  ( 2.3M REGEN)\n```",
          "inline": false
        },
        {
          "name": "Turnout",
          "value": "37.8% of staked REGEN",
          "inline": true
        },
        {
          "name": "Quorum Status",
          "value": "Reached (33.4% required)",
          "inline": true
        },
        {
          "name": "Time Remaining",
          "value": "<t:1711900800:R>",
          "inline": true
        },
        {
          "name": "Projected Outcome",
          "value": "**PASS** (Confidence: 0.89)\nBased on current trajectory, historical turnout patterns, and validator voting behavior.",
          "inline": false
        },
        {
          "name": "Key Change Since Last Update",
          "value": "+4.2% turnout, +2.1% Yes share. Three validators voted Yes in last 6h.",
          "inline": false
        }
      ],
      "footer": {
        "text": "Regen Governance Analyst (AI Agent) | Data: Ledger MCP | Projections are statistical estimates, not guarantees.",
        "icon_url": "https://regen.network/assets/agent-icon.png"
      },
      "timestamp": "2026-03-24T20:30:00.000Z"
    }
  ]
}
```

#### Message Type 3: Quorum Risk Alert

**Trigger**: Quorum projected to NOT be met with <48 hours remaining
**Alert Level**: CRITICAL (per WF-GA-02 decide phase)

```json
{
  "embeds": [
    {
      "title": "Quorum Risk Alert: Proposal #73",
      "url": "https://regen.network/governance/proposals/73",
      "color": 15158332,
      "description": "Current turnout trends suggest this proposal may NOT reach quorum before the voting period ends.",
      "fields": [
        {
          "name": "Current Turnout",
          "value": "24.1% of staked REGEN",
          "inline": true
        },
        {
          "name": "Required Quorum",
          "value": "33.4%",
          "inline": true
        },
        {
          "name": "Gap",
          "value": "9.3% (~12.4M REGEN)",
          "inline": true
        },
        {
          "name": "Time Remaining",
          "value": "<t:1711900800:R> (41h 23m)",
          "inline": true
        },
        {
          "name": "Projected Turnout at Close",
          "value": "29.7% (linear extrapolation)\n31.2% (historical pattern adjustment)",
          "inline": true
        },
        {
          "name": "Quorum Probability",
          "value": "38% (down from 62% at last update)",
          "inline": true
        },
        {
          "name": "What This Means",
          "value": "If quorum is not met, the proposal's deposit (5,000 REGEN) will be returned and the proposal will not pass regardless of vote distribution. Voters can participate via [Keplr](https://wallet.keplr.app/chains/regen/proposals/73) or CLI.",
          "inline": false
        }
      ],
      "footer": {
        "text": "Regen Governance Analyst (AI Agent) | Data: Ledger MCP | This is a factual alert, not a call to vote in any direction.",
        "icon_url": "https://regen.network/assets/agent-icon.png"
      },
      "timestamp": "2026-03-24T08:15:00.000Z"
    }
  ]
}
```

#### Message Type 4: Vote Concluded (WF-GA-03)

**Trigger**: `ProposalFinalized` event on Regen Ledger

```json
{
  "embeds": [
    {
      "title": "Vote Concluded: Proposal #73 — PASSED",
      "url": "https://regen.network/governance/proposals/73",
      "color": 3066993,
      "fields": [
        {
          "name": "Final Results",
          "value": "```\nYes     ████████████████░░░░  76.1%  (52.3M REGEN)\nNo      █░░░░░░░░░░░░░░░░░░░   5.8%  ( 4.0M REGEN)\nAbstain ███░░░░░░░░░░░░░░░░░  14.2%  ( 9.8M REGEN)\nVeto    █░░░░░░░░░░░░░░░░░░░   3.9%  ( 2.7M REGEN)\n```",
          "inline": false
        },
        {
          "name": "Final Turnout",
          "value": "49.2% of staked REGEN",
          "inline": true
        },
        {
          "name": "Quorum",
          "value": "Met (49.2% vs 33.4% required)",
          "inline": true
        },
        {
          "name": "Unique Voters",
          "value": "847 addresses",
          "inline": true
        },
        {
          "name": "Prediction Accuracy",
          "value": "Agent predicted PASS at 0.89 confidence — Correct\nTurnout predicted: 45.6% (actual: 49.2%, error: 3.6%)\nYes share predicted: 74.8% (actual: 76.1%, error: 1.3%)",
          "inline": false
        },
        {
          "name": "Voting Pattern Insights",
          "value": "- 68% of validator voting power voted in final 48h\n- Top 10 validators: 8 Yes, 1 Abstain, 1 No\n- Delegator override rate: 12.3% (above 9.1% historical average)",
          "inline": false
        },
        {
          "name": "What Happens Next",
          "value": "The M012 Dynamic Supply module will activate following the next chain upgrade, contingent on M013 (Fee Routing) and M014 (PoA Governance) being active.",
          "inline": false
        }
      ],
      "footer": {
        "text": "Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP | Full post-vote report archived to KOI.",
        "icon_url": "https://regen.network/assets/agent-icon.png"
      },
      "timestamp": "2026-03-24T23:59:00.000Z"
    }
  ]
}
```

#### Message Type 5: Economic Reboot Milestone

**Trigger**: State transition in M012, M013, M014, or M015 modules

```json
{
  "embeds": [
    {
      "title": "Economic Reboot Milestone: M014 Entered TRANSITION Phase",
      "color": 10181046,
      "description": "The Proof of Authority governance module (M014) has transitioned from INACTIVE to TRANSITION state. This begins the coexistence period where both PoS validators and PoA authority validators operate in parallel.",
      "fields": [
        {
          "name": "Module",
          "value": "M014 — PoA Governance",
          "inline": true
        },
        {
          "name": "Previous State",
          "value": "INACTIVE",
          "inline": true
        },
        {
          "name": "New State",
          "value": "TRANSITION",
          "inline": true
        },
        {
          "name": "Overall Reboot Progress",
          "value": "```\nM013 Fee Routing       [ACTIVE]      ████████████████████  Done\nM014 PoA Governance    [TRANSITION]  ██████████░░░░░░░░░░  50%\nM012 Dynamic Supply    [PENDING]     ░░░░░░░░░░░░░░░░░░░░  Blocked on M013+M014\nM015 Contribution Rwds [PENDING]     ░░░░░░░░░░░░░░░░░░░░  Blocked on M014\n```",
          "inline": false
        },
        {
          "name": "Impact",
          "value": "During TRANSITION, the supply module uses `max(staking_multiplier, stability_multiplier)` to prevent regrowth discontinuity. No immediate token holder action required.",
          "inline": false
        }
      ],
      "footer": {
        "text": "Regen Governance Analyst (AI Agent) | Data: Ledger MCP | Economic reboot status is informational only.",
        "icon_url": "https://regen.network/assets/agent-icon.png"
      },
      "timestamp": "2026-03-24T16:00:00.000Z"
    }
  ]
}
```

---

### #proposal-discussion (Interactive)

This channel supports community interaction with the Governance Analyst bot. Community members can use both slash-style commands and natural language queries.

#### Command Reference

| Command | Description | Workflow | Response Time |
|---------|-------------|----------|---------------|
| `!proposal <id>` | Full proposal analysis | WF-GA-01 | <5s (cached) / <30s (fresh) |
| `!tally <id>` | Current voting status | WF-GA-02 | <5s |
| `!compare <id1> <id2>` | Side-by-side proposal comparison | WF-GA-01 | <15s |
| `!history [count]` | Recent proposal summary (default: 5) | WF-GA-03 | <5s |
| `!params <module>` | Current governance parameters | Ledger MCP | <5s |
| `!reboot-status` | Economic reboot phase summary | Ledger MCP | <5s |
| Natural language | LLM-powered governance Q&A | All WF-GA-* | <15s |

#### Command: `!proposal <id>`

Returns a comprehensive analysis embed equivalent to the WF-GA-01 output.

```
User: !proposal 73

Bot: [Proposal Analysis Embed — same structure as #governance-alerts "New Proposal Detected" but with additional detail fields:]
  - Full text summary (up to 1024 chars)
  - Related proposals (linked by topic or proposer)
  - Forum discussion link and sentiment summary
  - Current voting state (if in voting period)
```

#### Command: `!tally <id>`

Returns the current voting state for an active proposal.

```
User: !tally 73

Bot: [Voting Progress Embed — same structure as #governance-alerts "Voting Progress Update"]
```

#### Command: `!compare <id1> <id2>`

Returns a side-by-side comparison of two proposals.

```json
{
  "embeds": [
    {
      "title": "Proposal Comparison: #71 vs #73",
      "color": 3447003,
      "fields": [
        {
          "name": "Proposal #71",
          "value": "**Community Pool Spend: Regen Registry Audit**\nType: Spend | Status: Passed | Turnout: 38.4%",
          "inline": true
        },
        {
          "name": "Proposal #73",
          "value": "**Enable Dynamic Supply Module (M012)**\nType: Parameter Change | Status: Voting | Turnout: 37.8%",
          "inline": true
        },
        {
          "name": "\u200b",
          "value": "\u200b",
          "inline": false
        },
        {
          "name": "#71 Impact",
          "value": "Economic: Low\nTechnical: None\nGovernance: Low",
          "inline": true
        },
        {
          "name": "#73 Impact",
          "value": "Economic: High\nTechnical: Medium\nGovernance: High",
          "inline": true
        },
        {
          "name": "Key Differences",
          "value": "- #71 is a routine spend proposal; #73 is a constitutional-level mechanism change\n- #73 requires chain upgrade; #71 did not\n- #73 has higher community engagement (23 forum posts vs 7)\n- Both have similar turnout, but #73 is still in voting period",
          "inline": false
        }
      ],
      "footer": {
        "text": "Regen Governance Analyst (AI Agent) | Comparison is factual, not evaluative.",
        "icon_url": "https://regen.network/assets/agent-icon.png"
      }
    }
  ]
}
```

#### Command: `!history [count]`

Returns a summary of the most recent proposals (default 5, max 20).

```
User: !history

Bot:
┌─────────────────────────────────────────────────────────────────┐
│  Recent Governance Proposals                                      │
├──────┬─────────────────────────────────┬───────┬────────┬────────┤
│  #73 │ Enable Dynamic Supply (M012)    │ Vote  │ 37.8%  │ 4d rem │
│  #72 │ Upgrade IBC to v7.3             │ Pass  │ 51.2%  │ Yes 89%│
│  #71 │ Community Pool: Registry Audit   │ Pass  │ 38.4%  │ Yes 81%│
│  #70 │ Reduce Deposit to 2,500 REGEN   │ Fail  │ 28.1%  │ No  53%│
│  #69 │ Add NCT Credit Class            │ Pass  │ 44.7%  │ Yes 92%│
└──────┴─────────────────────────────────┴───────┴────────┴────────┘
```

#### Command: `!params <module>`

Returns current on-chain governance parameters for the specified module.

```
User: !params gov

Bot:
┌─────────────────────────────────────────────┐
│  x/gov Parameters (current on-chain state)   │
├─────────────────────────┬───────────────────┤
│  min_deposit            │ 5,000 REGEN       │
│  max_deposit_period     │ 14 days           │
│  voting_period          │ 14 days           │
│  quorum                 │ 33.4%             │
│  threshold              │ 50%               │
│  veto_threshold         │ 33.4%             │
│  min_initial_deposit    │ 20%               │
└─────────────────────────┴───────────────────┘
Source: Ledger MCP (queried <t:1711310400:R>)
```

#### Command: `!reboot-status`

Returns the current state of all economic reboot mechanisms.

```
User: !reboot-status

Bot: [Economic Reboot Status Embed — same structure as #economic-reboot-status milestone embed, with all four mechanism states]
```

#### Natural Language Queries

The bot responds to governance-related questions using the ElizaOS LLM pipeline with context from Ledger MCP and KOI MCP. The bot will NOT respond to off-topic messages.

**Topic Detection**: Messages are evaluated by the `governanceRelevanceEvaluator` which checks for governance-related keywords, proposal references, or economic reboot terminology. Confidence threshold: 0.6.

**Examples**:
- "What proposals are active right now?" -> Lists active proposals
- "How does the quorum requirement work?" -> Explains governance parameters
- "What happened with the last parameter change proposal?" -> Retrieves WF-GA-03 post-vote analysis
- "Is voter turnout improving?" -> Trend analysis from historical data

---

### #economic-reboot-status (Read-Only, Bot-Only)

Dedicated channel for tracking the Regen Economic Reboot roadmap (M012, M013, M014, M015).

#### Scheduled Posts

**Daily (08:00 UTC)**:

| Metric | Source Mechanism | Data Source |
|--------|-----------------|-------------|
| Fee revenue (24h) | M013 Fee Routing | Ledger MCP: tx fee events |
| Burn amount (24h) | M013 burn_share | Ledger MCP: EventSupplyBurn |
| Circulating supply | M012 Dynamic Supply | Ledger MCP: supply state |
| Supply delta (24h) | M012 | Calculated: M[t] - B[t] |

```json
{
  "embeds": [
    {
      "title": "Daily Economic Reboot Report — 2026-03-24",
      "color": 3066993,
      "fields": [
        {
          "name": "Fee Revenue (24h)",
          "value": "1,247.83 REGEN across 3,412 transactions",
          "inline": true
        },
        {
          "name": "Burned (24h)",
          "value": "498.71 REGEN (40% burn share)",
          "inline": true
        },
        {
          "name": "Minted (24h)",
          "value": "312.50 REGEN (regrowth)",
          "inline": true
        },
        {
          "name": "Circulating Supply",
          "value": "218,342,156 / 221,000,000 REGEN (98.8% of cap)",
          "inline": false
        },
        {
          "name": "Net Supply Change",
          "value": "-186.21 REGEN (deflationary day)",
          "inline": true
        },
        {
          "name": "7-Day Trend",
          "value": "Net burn: 1,204 REGEN (avg -172/day)",
          "inline": true
        }
      ],
      "footer": {
        "text": "Regen Governance Analyst (AI Agent) | Data: Ledger MCP | Auto-posted daily at 08:00 UTC",
        "icon_url": "https://regen.network/assets/agent-icon.png"
      },
      "timestamp": "2026-03-24T08:00:00.000Z"
    }
  ]
}
```

**Weekly (Monday 08:00 UTC)**:

| Metric | Source Mechanism | Data Source |
|--------|-----------------|-------------|
| Validator performance summary | M014 PoA Governance | Ledger MCP: validator stats |
| Authority validator set status | M014 | Ledger MCP: active set |
| Reward distribution summary | M015 Contribution Rewards | Ledger MCP: distribution events |
| Contribution scores (top 10) | M015 | Ledger MCP + KOI MCP |

```json
{
  "embeds": [
    {
      "title": "Weekly Economic Reboot Report — Week of 2026-03-17",
      "color": 10181046,
      "fields": [
        {
          "name": "Validator Performance (M014)",
          "value": "Active authority validators: 21/25\nAverage uptime: 99.4%\nMissed blocks: 127 (across all validators)\nSlashing events: 0",
          "inline": false
        },
        {
          "name": "Fee Revenue (M013, 7-day)",
          "value": "8,734.21 REGEN across 23,891 transactions\nTop fee categories: ecocredit ops (42%), IBC transfers (31%), governance (12%)",
          "inline": false
        },
        {
          "name": "Reward Distribution (M015)",
          "value": "Total distributed: 2,187.50 REGEN\nRecipient categories: validators (45%), ecological contributors (30%), stability commitments (25%)",
          "inline": false
        },
        {
          "name": "Supply State (M012)",
          "value": "Opening: 218,343,360 REGEN\nClosing: 218,342,156 REGEN\nNet change: -1,204 REGEN (deflationary week)\nCap utilization: 98.8%",
          "inline": false
        }
      ],
      "footer": {
        "text": "Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP | Auto-posted weekly on Mondays at 08:00 UTC",
        "icon_url": "https://regen.network/assets/agent-icon.png"
      }
    }
  ]
}
```

**On-Event (Immediate)**:

- Phase transitions for M012, M013, M014, M015 (uses Message Type 5 from #governance-alerts)
- Parameter changes to any economic reboot module
- Emergency alerts: slashing events, supply invariant warnings, fee routing anomalies

---

## ElizaOS Plugin Specification

### Plugin Definition

```typescript
// packages/plugin-discord-governance/src/index.ts
import {
  Plugin,
  IAgentRuntime,
  Action,
  Provider,
  Evaluator,
  Service,
  ServiceType,
} from "@elizaos/core";
import { DiscordGovernanceService } from "./services/discord-governance";
import { proposalAlertAction } from "./actions/proposal-alert";
import { votingUpdateAction } from "./actions/voting-update";
import { quorumRiskAlertAction } from "./actions/quorum-risk-alert";
import { postVoteReportAction } from "./actions/post-vote-report";
import { rebootMilestoneAction } from "./actions/reboot-milestone";
import { proposalQueryAction } from "./actions/proposal-query";
import { tallyQueryAction } from "./actions/tally-query";
import { compareProposalsAction } from "./actions/compare-proposals";
import { historyQueryAction } from "./actions/history-query";
import { paramsQueryAction } from "./actions/params-query";
import { rebootStatusQueryAction } from "./actions/reboot-status-query";
import { governanceStateProvider } from "./providers/governance-state";
import { rebootStateProvider } from "./providers/reboot-state";
import { votingPowerProvider } from "./providers/voting-power";
import { governanceRelevanceEvaluator } from "./evaluators/governance-relevance";
import { responseQualityEvaluator } from "./evaluators/response-quality";

export const discordGovernancePlugin: Plugin = {
  name: "@regen/plugin-discord-governance",
  description:
    "Discord integration for AGENT-002 Governance Analyst — " +
    "delivers governance intelligence, proposal analysis, voting updates, " +
    "and economic reboot status to Regen community Discord channels.",

  actions: [
    // Automated alert actions (push to #governance-alerts)
    proposalAlertAction,
    votingUpdateAction,
    quorumRiskAlertAction,
    postVoteReportAction,
    rebootMilestoneAction,

    // Interactive query actions (respond in #proposal-discussion)
    proposalQueryAction,
    tallyQueryAction,
    compareProposalsAction,
    historyQueryAction,
    paramsQueryAction,
    rebootStatusQueryAction,
  ],

  providers: [
    governanceStateProvider,
    rebootStateProvider,
    votingPowerProvider,
  ],

  evaluators: [
    governanceRelevanceEvaluator,
    responseQualityEvaluator,
  ],

  services: [
    DiscordGovernanceService,
  ],
};

export default discordGovernancePlugin;
```

### Discord Client Setup

```typescript
// packages/plugin-discord-governance/src/services/discord-governance.ts
import {
  Service,
  ServiceType,
  IAgentRuntime,
} from "@elizaos/core";
import {
  Client,
  GatewayIntentBits,
  TextChannel,
  EmbedBuilder,
  ActionRowBuilder,
  ButtonBuilder,
  ButtonStyle,
} from "discord.js";
import { LedgerMCPClient } from "@regen/plugin-ledger-mcp";
import { KOIMCPClient } from "@regen/plugin-koi-mcp";
import { CommandRouter } from "../router/command-router";
import { RateLimiter } from "../util/rate-limiter";
import { EmbedTemplates } from "../templates/embed-templates";

interface DiscordGovernanceConfig {
  botToken: string;
  guildId: string;
  channels: {
    governanceAlerts: string;   // channel ID for #governance-alerts
    proposalDiscussion: string; // channel ID for #proposal-discussion
    rebootStatus: string;       // channel ID for #economic-reboot-status
  };
  rateLimits: {
    alertsPerHour: number;      // default: 10
    cooldownMinutes: number;    // default: 30 (per-proposal)
  };
}

export class DiscordGovernanceService extends Service {
  static serviceType: ServiceType = ServiceType.DISCORD;

  private client: Client;
  private config: DiscordGovernanceConfig;
  private runtime: IAgentRuntime;
  private ledgerMCP: LedgerMCPClient;
  private koiMCP: KOIMCPClient;
  private commandRouter: CommandRouter;
  private rateLimiter: RateLimiter;
  private templates: EmbedTemplates;

  // Channel references (resolved on ready)
  private alertsChannel: TextChannel | null = null;
  private discussionChannel: TextChannel | null = null;
  private rebootChannel: TextChannel | null = null;

  async initialize(runtime: IAgentRuntime): Promise<void> {
    this.runtime = runtime;

    this.config = {
      botToken: runtime.getSetting("DISCORD_BOT_TOKEN"),
      guildId: runtime.getSetting("DISCORD_GUILD_ID"),
      channels: {
        governanceAlerts: runtime.getSetting("DISCORD_CHANNEL_GOVERNANCE_ALERTS"),
        proposalDiscussion: runtime.getSetting("DISCORD_CHANNEL_PROPOSAL_DISCUSSION"),
        rebootStatus: runtime.getSetting("DISCORD_CHANNEL_REBOOT_STATUS"),
      },
      rateLimits: {
        alertsPerHour: Number(runtime.getSetting("DISCORD_ALERTS_PER_HOUR") || "10"),
        cooldownMinutes: Number(runtime.getSetting("DISCORD_COOLDOWN_MINUTES") || "30"),
      },
    };

    // Initialize MCP clients
    this.ledgerMCP = new LedgerMCPClient({
      baseUrl: runtime.getSetting("LEDGER_MCP_URL") || "http://localhost:3001",
      apiKey: runtime.getSetting("LEDGER_MCP_API_KEY"),
    });

    this.koiMCP = new KOIMCPClient({
      baseUrl: runtime.getSetting("KOI_MCP_URL") || "http://localhost:3002",
      apiKey: runtime.getSetting("KOI_MCP_API_KEY"),
    });

    // Initialize subsystems
    this.rateLimiter = new RateLimiter(this.config.rateLimits);
    this.templates = new EmbedTemplates();
    this.commandRouter = new CommandRouter(
      this.ledgerMCP,
      this.koiMCP,
      this.templates,
      runtime
    );

    // Initialize Discord client
    this.client = new Client({
      intents: [
        GatewayIntentBits.Guilds,
        GatewayIntentBits.GuildMessages,
        GatewayIntentBits.MessageContent,
      ],
    });

    this.client.once("ready", () => this.onReady());
    this.client.on("messageCreate", (msg) => this.onMessage(msg));

    await this.client.login(this.config.botToken);
  }

  private async onReady(): Promise<void> {
    const guild = this.client.guilds.cache.get(this.config.guildId);
    if (!guild) {
      throw new Error(`Guild ${this.config.guildId} not found`);
    }

    this.alertsChannel = guild.channels.cache.get(
      this.config.channels.governanceAlerts
    ) as TextChannel;
    this.discussionChannel = guild.channels.cache.get(
      this.config.channels.proposalDiscussion
    ) as TextChannel;
    this.rebootChannel = guild.channels.cache.get(
      this.config.channels.rebootStatus
    ) as TextChannel;

    console.log(
      `[AGENT-002] Discord Governance Analyst connected to guild: ${guild.name}`
    );

    // Start scheduled tasks
    this.startScheduledTasks();

    // Subscribe to on-chain events via Ledger MCP
    this.subscribeToGovernanceEvents();
  }

  private async onMessage(message: any): Promise<void> {
    // Ignore bot's own messages
    if (message.author.bot) return;

    // Only respond in #proposal-discussion
    if (message.channel.id !== this.config.channels.proposalDiscussion) return;

    try {
      const response = await this.commandRouter.route(message.content, {
        userId: message.author.id,
        channelId: message.channel.id,
        guildId: message.guild?.id,
      });

      if (response) {
        await message.reply(response);
      }
    } catch (error) {
      console.error("[AGENT-002] Error handling message:", error);
      await message.reply({
        embeds: [
          this.templates.errorEmbed(
            "I encountered an error processing your request. " +
            "Please try again shortly or use a specific command like `!proposal 73`."
          ),
        ],
      });
    }
  }

  private startScheduledTasks(): void {
    // Daily economic reboot report at 08:00 UTC
    this.scheduleDailyReport();

    // Weekly report on Mondays at 08:00 UTC
    this.scheduleWeeklyReport();
  }

  private async scheduleDailyReport(): Promise<void> {
    const now = new Date();
    const next0800 = new Date(now);
    next0800.setUTCHours(8, 0, 0, 0);
    if (next0800 <= now) {
      next0800.setUTCDate(next0800.getUTCDate() + 1);
    }

    const delay = next0800.getTime() - now.getTime();
    setTimeout(async () => {
      await this.postDailyRebootReport();
      // Re-schedule for next day (every 24h)
      setInterval(() => this.postDailyRebootReport(), 24 * 60 * 60 * 1000);
    }, delay);
  }

  private async scheduleWeeklyReport(): Promise<void> {
    const now = new Date();
    const daysUntilMonday = (8 - now.getUTCDay()) % 7 || 7;
    const nextMonday = new Date(now);
    nextMonday.setUTCDate(now.getUTCDate() + daysUntilMonday);
    nextMonday.setUTCHours(8, 0, 0, 0);

    const delay = nextMonday.getTime() - now.getTime();
    setTimeout(async () => {
      await this.postWeeklyRebootReport();
      // Re-schedule for next week (every 7 days)
      setInterval(
        () => this.postWeeklyRebootReport(),
        7 * 24 * 60 * 60 * 1000
      );
    }, delay);
  }

  private async subscribeToGovernanceEvents(): Promise<void> {
    // Poll for governance events (Cosmos SDK does not support WebSocket
    // subscriptions via MCP; use polling with backoff)
    const POLL_INTERVAL_MS = 15_000; // 15 seconds

    setInterval(async () => {
      try {
        await this.checkForNewProposals();
        await this.checkForVoteUpdates();
        await this.checkForRebootStateChanges();
      } catch (error) {
        console.error("[AGENT-002] Event polling error:", error);
        // Graceful degradation: log and continue polling
      }
    }, POLL_INTERVAL_MS);
  }

  // --- Alert Posting Methods ---

  async postProposalAlert(proposalId: number): Promise<void> {
    if (!this.rateLimiter.canPost("alerts")) return;
    if (!this.rateLimiter.canPostForProposal(proposalId)) return;
    if (!this.alertsChannel) return;

    const proposal = await this.ledgerMCP.getProposal(proposalId);
    const context = await this.koiMCP.search({
      query: proposal.content.title,
      intent: "general",
      limit: 5,
    });

    const embed = this.templates.proposalAnalysisEmbed(proposal, context);
    const buttons = this.templates.proposalButtons(proposalId);

    await this.alertsChannel.send({ embeds: [embed], components: [buttons] });
    this.rateLimiter.recordPost("alerts", proposalId);
  }

  async postVotingUpdate(proposalId: number): Promise<void> {
    if (!this.rateLimiter.canPost("alerts")) return;
    if (!this.rateLimiter.canPostForProposal(proposalId)) return;
    if (!this.alertsChannel) return;

    const tally = await this.ledgerMCP.call("get_governance_tally", {
      proposal_id: proposalId,
    });
    const proposal = await this.ledgerMCP.getProposal(proposalId);

    const embed = this.templates.votingProgressEmbed(proposal, tally);
    await this.alertsChannel.send({ embeds: [embed] });
    this.rateLimiter.recordPost("alerts", proposalId);
  }

  async postQuorumRiskAlert(proposalId: number): Promise<void> {
    if (!this.alertsChannel) return;

    // Quorum risk alerts bypass normal cooldown but still respect hourly cap
    if (!this.rateLimiter.canPost("alerts")) return;

    const tally = await this.ledgerMCP.call("get_governance_tally", {
      proposal_id: proposalId,
    });
    const proposal = await this.ledgerMCP.getProposal(proposalId);

    const embed = this.templates.quorumRiskEmbed(proposal, tally);
    await this.alertsChannel.send({ embeds: [embed] });
    this.rateLimiter.recordPost("alerts", proposalId);
  }

  async postVoteConcluded(proposalId: number): Promise<void> {
    if (!this.alertsChannel) return;

    const proposal = await this.ledgerMCP.getProposal(proposalId);
    const tally = await this.ledgerMCP.call("get_governance_tally", {
      proposal_id: proposalId,
    });

    const embed = this.templates.postVoteEmbed(proposal, tally);
    await this.alertsChannel.send({ embeds: [embed] });
  }

  async postRebootMilestone(
    module: string,
    previousState: string,
    newState: string
  ): Promise<void> {
    if (!this.alertsChannel) return;

    const embed = this.templates.rebootMilestoneEmbed(
      module,
      previousState,
      newState
    );
    await this.alertsChannel.send({ embeds: [embed] });
  }

  private async postDailyRebootReport(): Promise<void> {
    if (!this.rebootChannel) return;

    try {
      const supplyState = await this.ledgerMCP.call("get_supply_state", {});
      const feeRevenue = await this.ledgerMCP.call("get_fee_revenue", {
        period: "24h",
      });

      const embed = this.templates.dailyRebootEmbed(supplyState, feeRevenue);
      await this.rebootChannel.send({ embeds: [embed] });
    } catch (error) {
      console.error("[AGENT-002] Failed to post daily reboot report:", error);
    }
  }

  private async postWeeklyRebootReport(): Promise<void> {
    if (!this.rebootChannel) return;

    try {
      const validatorStats = await this.ledgerMCP.call(
        "get_validator_summary",
        { period: "7d" }
      );
      const rewardDistribution = await this.ledgerMCP.call(
        "get_reward_distribution",
        { period: "7d" }
      );
      const supplyState = await this.ledgerMCP.call("get_supply_state", {});
      const feeRevenue = await this.ledgerMCP.call("get_fee_revenue", {
        period: "7d",
      });

      const embed = this.templates.weeklyRebootEmbed(
        validatorStats,
        rewardDistribution,
        supplyState,
        feeRevenue
      );
      await this.rebootChannel.send({ embeds: [embed] });
    } catch (error) {
      console.error("[AGENT-002] Failed to post weekly reboot report:", error);
    }
  }

  // --- Event Detection Methods ---

  private lastSeenProposalId: number = 0;
  private knownVoteCounts: Map<number, number> = new Map();
  private knownRebootStates: Map<string, string> = new Map();

  private async checkForNewProposals(): Promise<void> {
    try {
      const proposals = await this.ledgerMCP.call("list_governance_proposals", {
        status: "PROPOSAL_STATUS_VOTING_PERIOD",
        limit: 10,
      });

      for (const proposal of proposals.proposals || []) {
        const id = Number(proposal.id);
        if (id > this.lastSeenProposalId) {
          this.lastSeenProposalId = id;
          await this.postProposalAlert(id);
        }
      }
    } catch (error) {
      // Graceful degradation: skip this poll cycle
    }
  }

  private async checkForVoteUpdates(): Promise<void> {
    try {
      const proposals = await this.ledgerMCP.call("list_governance_proposals", {
        status: "PROPOSAL_STATUS_VOTING_PERIOD",
        limit: 10,
      });

      for (const proposal of proposals.proposals || []) {
        const id = Number(proposal.id);
        const tally = await this.ledgerMCP.call("get_governance_tally", {
          proposal_id: id,
        });

        const totalVotes =
          Number(tally.yes || 0) +
          Number(tally.no || 0) +
          Number(tally.abstain || 0) +
          Number(tally.no_with_veto || 0);

        const previousTotal = this.knownVoteCounts.get(id) || 0;

        if (totalVotes > previousTotal) {
          this.knownVoteCounts.set(id, totalVotes);
          await this.postVotingUpdate(id);

          // Check quorum risk
          const quorumMet = await this.isQuorumLikelyMet(id, tally, proposal);
          if (!quorumMet) {
            await this.postQuorumRiskAlert(id);
          }
        }
      }
    } catch (error) {
      // Graceful degradation: skip this poll cycle
    }
  }

  private async checkForRebootStateChanges(): Promise<void> {
    const modules = ["m012", "m013", "m014", "m015"];

    for (const module of modules) {
      try {
        const state = await this.ledgerMCP.call("get_module_state", {
          module,
        });

        const previousState = this.knownRebootStates.get(module);
        if (previousState && previousState !== state.state) {
          await this.postRebootMilestone(module, previousState, state.state);
        }
        this.knownRebootStates.set(module, state.state);
      } catch (error) {
        // Module may not be deployed yet; skip gracefully
      }
    }
  }

  private async isQuorumLikelyMet(
    proposalId: number,
    tally: any,
    proposal: any
  ): Promise<boolean> {
    const totalVoted =
      Number(tally.yes || 0) +
      Number(tally.no || 0) +
      Number(tally.abstain || 0) +
      Number(tally.no_with_veto || 0);

    const stakingPool = await this.ledgerMCP.call("get_staking_pool", {});
    const totalStaked = Number(stakingPool.bonded_tokens || 0);
    const turnout = totalStaked > 0 ? totalVoted / totalStaked : 0;

    const quorumRequired = 0.334; // 33.4%

    // If already met, no risk
    if (turnout >= quorumRequired) return true;

    // Calculate time remaining
    const votingEnd = new Date(proposal.voting_end_time).getTime();
    const now = Date.now();
    const hoursRemaining = (votingEnd - now) / (1000 * 60 * 60);

    // Only alert if <48h remaining and quorum not met
    if (hoursRemaining > 48) return true;

    // Linear projection
    const votingStart = new Date(proposal.voting_start_time).getTime();
    const elapsed = now - votingStart;
    const totalPeriod = votingEnd - votingStart;
    const projectedTurnout = turnout * (totalPeriod / elapsed);

    return projectedTurnout >= quorumRequired;
  }

  async cleanup(): Promise<void> {
    if (this.client) {
      this.client.destroy();
    }
  }
}
```

### Command Router

```typescript
// packages/plugin-discord-governance/src/router/command-router.ts
import { IAgentRuntime } from "@elizaos/core";
import { LedgerMCPClient } from "@regen/plugin-ledger-mcp";
import { KOIMCPClient } from "@regen/plugin-koi-mcp";
import { EmbedTemplates } from "../templates/embed-templates";

interface MessageContext {
  userId: string;
  channelId: string;
  guildId?: string;
}

export class CommandRouter {
  private ledgerMCP: LedgerMCPClient;
  private koiMCP: KOIMCPClient;
  private templates: EmbedTemplates;
  private runtime: IAgentRuntime;

  // Command prefix
  private readonly PREFIX = "!";

  // Command registry
  private commands: Map<string, (args: string[], ctx: MessageContext) => Promise<any>>;

  constructor(
    ledgerMCP: LedgerMCPClient,
    koiMCP: KOIMCPClient,
    templates: EmbedTemplates,
    runtime: IAgentRuntime
  ) {
    this.ledgerMCP = ledgerMCP;
    this.koiMCP = koiMCP;
    this.templates = templates;
    this.runtime = runtime;

    this.commands = new Map([
      ["proposal", this.handleProposal.bind(this)],
      ["tally", this.handleTally.bind(this)],
      ["compare", this.handleCompare.bind(this)],
      ["history", this.handleHistory.bind(this)],
      ["params", this.handleParams.bind(this)],
      ["reboot-status", this.handleRebootStatus.bind(this)],
    ]);
  }

  async route(content: string, ctx: MessageContext): Promise<any | null> {
    const trimmed = content.trim();

    // Check for command prefix
    if (trimmed.startsWith(this.PREFIX)) {
      const parts = trimmed.slice(this.PREFIX.length).split(/\s+/);
      const command = parts[0].toLowerCase();
      const args = parts.slice(1);

      const handler = this.commands.get(command);
      if (handler) {
        return handler(args, ctx);
      }

      // Unknown command — suggest available commands
      return {
        embeds: [
          this.templates.helpEmbed(Array.from(this.commands.keys())),
        ],
      };
    }

    // Natural language query — check governance relevance
    const relevance = await this.assessGovernanceRelevance(trimmed);
    if (relevance >= 0.6) {
      return this.handleNaturalLanguage(trimmed, ctx);
    }

    // Not governance-related — do not respond
    return null;
  }

  private async handleProposal(
    args: string[],
    ctx: MessageContext
  ): Promise<any> {
    if (args.length === 0) {
      return {
        embeds: [
          this.templates.errorEmbed("Usage: `!proposal <id>` — e.g., `!proposal 73`"),
        ],
      };
    }

    const proposalId = parseInt(args[0], 10);
    if (isNaN(proposalId)) {
      return {
        embeds: [
          this.templates.errorEmbed("Invalid proposal ID. Use a number, e.g., `!proposal 73`"),
        ],
      };
    }

    try {
      const proposal = await this.ledgerMCP.getProposal(proposalId);
      const context = await this.koiMCP.search({
        query: proposal.content?.title || `proposal ${proposalId}`,
        intent: "general",
        limit: 5,
      });

      return {
        embeds: [this.templates.proposalAnalysisEmbed(proposal, context)],
        components: [this.templates.proposalButtons(proposalId)],
      };
    } catch (error) {
      return {
        embeds: [
          this.templates.errorEmbed(
            `Could not retrieve proposal #${proposalId}. It may not exist or the data source is temporarily unavailable.`
          ),
        ],
      };
    }
  }

  private async handleTally(
    args: string[],
    ctx: MessageContext
  ): Promise<any> {
    if (args.length === 0) {
      return {
        embeds: [
          this.templates.errorEmbed("Usage: `!tally <id>` — e.g., `!tally 73`"),
        ],
      };
    }

    const proposalId = parseInt(args[0], 10);
    if (isNaN(proposalId)) {
      return {
        embeds: [
          this.templates.errorEmbed("Invalid proposal ID. Use a number, e.g., `!tally 73`"),
        ],
      };
    }

    try {
      const proposal = await this.ledgerMCP.getProposal(proposalId);
      const tally = await this.ledgerMCP.call("get_governance_tally", {
        proposal_id: proposalId,
      });

      return {
        embeds: [this.templates.votingProgressEmbed(proposal, tally)],
      };
    } catch (error) {
      return {
        embeds: [
          this.templates.errorEmbed(
            `Could not retrieve tally for proposal #${proposalId}.`
          ),
        ],
      };
    }
  }

  private async handleCompare(
    args: string[],
    ctx: MessageContext
  ): Promise<any> {
    if (args.length < 2) {
      return {
        embeds: [
          this.templates.errorEmbed(
            "Usage: `!compare <id1> <id2>` — e.g., `!compare 71 73`"
          ),
        ],
      };
    }

    const id1 = parseInt(args[0], 10);
    const id2 = parseInt(args[1], 10);
    if (isNaN(id1) || isNaN(id2)) {
      return {
        embeds: [
          this.templates.errorEmbed("Both proposal IDs must be numbers."),
        ],
      };
    }

    try {
      const [proposal1, proposal2] = await Promise.all([
        this.ledgerMCP.getProposal(id1),
        this.ledgerMCP.getProposal(id2),
      ]);

      return {
        embeds: [this.templates.comparisonEmbed(proposal1, proposal2)],
      };
    } catch (error) {
      return {
        embeds: [
          this.templates.errorEmbed(
            `Could not retrieve one or both proposals (#${id1}, #${id2}).`
          ),
        ],
      };
    }
  }

  private async handleHistory(
    args: string[],
    ctx: MessageContext
  ): Promise<any> {
    const count = Math.min(parseInt(args[0], 10) || 5, 20);

    try {
      const proposals = await this.ledgerMCP.call("list_governance_proposals", {
        limit: count,
      });

      return {
        embeds: [this.templates.historyEmbed(proposals.proposals || [])],
      };
    } catch (error) {
      return {
        embeds: [
          this.templates.errorEmbed("Could not retrieve proposal history."),
        ],
      };
    }
  }

  private async handleParams(
    args: string[],
    ctx: MessageContext
  ): Promise<any> {
    const module = (args[0] || "gov").toLowerCase();
    const validModules = ["gov", "staking", "slashing", "mint", "supply", "distribution"];

    if (!validModules.includes(module)) {
      return {
        embeds: [
          this.templates.errorEmbed(
            `Unknown module: \`${module}\`. Valid modules: ${validModules.join(", ")}`
          ),
        ],
      };
    }

    try {
      const params = await this.ledgerMCP.call("get_module_params", {
        module: `x/${module}`,
      });

      return {
        embeds: [this.templates.paramsEmbed(module, params)],
      };
    } catch (error) {
      return {
        embeds: [
          this.templates.errorEmbed(
            `Could not retrieve parameters for module \`${module}\`.`
          ),
        ],
      };
    }
  }

  private async handleRebootStatus(
    args: string[],
    ctx: MessageContext
  ): Promise<any> {
    try {
      const states = await Promise.all([
        this.ledgerMCP.call("get_module_state", { module: "m012" }).catch(() => ({ state: "UNKNOWN" })),
        this.ledgerMCP.call("get_module_state", { module: "m013" }).catch(() => ({ state: "UNKNOWN" })),
        this.ledgerMCP.call("get_module_state", { module: "m014" }).catch(() => ({ state: "UNKNOWN" })),
        this.ledgerMCP.call("get_module_state", { module: "m015" }).catch(() => ({ state: "UNKNOWN" })),
      ]);

      return {
        embeds: [
          this.templates.rebootStatusEmbed({
            m012: states[0].state,
            m013: states[1].state,
            m014: states[2].state,
            m015: states[3].state,
          }),
        ],
      };
    } catch (error) {
      return {
        embeds: [
          this.templates.errorEmbed("Could not retrieve economic reboot status."),
        ],
      };
    }
  }

  private async handleNaturalLanguage(
    query: string,
    ctx: MessageContext
  ): Promise<any> {
    // Use ElizaOS LLM pipeline with governance context
    const governanceContext = await this.buildGovernanceContext();

    const response = await this.runtime.completion({
      context: `${governanceContext}\n\nUser question: ${query}`,
      stop: [],
    });

    return {
      embeds: [this.templates.naturalLanguageEmbed(query, response)],
    };
  }

  private async assessGovernanceRelevance(text: string): Promise<number> {
    const governanceKeywords = [
      "proposal", "vote", "voting", "governance", "quorum", "tally",
      "validator", "stake", "staking", "delegation", "parameter",
      "upgrade", "reboot", "supply", "burn", "mint", "fee",
      "m012", "m013", "m014", "m015", "poa", "proof of authority",
      "community pool", "deposit", "veto", "turnout",
    ];

    const lower = text.toLowerCase();
    const matches = governanceKeywords.filter((kw) => lower.includes(kw));
    const keywordScore = Math.min(matches.length / 3, 1.0);

    // Proposal number reference pattern
    const proposalRef = /(?:proposal|prop)\s*#?\d+/i.test(text);
    const refScore = proposalRef ? 0.8 : 0;

    return Math.max(keywordScore, refScore);
  }

  private async buildGovernanceContext(): Promise<string> {
    try {
      const activeProposals = await this.ledgerMCP.call(
        "list_governance_proposals",
        { status: "PROPOSAL_STATUS_VOTING_PERIOD", limit: 5 }
      );

      const params = await this.ledgerMCP.call("get_module_params", {
        module: "x/gov",
      });

      return `You are the Regen Governance Analyst (AGENT-002), an AI agent that provides objective governance analysis for Regen Network.

RULES:
- NEVER recommend how to vote
- NEVER endorse validators or candidates
- Present balanced analysis with data citations
- Disclose that you are an AI agent
- Cite data sources (Ledger MCP, KOI MCP)

CURRENT GOVERNANCE STATE:
Active proposals: ${JSON.stringify(activeProposals.proposals || [])}
Governance parameters: ${JSON.stringify(params)}`;
    } catch {
      return `You are the Regen Governance Analyst (AGENT-002). Respond to governance questions objectively. Data sources are temporarily limited.`;
    }
  }
}
```

### Rate Limiter

```typescript
// packages/plugin-discord-governance/src/util/rate-limiter.ts
interface RateLimitConfig {
  alertsPerHour: number;
  cooldownMinutes: number;
}

export class RateLimiter {
  private config: RateLimitConfig;
  private postTimestamps: number[] = [];
  private proposalCooldowns: Map<number, number> = new Map();

  constructor(config: RateLimitConfig) {
    this.config = config;
  }

  canPost(channel: string): boolean {
    const now = Date.now();
    const oneHourAgo = now - 60 * 60 * 1000;

    // Clean old timestamps
    this.postTimestamps = this.postTimestamps.filter((t) => t > oneHourAgo);

    return this.postTimestamps.length < this.config.alertsPerHour;
  }

  canPostForProposal(proposalId: number): boolean {
    const now = Date.now();
    const lastPost = this.proposalCooldowns.get(proposalId);

    if (!lastPost) return true;

    const cooldownMs = this.config.cooldownMinutes * 60 * 1000;
    return now - lastPost >= cooldownMs;
  }

  recordPost(channel: string, proposalId?: number): void {
    const now = Date.now();
    this.postTimestamps.push(now);

    if (proposalId !== undefined) {
      this.proposalCooldowns.set(proposalId, now);
    }
  }
}
```

### Governance Relevance Evaluator

```typescript
// packages/plugin-discord-governance/src/evaluators/governance-relevance.ts
import { Evaluator, IAgentRuntime, Memory, State } from "@elizaos/core";

export const governanceRelevanceEvaluator: Evaluator = {
  name: "GOVERNANCE_RELEVANCE",
  similes: ["CHECK_GOVERNANCE_TOPIC", "IS_GOVERNANCE_QUESTION"],
  description:
    "Evaluates whether a message is related to Regen Network governance, " +
    "proposals, voting, or economic reboot mechanisms.",

  validate: async (runtime: IAgentRuntime, message: Memory): Promise<boolean> => {
    // Only evaluate messages in the proposal-discussion channel
    const channelId = runtime.getSetting("DISCORD_CHANNEL_PROPOSAL_DISCUSSION");
    return message.roomId === channelId;
  },

  handler: async (
    runtime: IAgentRuntime,
    message: Memory,
    state?: State
  ): Promise<any> => {
    const text = (message.content?.text || "").toLowerCase();

    const governanceTerms = [
      "proposal", "vote", "voting", "governance", "quorum",
      "validator", "stake", "delegation", "parameter", "upgrade",
      "reboot", "supply", "burn", "mint", "fee", "poa",
      "m012", "m013", "m014", "m015", "community pool",
      "deposit", "veto", "turnout", "regen",
    ];

    const matchCount = governanceTerms.filter((term) =>
      text.includes(term)
    ).length;

    const score = Math.min(matchCount / 3, 1.0);

    return {
      relevant: score >= 0.6,
      score,
      matchedTerms: governanceTerms.filter((term) => text.includes(term)),
    };
  },

  examples: [
    {
      context: "User asks about a proposal",
      messages: [
        { user: "user1", content: { text: "What is proposal 73 about?" } },
      ],
      outcome: "relevant: true, score: 0.67",
    },
    {
      context: "User asks off-topic question",
      messages: [
        { user: "user1", content: { text: "What's the weather today?" } },
      ],
      outcome: "relevant: false, score: 0.0",
    },
  ],
};
```

### Governance State Provider

```typescript
// packages/plugin-discord-governance/src/providers/governance-state.ts
import { Provider, IAgentRuntime, Memory, State } from "@elizaos/core";
import { LedgerMCPClient } from "@regen/plugin-ledger-mcp";

export const governanceStateProvider: Provider = {
  name: "GOVERNANCE_STATE",
  description:
    "Provides current governance state including active proposals, " +
    "voting parameters, and recent governance activity.",

  get: async (
    runtime: IAgentRuntime,
    message: Memory,
    state?: State
  ): Promise<string> => {
    const ledgerMCP = new LedgerMCPClient({
      baseUrl: runtime.getSetting("LEDGER_MCP_URL") || "http://localhost:3001",
      apiKey: runtime.getSetting("LEDGER_MCP_API_KEY"),
    });

    try {
      const [activeProposals, govParams, stakingPool] = await Promise.all([
        ledgerMCP.call("list_governance_proposals", {
          status: "PROPOSAL_STATUS_VOTING_PERIOD",
          limit: 10,
        }),
        ledgerMCP.call("get_module_params", { module: "x/gov" }),
        ledgerMCP.call("get_staking_pool", {}),
      ]);

      return JSON.stringify({
        activeProposals: activeProposals.proposals || [],
        governanceParams: govParams,
        stakingPool: {
          bondedTokens: stakingPool.bonded_tokens,
          notBondedTokens: stakingPool.not_bonded_tokens,
        },
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      return JSON.stringify({
        error: "Governance state temporarily unavailable",
        timestamp: new Date().toISOString(),
      });
    }
  },
};
```

---

## Embed Templates

### Proposal Analysis Embed

```typescript
// packages/plugin-discord-governance/src/templates/embed-templates.ts (partial)
import {
  EmbedBuilder,
  ActionRowBuilder,
  ButtonBuilder,
  ButtonStyle,
  Colors,
} from "discord.js";

// Color constants
const COLORS = {
  INFO: 0x3498DB,        // Blue — informational
  SUCCESS: 0x2ECC71,     // Green — passed / positive
  WARNING: 0xF1C40F,     // Yellow — in progress / caution
  DANGER: 0xE74C3C,      // Red — failed / alert
  PURPLE: 0x9B59B6,      // Purple — economic reboot milestones
  NEUTRAL: 0x95A5A6,     // Gray — no-action / concluded
};

const FOOTER_TEXT =
  "Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP | " +
  "This is analysis, not a voting recommendation.";

const FOOTER_ICON = "https://regen.network/assets/agent-icon.png";

export class EmbedTemplates {
  proposalAnalysisEmbed(proposal: any, context: any): EmbedBuilder {
    const embed = new EmbedBuilder()
      .setTitle(
        `New Proposal #${proposal.id}: ${proposal.content?.title || "Untitled"}`
      )
      .setURL(`https://regen.network/governance/proposals/${proposal.id}`)
      .setColor(COLORS.INFO)
      .setDescription(
        "A new governance proposal has been submitted and is now in voting period."
      )
      .addFields(
        {
          name: "Category",
          value: this.classifyProposal(proposal),
          inline: true,
        },
        {
          name: "Proposer",
          value: `\`${this.truncateAddress(proposal.proposer)}\``,
          inline: true,
        },
        {
          name: "Deposit Status",
          value: this.formatDeposit(proposal),
          inline: true,
        },
        {
          name: "TL;DR",
          value: this.generateTLDR(proposal, context),
          inline: false,
        },
        {
          name: "Impact Assessment",
          value: this.generateImpactAssessment(proposal),
          inline: false,
        },
        {
          name: "Voting Period",
          value: `Ends <t:${Math.floor(new Date(proposal.voting_end_time).getTime() / 1000)}:R>`,
          inline: true,
        },
        {
          name: "Vote Now",
          value: `[Keplr](https://wallet.keplr.app/chains/regen/proposals/${proposal.id}) | [CLI](https://docs.regen.network/ledger/governance)`,
          inline: false,
        }
      )
      .setFooter({ text: FOOTER_TEXT, iconURL: FOOTER_ICON })
      .setTimestamp();

    return embed;
  }

  votingProgressEmbed(proposal: any, tally: any): EmbedBuilder {
    const total =
      Number(tally.yes || 0) +
      Number(tally.no || 0) +
      Number(tally.abstain || 0) +
      Number(tally.no_with_veto || 0);

    const pct = (val: number) =>
      total > 0 ? ((val / total) * 100).toFixed(1) : "0.0";
    const bar = (val: number) => {
      const filled = total > 0 ? Math.round((val / total) * 20) : 0;
      return "\u2588".repeat(filled) + "\u2591".repeat(20 - filled);
    };

    const tallyDisplay = [
      `Yes     ${bar(Number(tally.yes || 0))}  ${pct(Number(tally.yes || 0))}%`,
      `No      ${bar(Number(tally.no || 0))}  ${pct(Number(tally.no || 0))}%`,
      `Abstain ${bar(Number(tally.abstain || 0))}  ${pct(Number(tally.abstain || 0))}%`,
      `Veto    ${bar(Number(tally.no_with_veto || 0))}  ${pct(Number(tally.no_with_veto || 0))}%`,
    ].join("\n");

    return new EmbedBuilder()
      .setTitle(
        `Voting Update: Proposal #${proposal.id} — ${proposal.content?.title || "Untitled"}`
      )
      .setURL(`https://regen.network/governance/proposals/${proposal.id}`)
      .setColor(COLORS.WARNING)
      .addFields(
        {
          name: "Current Tally",
          value: `\`\`\`\n${tallyDisplay}\n\`\`\``,
          inline: false,
        },
        {
          name: "Time Remaining",
          value: `<t:${Math.floor(new Date(proposal.voting_end_time).getTime() / 1000)}:R>`,
          inline: true,
        }
      )
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | Data: Ledger MCP | Projections are statistical estimates, not guarantees.",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }

  postVoteEmbed(proposal: any, tally: any): EmbedBuilder {
    const passed = Number(tally.yes || 0) > Number(tally.no || 0);
    const status = passed ? "PASSED" : "FAILED";
    const color = passed ? COLORS.SUCCESS : COLORS.DANGER;

    return new EmbedBuilder()
      .setTitle(
        `Vote Concluded: Proposal #${proposal.id} — ${status}`
      )
      .setURL(`https://regen.network/governance/proposals/${proposal.id}`)
      .setColor(color)
      .addFields(
        {
          name: "Final Results",
          value: this.formatFinalTally(tally),
          inline: false,
        },
        {
          name: "Outcome",
          value: status,
          inline: true,
        }
      )
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP | Full post-vote report archived to KOI.",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }

  quorumRiskEmbed(proposal: any, tally: any): EmbedBuilder {
    return new EmbedBuilder()
      .setTitle(`Quorum Risk Alert: Proposal #${proposal.id}`)
      .setURL(`https://regen.network/governance/proposals/${proposal.id}`)
      .setColor(COLORS.DANGER)
      .setDescription(
        "Current turnout trends suggest this proposal may NOT reach quorum before the voting period ends."
      )
      .addFields(
        {
          name: "Time Remaining",
          value: `<t:${Math.floor(new Date(proposal.voting_end_time).getTime() / 1000)}:R>`,
          inline: true,
        },
        {
          name: "What This Means",
          value:
            "If quorum is not met, the proposal's deposit will be returned and the proposal will not pass regardless of vote distribution.",
          inline: false,
        }
      )
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | Data: Ledger MCP | This is a factual alert, not a call to vote in any direction.",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }

  rebootMilestoneEmbed(
    module: string,
    previousState: string,
    newState: string
  ): EmbedBuilder {
    const moduleNames: Record<string, string> = {
      m012: "M012 — Dynamic Supply",
      m013: "M013 — Fee Routing",
      m014: "M014 — PoA Governance",
      m015: "M015 — Contribution Rewards",
    };

    return new EmbedBuilder()
      .setTitle(
        `Economic Reboot Milestone: ${moduleNames[module] || module} Entered ${newState} Phase`
      )
      .setColor(COLORS.PURPLE)
      .addFields(
        { name: "Module", value: moduleNames[module] || module, inline: true },
        { name: "Previous State", value: previousState, inline: true },
        { name: "New State", value: newState, inline: true }
      )
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | Data: Ledger MCP | Economic reboot status is informational only.",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }

  rebootStatusEmbed(states: Record<string, string>): EmbedBuilder {
    const stateIcon = (state: string): string => {
      switch (state.toUpperCase()) {
        case "ACTIVE":
        case "EQUILIBRIUM":
          return "[ACTIVE]";
        case "TRANSITION":
        case "DYNAMIC":
          return "[TRANSITION]";
        case "PENDING":
        case "INACTIVE":
          return "[PENDING]";
        default:
          return "[UNKNOWN]";
      }
    };

    const statusLines = [
      `M013 Fee Routing         ${stateIcon(states.m013)}`,
      `M014 PoA Governance      ${stateIcon(states.m014)}`,
      `M012 Dynamic Supply      ${stateIcon(states.m012)}`,
      `M015 Contribution Rwds   ${stateIcon(states.m015)}`,
    ].join("\n");

    return new EmbedBuilder()
      .setTitle("Economic Reboot Status")
      .setColor(COLORS.PURPLE)
      .addFields({
        name: "Module States",
        value: `\`\`\`\n${statusLines}\n\`\`\``,
        inline: false,
      })
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | Data: Ledger MCP",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }

  errorEmbed(message: string): EmbedBuilder {
    return new EmbedBuilder()
      .setTitle("Error")
      .setColor(COLORS.DANGER)
      .setDescription(message)
      .setFooter({
        text: "Regen Governance Analyst (AI Agent)",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }

  helpEmbed(commands: string[]): EmbedBuilder {
    const commandList = commands
      .map((cmd) => `\`!${cmd}\``)
      .join(", ");

    return new EmbedBuilder()
      .setTitle("Regen Governance Analyst — Commands")
      .setColor(COLORS.INFO)
      .setDescription(
        `Available commands: ${commandList}\n\nYou can also ask governance questions in plain language.`
      )
      .addFields(
        { name: "`!proposal <id>`", value: "Full proposal analysis", inline: true },
        { name: "`!tally <id>`", value: "Current voting status", inline: true },
        { name: "`!compare <id1> <id2>`", value: "Compare proposals", inline: true },
        { name: "`!history [count]`", value: "Recent proposals", inline: true },
        { name: "`!params <module>`", value: "Governance parameters", inline: true },
        { name: "`!reboot-status`", value: "Economic reboot state", inline: true }
      )
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | I provide analysis, not voting recommendations.",
        iconURL: FOOTER_ICON,
      });
  }

  naturalLanguageEmbed(query: string, response: string): EmbedBuilder {
    return new EmbedBuilder()
      .setColor(COLORS.INFO)
      .setDescription(response)
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP | AI-generated analysis — verify with on-chain data.",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }

  proposalButtons(proposalId: number): ActionRowBuilder<ButtonBuilder> {
    return new ActionRowBuilder<ButtonBuilder>().addComponents(
      new ButtonBuilder()
        .setStyle(ButtonStyle.Link)
        .setLabel("Full Analysis")
        .setURL(
          `https://regen.network/governance/proposals/${proposalId}/analysis`
        ),
      new ButtonBuilder()
        .setStyle(ButtonStyle.Link)
        .setLabel("Forum Discussion")
        .setURL(
          `https://forum.regen.network/t/proposal-${proposalId}-discussion`
        ),
      new ButtonBuilder()
        .setStyle(ButtonStyle.Link)
        .setLabel("Vote on Keplr")
        .setURL(
          `https://wallet.keplr.app/chains/regen/proposals/${proposalId}`
        )
    );
  }

  // --- Helper Methods ---

  private classifyProposal(proposal: any): string {
    const title = (proposal.content?.title || "").toLowerCase();
    if (title.includes("upgrade") || title.includes("software"))
      return "Software Upgrade";
    if (title.includes("parameter") || title.includes("param"))
      return "Parameter Change";
    if (title.includes("spend") || title.includes("community pool"))
      return "Community Pool Spend";
    if (title.includes("text") || title.includes("signal"))
      return "Text/Signaling";
    if (title.includes("class") || title.includes("credit"))
      return "Registry";
    return "General";
  }

  private truncateAddress(address: string): string {
    if (!address || address.length < 16) return address || "unknown";
    return `${address.slice(0, 10)}...${address.slice(-4)}`;
  }

  private formatDeposit(proposal: any): string {
    const deposit = proposal.total_deposit?.[0];
    if (!deposit) return "Unknown";
    const amount = (Number(deposit.amount) / 1_000_000).toLocaleString();
    return `${amount} REGEN`;
  }

  private generateTLDR(proposal: any, context: any): string {
    // In production, this would use the LLM via ElizaOS runtime
    return proposal.content?.description?.slice(0, 300) || "No description available.";
  }

  private generateImpactAssessment(proposal: any): string {
    // Placeholder — full implementation uses LLM with WF-GA-01 orient phase
    return (
      "**Economic**: Pending analysis\n" +
      "**Technical**: Pending analysis\n" +
      "**Governance**: Pending analysis"
    );
  }

  private formatFinalTally(tally: any): string {
    const total =
      Number(tally.yes || 0) +
      Number(tally.no || 0) +
      Number(tally.abstain || 0) +
      Number(tally.no_with_veto || 0);

    const pct = (val: number) =>
      total > 0 ? ((val / total) * 100).toFixed(1) : "0.0";
    const bar = (val: number) => {
      const filled = total > 0 ? Math.round((val / total) * 20) : 0;
      return "\u2588".repeat(filled) + "\u2591".repeat(20 - filled);
    };

    const lines = [
      `Yes     ${bar(Number(tally.yes || 0))}  ${pct(Number(tally.yes || 0))}%`,
      `No      ${bar(Number(tally.no || 0))}  ${pct(Number(tally.no || 0))}%`,
      `Abstain ${bar(Number(tally.abstain || 0))}  ${pct(Number(tally.abstain || 0))}%`,
      `Veto    ${bar(Number(tally.no_with_veto || 0))}  ${pct(Number(tally.no_with_veto || 0))}%`,
    ].join("\n");

    return `\`\`\`\n${lines}\n\`\`\``;
  }

  dailyRebootEmbed(supplyState: any, feeRevenue: any): EmbedBuilder {
    return new EmbedBuilder()
      .setTitle(
        `Daily Economic Reboot Report — ${new Date().toISOString().split("T")[0]}`
      )
      .setColor(COLORS.SUCCESS)
      .addFields(
        {
          name: "Fee Revenue (24h)",
          value: `${feeRevenue.total || "N/A"} REGEN across ${feeRevenue.txCount || "N/A"} transactions`,
          inline: true,
        },
        {
          name: "Burned (24h)",
          value: `${feeRevenue.burned || "N/A"} REGEN`,
          inline: true,
        },
        {
          name: "Minted (24h)",
          value: `${supplyState.minted || "N/A"} REGEN`,
          inline: true,
        },
        {
          name: "Circulating Supply",
          value: `${supplyState.circulating || "N/A"} / ${supplyState.cap || "221,000,000"} REGEN`,
          inline: false,
        }
      )
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | Data: Ledger MCP | Auto-posted daily at 08:00 UTC",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }

  weeklyRebootEmbed(
    validatorStats: any,
    rewardDistribution: any,
    supplyState: any,
    feeRevenue: any
  ): EmbedBuilder {
    return new EmbedBuilder()
      .setTitle(
        `Weekly Economic Reboot Report — Week of ${new Date().toISOString().split("T")[0]}`
      )
      .setColor(COLORS.PURPLE)
      .addFields(
        {
          name: "Validator Performance (M014)",
          value: `Active authority validators: ${validatorStats.activeCount || "N/A"}/${validatorStats.totalCount || "N/A"}\nAverage uptime: ${validatorStats.avgUptime || "N/A"}%\nSlashing events: ${validatorStats.slashingEvents || 0}`,
          inline: false,
        },
        {
          name: "Fee Revenue (M013, 7-day)",
          value: `${feeRevenue.total || "N/A"} REGEN across ${feeRevenue.txCount || "N/A"} transactions`,
          inline: false,
        },
        {
          name: "Reward Distribution (M015)",
          value: `Total distributed: ${rewardDistribution.total || "N/A"} REGEN`,
          inline: false,
        },
        {
          name: "Supply State (M012)",
          value: `Circulating: ${supplyState.circulating || "N/A"} REGEN\nCap utilization: ${supplyState.capUtilization || "N/A"}%`,
          inline: false,
        }
      )
      .setFooter({
        text: "Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP | Auto-posted weekly on Mondays at 08:00 UTC",
        iconURL: FOOTER_ICON,
      })
      .setTimestamp();
  }
}
```

---

## Guardrails

### Content Policy

| Rule | Implementation | Enforcement |
|------|---------------|-------------|
| Agent NEVER recommends how to vote | System prompt + output filter | LLM system prompt includes explicit prohibition; post-generation regex scan for recommendation phrases ("you should vote", "vote yes/no", "I recommend") |
| Agent NEVER endorses validators or candidates | System prompt + output filter | Blocked phrases list; mentions of specific validators are factual only (e.g., "Validator X voted Yes") |
| AI disclosure in every embed | Embed footer template | Hardcoded in `FOOTER_TEXT` constant; cannot be overridden by LLM output |
| Data source citation | Embed footer + field-level attribution | Every embed includes source in footer; analysis fields reference Ledger MCP or KOI MCP |
| Layer 1 informational only | No transaction-signing capability | Plugin has no access to private keys; `@regen/plugin-discord-governance` does not import TX Builder |

### Rate Limits

| Limit | Value | Scope | Rationale |
|-------|-------|-------|-----------|
| Max posts per hour in `#governance-alerts` | 10 | Channel-wide | Prevent alert fatigue during high-activity periods |
| Per-proposal cooldown | 30 minutes | Per proposal ID | Prevent duplicate/redundant updates for same proposal |
| Quorum risk alert override | Bypasses per-proposal cooldown, respects hourly cap | Per proposal | Critical alerts warrant shorter cooldown |
| Natural language response | 1 response per user per 60 seconds | Per user | Prevent abuse of LLM-powered responses |
| Error responses | Not counted against rate limits | N/A | Error messages should always be delivered |

### Output Filtering

```typescript
// packages/plugin-discord-governance/src/util/output-filter.ts
const BLOCKED_PATTERNS: RegExp[] = [
  /you should vote/i,
  /vote (yes|no|abstain|veto)/i,
  /I recommend (voting|that you)/i,
  /the (best|right|correct) vote is/i,
  /you (must|need to|have to) vote/i,
  /support(ing)? this proposal/i,
  /reject(ing)? this proposal/i,
  /I (endorse|support|recommend)/i,
];

export function filterOutput(text: string): string {
  for (const pattern of BLOCKED_PATTERNS) {
    if (pattern.test(text)) {
      return (
        text.replace(pattern, "[REDACTED — agent does not make voting recommendations]") +
        "\n\n_Note: A recommendation phrase was detected and removed. " +
        "This agent provides analysis only, not voting advice._"
      );
    }
  }
  return text;
}
```

### Error Handling and Graceful Degradation

| Failure Mode | Behavior | User-Facing Message |
|-------------|----------|-------------------|
| Ledger MCP unavailable | Cached data used if <5 min old; otherwise degrade to "data unavailable" | "On-chain data is temporarily unavailable. Showing cached data from [timestamp]." |
| KOI MCP unavailable | Omit context/sentiment fields; core tally data still shown | Analysis proceeds without historical context fields |
| Discord API rate limit (429) | Exponential backoff with jitter; queue messages | No user-facing impact (messages delivered after backoff) |
| LLM completion failure | Fall back to template-based response for commands; skip NL response | "I'm unable to generate a detailed analysis right now. Here's the raw data:" |
| Invalid proposal ID | Return error embed with usage hint | "Could not retrieve proposal #[id]. It may not exist or the data source is temporarily unavailable." |

---

## Interaction Examples

### Example 1: User Asks About a Proposal

```
[#proposal-discussion]

@alice: !proposal 73

@RegenGovAnalyst:
┌─────────────────────────────────────────────────────────────────────┐
│  Proposal #73 Analysis                                                │
│  Enable Dynamic Supply Module (M012)                                  │
│                                                                       │
│  Category: Parameter Change              Proposer: regen1qx8...f4m2  │
│  Deposit: 5,000 REGEN (Met)              Status: Voting Period       │
│                                                                       │
│  TL;DR                                                                │
│  Activates the M012 fixed-cap dynamic supply mechanism, replacing    │
│  the current inflationary x/mint module with algorithmic mint/burn   │
│  cycles tied to ecological activity. Sets hard cap at 221M REGEN.    │
│                                                                       │
│  Impact Assessment                                                    │
│  Economic: High — fundamentally changes supply model from            │
│    inflationary to deflationary/equilibrium                          │
│  Technical: Medium — requires coordinated chain upgrade              │
│  Governance: High — Layer 4 constitutional change                    │
│                                                                       │
│  Community Sentiment (from forum)                                     │
│  23 posts, 89% supportive. Key discussion points:                    │
│  - Hard cap level (221M vs current 224M supply)                      │
│  - Ecological multiplier oracle dependency                           │
│  - Transition timeline                                               │
│                                                                       │
│  Historical Precedent                                                 │
│  No direct precedent on Regen. Comparable: Cosmos Hub Prop #848      │
│  (inflation reduction, passed 72.3%, 52.1% turnout)                  │
│                                                                       │
│  Voting Period: Ends in 4 days                                       │
│  [Full Analysis] [Forum Discussion] [Vote on Keplr]                  │
│                                                                       │
│  Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP    │
│  This is analysis, not a voting recommendation.                      │
└─────────────────────────────────────────────────────────────────────┘

@bob: What are the risks of this proposal?

@RegenGovAnalyst:
Proposal #73 (M012 Dynamic Supply) has several risk dimensions worth
considering:

1. **Transition Risk**: The switch from inflationary to dynamic supply
   requires M013 (Fee Routing) and M014 (PoA) to be active first. If
   either has issues, M012 activation could be delayed.

2. **Parameter Sensitivity**: The base regrowth rate (initially 2%) and
   ecological multiplier significantly affect supply dynamics. The
   community will need to monitor and potentially adjust these via
   governance.

3. **Immediate Scarcity**: If the hard cap (221M) is set below current
   total supply (~224M), existing tokens above the cap cannot be minted
   back once burned. This is an open question (OQ-M012-1) flagged in
   the mechanism spec.

4. **Oracle Dependency**: The ecological multiplier requires a reliable
   oracle for CO2/ecological data. The v0 deployment disables this
   (multiplier = 1.0) until a suitable oracle exists.

These are factual risk factors from the mechanism specification and
community discussion. Each voter should weigh them according to their
own assessment.

_Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP |
This is analysis, not a voting recommendation._
```

### Example 2: Quorum Risk Alert and Community Response

```
[#governance-alerts]

@RegenGovAnalyst:
┌─────────────────────────────────────────────────────────────────────┐
│  ⚠ QUORUM RISK ALERT: Proposal #73                                  │
│                                                                       │
│  Current turnout trends suggest this proposal may NOT reach quorum   │
│  before the voting period ends.                                      │
│                                                                       │
│  Current Turnout:    24.1% of staked REGEN                           │
│  Required Quorum:    33.4%                                           │
│  Gap:                9.3% (~12.4M REGEN)                             │
│  Time Remaining:     41h 23m                                         │
│                                                                       │
│  Projected Turnout:  29.7% (linear) / 31.2% (historical pattern)    │
│  Quorum Probability: 38%                                             │
│                                                                       │
│  If quorum is not met, the deposit (5,000 REGEN) will be returned   │
│  and the proposal will not pass regardless of vote distribution.     │
│                                                                       │
│  Regen Governance Analyst (AI Agent) | Data: Ledger MCP              │
│  This is a factual alert, not a call to vote in any direction.       │
└─────────────────────────────────────────────────────────────────────┘

[#proposal-discussion]

@carol: Wow, turnout is low. What's the historical average?

@RegenGovAnalyst:
Based on 61+ historical proposals on Regen:

- **Average final turnout**: 38.2%
- **Median final turnout**: 35.7%
- **Parameter change proposals** (like #73): 41.3% average
- **Lowest turnout (passed)**: 34.1% (Proposal #47)
- **Proposals that failed quorum**: 3 out of 61 (4.9%)

The October 2025 governance findings (documented in [PR #33 — Community Background](../docs/community-background.md)) noted that 65% of voting power was concentrated among top validators, and the average turnout of 25% (measured at that time) has since improved slightly.

Note: A significant portion of votes typically arrive in the final 48
hours. Historical data shows 40-60% of total votes are cast in the
last two days of the voting period.

_Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP |
This is analysis, not a voting recommendation._
```

### Example 3: Economic Reboot Status Query

```
[#proposal-discussion]

@dave: !reboot-status

@RegenGovAnalyst:
┌─────────────────────────────────────────────────────────────────────┐
│  Economic Reboot Status                                               │
│                                                                       │
│  M013 Fee Routing         [ACTIVE]      ████████████████████  Done   │
│  M014 PoA Governance      [TRANSITION]  ██████████░░░░░░░░░░  50%   │
│  M012 Dynamic Supply      [PENDING]     ░░░░░░░░░░░░░░░░░░░░  Blocked│
│  M015 Contribution Rwds   [PENDING]     ░░░░░░░░░░░░░░░░░░░░  Blocked│
│                                                                       │
│  M013 (Fee Routing): Active since 2026-02-15. Value-based %          │
│  fees operational. 40% burn share routing to supply contraction.     │
│                                                                       │
│  M014 (PoA Governance): In transition — PoS validators and PoA      │
│  authority validators running in parallel. 21/25 authority           │
│  validators active. Supply module uses max(staking, stability)       │
│  multiplier during this phase.                                        │
│                                                                       │
│  M012 (Dynamic Supply): Blocked on M013 + M014 both being active.   │
│  Proposal #73 is currently in voting to activate this module.        │
│                                                                       │
│  M015 (Contribution Rewards): Blocked on M014 reaching ACTIVE       │
│  state. Will replace passive staking rewards with activity-based     │
│  distribution.                                                        │
│                                                                       │
│  Regen Governance Analyst (AI Agent) | Data: Ledger MCP              │
│  Economic reboot status is informational only.                       │
└─────────────────────────────────────────────────────────────────────┘

@dave: When is M014 expected to finish transitioning?

@RegenGovAnalyst:
The M014 transition timeline depends on several governance-controlled
milestones:

1. **Authority validator onboarding**: Currently 21 of 25 target
   validators are active. The remaining 4 positions are in application
   review.

2. **PoS wind-down**: Once the full authority set is active and stable
   for a governance-defined period, a follow-up proposal can advance
   M014 from TRANSITION to ACTIVE state.

3. **Community governance vote**: The state transition requires a
   governance proposal to pass (similar to Proposal #73 for M012).

There is no hard deadline — the transition proceeds based on community
readiness and validator set health. The forum discussion at
forum.regen.network/t/regen-network-proof-of-authority-consensus-rfc/70
tracks the latest timeline estimates.

_Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP |
This is analysis, not a voting recommendation._
```

### Example 4: Natural Language Governance Question

```
[#proposal-discussion]

@eve: Has voter participation improved since the October audit?

@RegenGovAnalyst:
Based on available data comparing the October 2025 audit baseline to
current governance activity:

**October 2025 Audit Findings:**
- Average voter turnout: ~25%
- Voting power concentration: 65% among top validators
- Community engagement: moderate (forum activity declining)

**Post-Audit Trends (Nov 2025 — Mar 2026):**
- Average turnout for proposals #67-#72: 39.4% (+14.4pp improvement)
- Proposal #69 (Add NCT Credit Class) saw 44.7% turnout — highest
  in 6 months
- Delegator override rate increased from 7.2% to 12.3%, suggesting
  more independent voter participation
- Voting power concentration: still ~62% among top 10 validators
  (marginal improvement)

**Contributing Factors:**
- Economic reboot proposals generating community interest
- Governance Analyst bot (this agent) providing real-time updates
  since January 2026
- Forum.regen.network activity up 34% since economic reboot roadmap
  publication

Turnout is improving but voting power concentration remains a
structural issue that M014 (PoA Governance) aims to address.

_Regen Governance Analyst (AI Agent) | Data: Ledger MCP, KOI MCP |
This is analysis, not a voting recommendation._
```

---

## Configuration Reference

### Environment Variables

```bash
# Discord
DISCORD_BOT_TOKEN=<discord-bot-token>
DISCORD_GUILD_ID=<regen-community-guild-id>
DISCORD_CHANNEL_GOVERNANCE_ALERTS=<channel-id>
DISCORD_CHANNEL_PROPOSAL_DISCUSSION=<channel-id>
DISCORD_CHANNEL_REBOOT_STATUS=<channel-id>

# Rate Limits
DISCORD_ALERTS_PER_HOUR=10
DISCORD_COOLDOWN_MINUTES=30

# MCP Servers
LEDGER_MCP_URL=http://localhost:3001
LEDGER_MCP_API_KEY=<api-key>
KOI_MCP_URL=http://localhost:3002
KOI_MCP_API_KEY=<api-key>

# Agent
ANTHROPIC_API_KEY=<anthropic-api-key>
```

### Character Configuration Extension

The Discord governance plugin extends the existing AGENT-002 character definition from Phase 3.2:

```typescript
// characters/governance-analyst.ts (updated plugins array)
plugins: [
  "@elizaos/plugin-knowledge",
  "@regen/plugin-ledger-mcp",
  "@regen/plugin-koi-mcp",
  "@regen/plugin-social",
  "@regen/plugin-discord-governance",  // NEW: Discord governance integration
],
```

---

## Summary

This specification defines a complete Discord integration for AGENT-002 (Governance Analyst) that:

1. **Delivers automated governance intelligence** via `#governance-alerts` — new proposal analysis (WF-GA-01), voting progress updates (WF-GA-02), quorum risk alerts, post-vote reports (WF-GA-03), and economic reboot milestones (M012/M013/M014/M015).

2. **Enables interactive community engagement** via `#proposal-discussion` — six structured commands (`!proposal`, `!tally`, `!compare`, `!history`, `!params`, `!reboot-status`) plus natural language governance Q&A powered by the ElizaOS LLM pipeline.

3. **Tracks economic reboot progress** via `#economic-reboot-status` — daily fee revenue and supply metrics, weekly validator performance and reward distribution summaries, and immediate phase transition alerts.

4. **Enforces strict guardrails** — the agent never recommends how to vote, never endorses validators, discloses its AI identity in every message, cites data sources, and operates under rate limits (10 alerts/hour, 30-minute per-proposal cooldown).

5. **Degrades gracefully** — when MCP servers are unavailable, the bot serves cached data with timestamps, omits unavailable fields, and clearly communicates data limitations to the community.

The plugin (`@regen/plugin-discord-governance`) integrates with the existing ElizaOS runtime and extends AGENT-002's character configuration from Phase 3.2, building on `@regen/plugin-social` and the Ledger/KOI MCP clients defined in the agent implementation guide.
