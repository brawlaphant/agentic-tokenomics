import { config } from "./config.js";
import type {
  CreditClass,
  CreditBatch,
  BatchSupply,
  SellOrder,
  Retirement,
} from "./types.js";

/** Extract the class id prefix from a batch denom (shared helper). */
function classIdFromBatchDenomHelper(denom: string): string {
  const idx = denom.indexOf("-");
  return idx > 0 ? denom.slice(0, idx) : denom;
}

/**
 * Regen Ledger LCD (REST) client — ecocredit marketplace endpoints.
 *
 * Talks directly to a Cosmos LCD endpoint — no MCP dependency.
 * When Ledger MCP becomes available, this can be swapped out behind
 * the same interface. Matches the pattern used by AGENT-001
 * (registry-reviewer) and AGENT-002 (governance-analyst).
 */
export class LedgerClient {
  private baseUrl: string;

  constructor(baseUrl?: string) {
    this.baseUrl = (baseUrl || config.lcdUrl).replace(/\/$/, "");
  }

  // ── Credit Classes ─────────────────────────────────────────

  async getCreditClasses(): Promise<CreditClass[]> {
    // Walk every page of the `/regen/ecocredit/v1/classes` endpoint so
    // the agent keeps monitoring new credit classes as the registry
    // grows. A hardcoded limit of 200 would silently drop classes
    // created after that ceiling, which is exactly the kind of blind
    // spot this agent is supposed to prevent.
    const pageSize = 200;
    const classes: CreditClass[] = [];
    let nextKey: string | null = null;
    // Cap total pages as a safety belt against runaway pagination.
    const MAX_PAGES = 25;
    for (let i = 0; i < MAX_PAGES; i++) {
      const params = new URLSearchParams();
      params.set("pagination.limit", String(pageSize));
      if (nextKey) params.set("pagination.key", nextKey);

      const data = await this.get(
        `/regen/ecocredit/v1/classes?${params.toString()}`
      );
      const pageClasses = (data.classes || []) as CreditClass[];
      classes.push(...pageClasses);

      const pagination = data.pagination as
        | { next_key?: string | null }
        | undefined;
      const rawKey = pagination?.next_key;
      if (!rawKey) break;
      nextKey = rawKey;
    }
    return classes;
  }

  async getCreditClass(classId: string): Promise<CreditClass | null> {
    try {
      const data = await this.get(
        `/regen/ecocredit/v1/classes/${classId}`
      );
      return (data.class || null) as CreditClass | null;
    } catch {
      return null;
    }
  }

  // ── Credit Batches ─────────────────────────────────────────

  async getCreditBatches(): Promise<CreditBatch[]> {
    const params = new URLSearchParams();
    params.set("pagination.limit", "200");
    params.set("pagination.reverse", "true");

    const data = await this.get(
      `/regen/ecocredit/v1/batches?${params.toString()}`
    );
    return (data.batches || []) as CreditBatch[];
  }

  async getCreditBatch(denom: string): Promise<CreditBatch | null> {
    try {
      const data = await this.get(
        `/regen/ecocredit/v1/batches/${denom}`
      );
      return (data.batch || null) as CreditBatch | null;
    } catch {
      return null;
    }
  }

  async getBatchSupply(denom: string): Promise<BatchSupply | null> {
    try {
      const data = await this.get(
        `/regen/ecocredit/v1/batches/${denom}/supply`
      );
      return (data.supply || null) as BatchSupply | null;
    } catch {
      return null;
    }
  }

  // ── Marketplace: Sell Orders ───────────────────────────────

  async getSellOrders(limit = 200): Promise<SellOrder[]> {
    try {
      const params = new URLSearchParams();
      params.set("pagination.limit", String(limit));

      const data = await this.get(
        `/regen/ecocredit/marketplace/v1/sell-orders?${params.toString()}`
      );
      return (data.sell_orders || []) as SellOrder[];
    } catch {
      return [];
    }
  }

  async getSellOrdersByBatch(denom: string): Promise<SellOrder[]> {
    try {
      const params = new URLSearchParams();
      params.set("pagination.limit", "200");

      const data = await this.get(
        `/regen/ecocredit/marketplace/v1/sell-orders/batch/${denom}?${params.toString()}`
      );
      return (data.sell_orders || []) as SellOrder[];
    } catch {
      return [];
    }
  }

  async getSellOrder(orderId: string): Promise<SellOrder | null> {
    try {
      const data = await this.get(
        `/regen/ecocredit/marketplace/v1/sell-orders/${orderId}`
      );
      return (data.sell_order || null) as SellOrder | null;
    } catch {
      return null;
    }
  }

  // ── Tx-search: MsgRetire events ─────────────────────────────
  //
  // Pulls recent tx responses filtered by `message.action` matching
  // the Regen ecocredit MsgRetire type URL. Each matching response
  // is parsed into zero or more Retirement records by walking the
  // `events` list and extracting the EventRetire attributes.
  //
  // A single transaction can contain multiple MsgRetire messages
  // (batched retirements), so each tx can emit more than one
  // Retirement record.
  //
  // Returns an empty array on any error — the workflow treats
  // "no tx results" identically to "no recent retirements", so
  // transient LCD failures degrade to zero retirement activity
  // rather than crashing the cycle.

