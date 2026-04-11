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
    const params = new URLSearchParams();
    params.set("pagination.limit", "200");

    const data = await this.get(
      `/regen/ecocredit/v1/classes?${params.toString()}`
    );
    return (data.classes || []) as CreditClass[];
  }

  async getCreditClass(classId: string): Promise<CreditClass | null> {
    try {
      const data = await this.get(
        `/regen/ecocredit/v1/classes/${classId}`
      );
      return (data.class || null) as CreditClass | null;
    } catch (err) {
      console.error(`LedgerClient.getCreditClass(${classId}) failed:`, err);
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
    } catch (err) {
      console.error(`LedgerClient.getCreditBatch(${denom}) failed:`, err);
      return null;
    }
  }

  async getBatchSupply(denom: string): Promise<BatchSupply | null> {
    try {
      const data = await this.get(
        `/regen/ecocredit/v1/batches/${denom}/supply`
      );
      return (data.supply || null) as BatchSupply | null;
    } catch (err) {
      console.error(`LedgerClient.getBatchSupply(${denom}) failed:`, err);
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
    } catch (err) {
      console.error(`LedgerClient.getSellOrders(limit=${limit}) failed:`, err);
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
    } catch (err) {
      console.error(
        `LedgerClient.getSellOrdersByBatch(${denom}) failed:`,
        err
      );
      return [];
    }
  }

  async getSellOrder(orderId: string): Promise<SellOrder | null> {
    try {
      const data = await this.get(
        `/regen/ecocredit/marketplace/v1/sell-orders/${orderId}`
      );
      return (data.sell_order || null) as SellOrder | null;
    } catch (err) {
      console.error(`LedgerClient.getSellOrder(${orderId}) failed:`, err);
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
