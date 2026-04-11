/**
 * @regen/plugin-ledger-mcp
 *
 * ElizaOS plugin providing Regen Ledger on-chain data access via MCP.
 * Registers actions, providers, and evaluators for interacting with
 * the Regen Ledger blockchain.
 *
 * Exports both:
 * - Raw functions for standalone mode (analyzeProposal, getLedgerState, etc.)
 * - ElizaOS plugin object (ledgerMcpPlugin) for runtime registration
 *
 * Based on phase-3/3.2-agent-implementation.md §@regen/plugin-ledger-mcp.
 */

import type { LedgerMCPClient } from "./client.js";
import {
  analyzeProposal,
  formatProposalAnalysis,
  extractProposalId,
} from "./actions/analyze-proposal.js";
import {
  getLedgerState,
  formatLedgerState,
} from "./providers/ledger-state.js";

// Re-export everything for direct imports
export { LedgerMCPClient, type LedgerMCPConfig } from "./client.js";
export {
  analyzeProposal,
  formatProposalAnalysis,
  extractProposalId,
  type ProposalAnalysis,
} from "./actions/analyze-proposal.js";
export {
  getLedgerState,
  formatLedgerState,
  type LedgerStateSnapshot,
} from "./providers/ledger-state.js";

// ---------------------------------------------------------------------------
// ElizaOS Plugin Object
//
// Conforms to the ElizaOS Plugin interface. Actions and providers wrap the
// existing pure functions above so they work within the ElizaOS runtime.
// ---------------------------------------------------------------------------

/** Shared client instance, set via _setClient before runtime.initialize(). */
let _client: LedgerMCPClient | null = null;

function getClient(): LedgerMCPClient {
  if (!_client) {
    throw new Error(
      "LedgerMCPClient not initialized. Call ledgerMcpPlugin._setClient() before runtime.initialize()."
    );
  }
  return _client;
}

/**
 * ElizaOS action: analyze-proposal
 *
 * Validates that the message contains a proposal reference,
 * fetches it from chain, and returns structured analysis.
 */
const analyzeProposalAction = {
  name: "ANALYZE_PROPOSAL",
  description:
    "Fetch a Regen governance proposal by ID and return a structured analysis with voting tallies and impact assessment.",
  similes: [
    "SUMMARIZE_PROPOSAL",
    "GET_PROPOSAL",
    "PROPOSAL_ANALYSIS",
    "CHECK_PROPOSAL",
  ],

  validate: async (_runtime: any, message: any): Promise<boolean> => {
    const text =
      typeof message.content === "string"
        ? message.content
        : message.content?.text ?? "";
    return extractProposalId(text) !== null;
  },

  handler: async (
    _runtime: any,
    message: any,
    _state: any,
    _options: any,
    callback: (response: { text: string }) => void
  ): Promise<void> => {
    const text =
      typeof message.content === "string"
        ? message.content
        : message.content?.text ?? "";
    const proposalId = extractProposalId(text);

    if (!proposalId) {
      callback({
        text: "Could not extract a proposal ID from that message. Try: 'Analyze proposal #62'",
      });
      return;
    }

    try {
      const analysis = await analyzeProposal(getClient(), proposalId);
      if (!analysis) {
        callback({
          text: `Proposal #${proposalId} not found on chain.`,
        });
        return;
      }
      callback({ text: formatProposalAnalysis(analysis) });
    } catch (err) {
      callback({
        text: `Failed to analyze proposal #${proposalId}: ${(err as Error).message}`,
      });
    }
  },

  examples: [
    [
      {
        user: "{{user1}}",
        content: { text: "Analyze proposal #62" },
      },
      {
        user: "{{agentName}}",
        content: {
          text: "## Proposal #62 Analysis\n\n**Title**: Enable IBC Transfer Memo Field...",
          action: "ANALYZE_PROPOSAL",
        },
      },
    ],
  ],
};

/**
 * ElizaOS provider: ledger-state
 *
 * Injects current Regen Ledger state (active proposals, credit classes,
 * REGEN supply) into the agent's context on every message cycle.
 */
const ledgerStateProvider = {
  name: "LEDGER_STATE",
  description:
    "Current Regen Ledger state: active proposals, credit class count, total REGEN supply.",

  get: async (
    _runtime: any,
    _message: any,
    _state: any
  ): Promise<string> => {
    try {
      const state = await getLedgerState(getClient());
      return formatLedgerState(state);
    } catch (err) {
      return `[Ledger state unavailable: ${(err as Error).message}]`;
    }
  },
};

/**
 * The ElizaOS plugin export.
 *
 * Usage:
 *   import { ledgerMcpPlugin } from "@regen/plugin-ledger-mcp";
 *   ledgerMcpPlugin._setClient(myLedgerClient);
 *   // then pass to AgentRuntime plugins array
 */
export const ledgerMcpPlugin = {
  name: "@regen/plugin-ledger-mcp",
  description:
    "Provides Regen Ledger on-chain data access via MCP -- governance proposals, credit classes, validators, and supply.",
  actions: [analyzeProposalAction],
  providers: [ledgerStateProvider],
  evaluators: [],
  services: [],

  /** Inject the MCP client before runtime initialization. */
  _setClient(client: LedgerMCPClient) {
    _client = client;
  },
};