  async getRecentRetirementTxs(limit = 100): Promise<Retirement[]> {
    // Query both the v1 and v1beta1 MsgRetire type URLs — the SDK
    // has carried both for years and registries still emit v1beta1
    // for older code paths. The parser below accepts either event
    // type. Dedup by tx hash so the same tx appearing under both
    // queries is only processed once.
    const typeUrls = [
      "/regen.ecocredit.v1.MsgRetire",
      "/regen.ecocredit.v1beta1.MsgRetire",
    ];

    const seenHashes = new Set<string>();
    const retirements: Retirement[] = [];

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
          const hash = typeof tx.txhash === "string" ? tx.txhash : "";
          if (hash && seenHashes.has(hash)) continue;
          if (hash) seenHashes.add(hash);
          retirements.push(...this.parseRetirementsFromTx(tx));
        }
      } catch (err) {
        console.error(
          `LedgerClient.getRecentRetirementTxs(${typeUrl}) failed:`,
          err
        );
      }
    }

    return retirements;
  }

  /**
   * Parse a single Cosmos tx_response into zero or more Retirement
   * records. Public so the unit tests can feed it synthetic
   * responses without hitting a real LCD.
   */
  parseRetirementsFromTx(tx: Record<string, unknown>): Retirement[] {
    const txHash = typeof tx.txhash === "string" ? tx.txhash : "";
    const timestamp = typeof tx.timestamp === "string" ? tx.timestamp : new Date().toISOString();
    const logs = (tx.logs || []) as Array<Record<string, unknown>>;
    const events: Array<Record<string, unknown>> = [];

    // Cosmos LCD responses can carry events at two levels: per-msg
    // inside `logs[i].events[]`, and flattened at `tx.events[]`.
    // In modern Cosmos SDK versions the flat list is a superset of
    // everything in the logs, so walking both paths double-counts
    // every event. Prefer the flat list if it's present and non-empty,
    // and fall back to the per-log list only when the flat list is
    // missing or empty (very old LCD builds).
    const flatEvents = (tx.events || []) as Array<Record<string, unknown>>;
    if (flatEvents.length > 0) {
      events.push(...flatEvents);
    } else {
      for (const log of logs) {
        const logEvents = (log.events || []) as Array<Record<string, unknown>>;
        events.push(...logEvents);
      }
    }

    const results: Retirement[] = [];
    for (const ev of events) {
      const type = typeof ev.type === "string" ? ev.type : "";
      // Match the Regen v1 EventRetire type or the fallback SDK
      // message event. Either path produces the same Retirement
      // shape for the downstream workflow.
      if (
        type !== "regen.ecocredit.v1.EventRetire" &&
        type !== "regen.ecocredit.v1beta1.EventRetire"
      ) {
        continue;
      }

      const attributes = (ev.attributes || []) as Array<{ key: string; value: string }>;
      const attr = (k: string): string | null => {
        const hit = attributes.find((a) => a.key === k);
        return hit ? hit.value : null;
      };

      const batchDenom = attr("batch_denom") ?? attr("batchDenom") ?? "";
      if (!batchDenom) continue;

      // Parse raw integer units as BigInt first to preserve precision
      // beyond Number.MAX_SAFE_INTEGER. Regen batches are 6-decimal
      // today (capped well below 2^53), but higher-precision credit
      // types (18-decimal biodiversity tokens etc.) are on the roadmap
      // and the parser should not lose precision at the boundary.
      const quantityRaw = (attr("amount") ?? attr("quantity") ?? "0").trim();
      let quantityBig: bigint;
      try {
        // Strip any decimal suffix — the on-chain event always emits
        // the smallest-unit integer, but some forwarders prettify it.
        const intPart = quantityRaw.split(".")[0] || "0";
        quantityBig = BigInt(intPart);
      } catch {
        continue;
      }
      if (quantityBig <= 0n) continue;
      // The downstream Retirement record uses `number` for ergonomics.
      // For current Regen credit precision this is a safe downcast;
      // future higher-precision assets should lift the whole field to
      // string/bigint in types.ts.
      const quantity = Number(quantityBig);
      if (!Number.isFinite(quantity)) continue;

      const retiree = attr("owner") ?? attr("retirer") ?? "";
      const jurisdiction = attr("jurisdiction");
      const reason = attr("reason");
      const classId = classIdFromBatchDenomHelper(batchDenom);

      results.push({
        txHash,
        batchDenom,
        classId,
        retiree,
        quantity,
        jurisdiction,
        reason,
        retiredAt: timestamp,
      });
    }
    return results;
  }

  // ── Connectivity check ────────────────────────────────────

  async checkConnection(): Promise<{ blockHeight: string }> {
    const data = await this.get(`/cosmos/base/tendermint/v1beta1/blocks/latest`);
    const block = data.block as Record<string, unknown> | undefined;
    const header = block?.header as Record<string, unknown> | undefined;
    const height = (header?.height as string) || "unknown";
    return { blockHeight: height };
  }

  // ── HTTP ────────────────────────────────────────────────────

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
