import { config } from "./config.js";
import type {
  CreditClass,
  CreditBatch,
  BatchSupply,
  SellOrder,
} from "./types.js";

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
