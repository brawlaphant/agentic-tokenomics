import { config } from "./config.js";
import type {
  Validator,
  SigningInfo,
  SlashingParams,
  StakingPool,
  Proposal,
  Vote,
} from "./types.js";

/**
 * Regen Ledger LCD (REST) client — staking, slashing, gov endpoints.
 *
 * Talks directly to a Cosmos LCD endpoint — no MCP dependency. Matches
 * the pattern used by AGENT-002 and AGENT-003. When Ledger MCP becomes
 * available, this can be swapped behind the same interface.
 */
export class LedgerClient {
  private baseUrl: string;

  constructor(baseUrl?: string) {
    this.baseUrl = (baseUrl || config.lcdUrl).replace(/\/$/, "");
  }

  // ── Staking ──────────────────────────────────────────────────

  async getValidators(status = "BOND_STATUS_BONDED"): Promise<Validator[]> {
    const params = new URLSearchParams();
    params.set("status", status);
    params.set("pagination.limit", "500");

    const data = await this.get(
      `/cosmos/staking/v1beta1/validators?${params.toString()}`
    );
    return (data.validators || []) as Validator[];
  }

  async getValidator(operatorAddress: string): Promise<Validator | null> {
    try {
      const data = await this.get(
        `/cosmos/staking/v1beta1/validators/${operatorAddress}`
      );
      return (data.validator || null) as Validator | null;
    } catch {
      return null;
    }
  }

  async getStakingPool(): Promise<StakingPool> {
    const data = await this.get(`/cosmos/staking/v1beta1/pool`);
    return data.pool as StakingPool;
  }

  // ── Slashing ─────────────────────────────────────────────────

  async getSigningInfos(): Promise<SigningInfo[]> {
    const params = new URLSearchParams();
    params.set("pagination.limit", "500");

    const data = await this.get(
      `/cosmos/slashing/v1beta1/signing_infos?${params.toString()}`
    );
    return (data.info || []) as SigningInfo[];
  }

  async getSlashingParams(): Promise<SlashingParams | null> {
    try {
      const data = await this.get(`/cosmos/slashing/v1beta1/params`);
      return (data.params || null) as SlashingParams | null;
    } catch {
      return null;
    }
  }

  // ── Governance (for participation scoring) ───────────────────

  async getRecentFinalizedProposals(limit = 20): Promise<Proposal[]> {
    // Pull the most recent passed + rejected proposals. Passed = "3",
    // Rejected = "4" in the Cosmos gov v1beta1 status enum.
    const params = new URLSearchParams();
    params.set("pagination.limit", String(limit));
    params.set("pagination.reverse", "true");

    const results: Proposal[] = [];
    for (const status of ["3", "4"]) {
      params.set("proposal_status", status);
      try {
        const data = await this.get(
          `/cosmos/gov/v1beta1/proposals?${params.toString()}`
        );
        const list = (data.proposals || []) as Proposal[];
        results.push(...list);
      } catch {
        // ignore; partial proposal history still produces a useful score
      }
    }

    // Newest first, deduped by id, capped at `limit`
    const seen = new Set<string>();
    const ordered = results
      .slice()
      .sort((a, b) => Number(b.id) - Number(a.id))
      .filter((p) => {
        if (seen.has(p.id)) return false;
        seen.add(p.id);
        return true;
      })
      .slice(0, limit);
    return ordered;
  }

  async getVoteForVoter(
    proposalId: string,
    voter: string
  ): Promise<Vote | null> {
    try {
      const data = await this.get(
        `/cosmos/gov/v1beta1/proposals/${proposalId}/votes/${voter}`
      );
      return (data.vote || null) as Vote | null;
    } catch {
      return null;
    }
  }

  // ── Connectivity check ──────────────────────────────────────

  async checkConnection(): Promise<{ blockHeight: string }> {
    const data = await this.get(`/cosmos/base/tendermint/v1beta1/blocks/latest`);
    const block = data.block as Record<string, unknown> | undefined;
    const header = block?.header as Record<string, unknown> | undefined;
    const height = (header?.height as string) || "unknown";
    return { blockHeight: height };
  }

  // ── HTTP ─────────────────────────────────────────────────────

  private async get(path: string): Promise<Record<string, unknown>> {
    const url = `${this.baseUrl}${path}`;
    const res = await fetch(url, {
      headers: { Accept: "application/json" },
      signal: AbortSignal.timeout(15_000),
    });

    if (!res.ok) {
      throw new Error(`LCD ${res.status}: ${res.statusText} — ${url}`);
    }

    return (await res.json()) as Record<string, unknown>;
  }
}
