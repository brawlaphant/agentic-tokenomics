/**
 * @regen/plugin-koi-mcp
 *
 * ElizaOS plugin providing Regen KOI knowledge graph access via MCP.
 * Enables semantic search, entity resolution, SPARQL queries, and
 * code graph navigation.
 *
 * Exports both:
 * - Raw functions for standalone mode (getKnowledgeContext, etc.)
 * - ElizaOS plugin object (koiMcpPlugin) for runtime registration
 *
 * Based on phase-3/3.2-agent-implementation.md §@regen/plugin-koi-mcp.
 */

import type { KOIMCPClient, SearchIntent } from "./client.js";
import {
  getKnowledgeContext,
  formatKnowledgeContext,
} from "./providers/knowledge-context.js";

// Re-export everything for direct imports
export { KOIMCPClient, type KOIMCPConfig, type SearchIntent } from "./client.js";
export {
  getKnowledgeContext,
  formatKnowledgeContext,
  type KnowledgeResult,
} from "./providers/knowledge-context.js";

// ---------------------------------------------------------------------------
// ElizaOS Plugin Object
// ---------------------------------------------------------------------------

/** Shared client instance, set via _setClient before runtime.initialize(). */
let _client: KOIMCPClient | null = null;

function getClient(): KOIMCPClient {
  if (!_client) {
    throw new Error(
      "KOIMCPClient not initialized. Call koiMcpPlugin._setClient() before runtime.initialize()."
    );
  }
  return _client;
}

/**
 * ElizaOS provider: knowledge-context
 *
 * Searches the KOI knowledge graph for context relevant to the current
 * message and injects it into the agent's context window. Grounds all
 * agent analysis in Regen's institutional knowledge.
 */
const knowledgeContextProvider = {
  name: "KNOWLEDGE_CONTEXT",
  description:
    "Relevant knowledge from the Regen KOI knowledge graph -- methodology docs, forum discussions, historical context.",

  get: async (
    _runtime: any,
    message: any,
    _state: any
  ): Promise<string> => {
    // Extract query text from the message
    const text =
      typeof message?.content === "string"
        ? message.content
        : message?.content?.text ?? "regen governance";

    // Detect intent from message content
    let intent: SearchIntent = "general";
    if (/how\s+to|setup|install|configure/i.test(text)) {
      intent = "technical_howto";
    } else if (/who|team|contributor|member/i.test(text)) {
      intent = "person_activity";
    }

    try {
      const results = await getKnowledgeContext(
        getClient(),
        text,
        intent,
        5
      );
      return formatKnowledgeContext(results);
    } catch (err) {
      return `[Knowledge context unavailable: ${(err as Error).message}]`;
    }
  },
};

/**
 * The ElizaOS plugin export.
 *
 * Usage:
 *   import { koiMcpPlugin } from "@regen/plugin-koi-mcp";
 *   koiMcpPlugin._setClient(myKoiClient);
 *   // then pass to AgentRuntime plugins array
 */
export const koiMcpPlugin = {
  name: "@regen/plugin-koi-mcp",
  description:
    "Provides Regen KOI knowledge graph access via MCP -- semantic search, entity resolution, SPARQL, code graph.",
  actions: [],
  providers: [knowledgeContextProvider],
  evaluators: [],
  services: [],

  /** Inject the MCP client before runtime initialization. */
  _setClient(client: KOIMCPClient) {
    _client = client;
  },
};
