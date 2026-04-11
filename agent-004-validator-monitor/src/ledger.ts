import { config } from "./config.js";
import type {
  Validator,
  SigningInfo,
  SlashingParams,
  StakingPool,
  Proposal,
  Vote,
  DelegationEvent,
} from "./types.js";

/**
 * Parse a Cosmos SDK coin-amount attribute into a uregen string.
 * The staking event attributes carry amounts like `"1000uregen"` —
 * the numeric part followed by the denom. Strip the denom and
 * return the numeric prefix as a string so BigInt downstream can
 * consume it losslessly. Returns null on bad input.
 */
function parseCoinAmount(raw: string | null): string | null {
  if (!raw) return null;
  const m = raw.match(/^(\d+)/);
  if (!m) return null;
  return m[1] ?? null;
}

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
    } catch (err) {
      console.error(
        `LedgerClient.getVoteForVoter(${proposalId}, ${voter}) failed:`,
        err
      );
      return null;
    }
  }

  /**
   * Fetch every vote cast on a proposal in one query. Used instead
   * of the per-voter `getVoteForVoter` when scoring a large validator
   * set — the per-voter path would fan out O(validators × proposals)
   * LCD requests per cycle and trip rate limits on public endpoints.
   * With this, we spend O(proposals) requests and index the results
   * locally by voter address.
   *
   * Walks pagination via `pagination.key` with a safety cap of 25
   * pages (500 votes per page × 25 = 12.5k voters, more than
   * enough headroom for the current validator set).
   */
  async getVotesForProposal(proposalId: string): Promise<Vote[]> {
    const pageSize = 500;
    const MAX_PAGES = 25;
    const votes: Vote[] = [];
    let nextKey: string | null = null;
    for (let i = 0; i < MAX_PAGES; i++) {
      try {
        const params = new URLSearchParams();
        params.set("pagination.limit", String(pageSize));
        if (nextKey) params.set("pagination.key", nextKey);

        const data = await this.get(
          `/cosmos/gov/v1beta1/proposals/${proposalId}/votes?${params.toString()}`
        );
        const page = (data.votes || []) as Vote[];
        votes.push(...page);

        const pagination = data.pagination as
          | { next_key?: string | null }
          | undefined;
        const rawKey = pagination?.next_key;
        if (!rawKey) break;
        nextKey = rawKey;
      } catch (err) {
        console.error(
          `LedgerClient.getVotesForProposal(${proposalId}) page ${i} failed:`,
          err
        );
        break;
      }
    }
    return votes;
  }

  // ── Tx-search: staking delegation events ────────────────────
  //
  // Pulls recent tx responses for each of the three staking message
  // types and parses them into DelegationEvent records. Each call
  // hits a different tx-search event filter because the LCD only
  // supports one `events=` filter per request.

  async getRecentDelegationTxs(limit = 100): Promise<DelegationEvent[]> {
    const typeUrls = [
      "/cosmos.staking.v1beta1.MsgDelegate",
      "/cosmos.staking.v1beta1.MsgUndelegate",
      "/cosmos.staking.v1beta1.MsgBeginRedelegate",
    ];

    const results: DelegationEvent[] = [];

    for (const typeUrl of typeUrls) {
      try {
        const params = new URLSearchParams();
        params.set("events", `message.action='${typeUrl}'`);
        params.set("pagination.limit", String(limit));
        params.set("pagination.reverse", "true");
        params.set("order_by", "ORDER_BY_DESC");

        const data = await this.get(`/cosmos/tx/v1beta1/txs?${params.toString()}`);
        const txResponses = (data.tx_responses || []) as Array<Record<string, unknown>>;

        for (const tx of txResponses) {
          results.push(...this.parseDelegationEventsFromTx(tx));
        }
      } catch {
        // Per-type-url failure is isolated — the other two still run.
        // An off-chain aggregator that loses one of three queries is
        // still better than falling back to the token-delta proxy.
      }
    }

    return results;
  }

  /**
   * Parse a single Cosmos tx_response into zero or more
   * DelegationEvent records. Public so unit tests can feed it
   * synthetic inputs. Matches three Cosmos SDK event types:
   *
   *   - `delegate`     (MsgDelegate)
   *   - `unbond`       (MsgUndelegate)
   *   - `redelegate`   (MsgBeginRedelegate)
   *
   * Also reads the `message` event to extract the delegator address
   * (sender) because the staking events themselves don't carry it.
   * The association is positional: the Nth staking event in a tx
   * maps to the Nth message event's sender.
   */
  parseDelegationEventsFromTx(tx: Record<string, unknown>): DelegationEvent[] {
    const txHash = typeof tx.txhash === "string" ? tx.txhash : "";
    const timestamp =
      typeof tx.timestamp === "string" ? tx.timestamp : new Date().toISOString();

    const collected: Array<Record<string, unknown>> = [];
    const logs = (tx.logs || []) as Array<Record<string, unknown>>;
    for (const log of logs) {
      collected.push(...((log.events || []) as Array<Record<string, unknown>>));
    }
    collected.push(...((tx.events || []) as Array<Record<string, unknown>>));

    // Sender addresses from message events, in order of appearance.
    const senders: string[] = [];
    for (const ev of collected) {
      if (typeof ev.type === "string" && ev.type === "message") {
        const attributes = (ev.attributes || []) as Array<{ key: string; value: string }>;
        const sender = attributes.find((a) => a.key === "sender");
        if (sender) senders.push(sender.value);
      }
    }

    const results: DelegationEvent[] = [];
    let senderIdx = 0;

    const nextSender = (): string => {
      const s = senders[senderIdx] ?? "";
      senderIdx++;
      return s;
    };

    for (const ev of collected) {
      const type = typeof ev.type === "string" ? ev.type : "";
      const attributes = (ev.attributes || []) as Array<{ key: string; value: string }>;
      const attr = (k: string): string | null => {
        const hit = attributes.find((a) => a.key === k);
        return hit ? hit.value : null;
      };

      if (type === "delegate") {
        const validator = attr("validator") ?? "";
        const amount = parseCoinAmount(attr("amount"));
        if (!validator || amount === null) continue;
        results.push({
          txHash,
          eventType: "delegate",
          delegator: nextSender(),
          validator,
          sourceValidator: null,
          amountUregen: amount,
          occurredAt: timestamp,
        });
      } else if (type === "unbond") {
        const validator = attr("validator") ?? "";
        const amount = parseCoinAmount(attr("amount"));
        if (!validator || amount === null) continue;
        results.push({
          txHash,
          eventType: "undelegate",
          delegator: nextSender(),
          validator,
          sourceValidator: null,
          amountUregen: amount,
          occurredAt: timestamp,
        });
      } else if (type === "redelegate") {
        const src = attr("source_validator") ?? "";
        const dst = attr("destination_validator") ?? "";
        const amount = parseCoinAmount(attr("amount"));
        if (!src || !dst || amount === null) continue;
        results.push({
          txHash,
          eventType: "redelegate",
          delegator: nextSender(),
          validator: dst,
          sourceValidator: src,
          amountUregen: amount,
          occurredAt: timestamp,
        });
      }
    }

    return results;
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
