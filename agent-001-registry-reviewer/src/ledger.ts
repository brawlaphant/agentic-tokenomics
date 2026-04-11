import { config } from "./config.js";
import type { CreditClass, Project, CreditBatch } from "./types.js";

/**
 * Regen Ledger LCD (REST) client — ecocredit module.
 *
 * Talks directly to a Cosmos LCD endpoint — no MCP dependency.
 * When Ledger MCP becomes available, this can be swapped out
 * behind the same interface.
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
    } catch {
      return null;
    }
  }

  async getClassIssuers(classId: string): Promise<string[]> {
    try {
      const params = new URLSearchParams();
      params.set("pagination.limit", "100");

      const data = await this.get(
        `/regen/ecocredit/v1/classes/${classId}/issuers?${params.toString()}`
      );
      return (data.issuers || []) as string[];
    } catch {
      return [];
    }
  }

  // ── Projects ───────────────────────────────────────────────

  async getProjects(): Promise<Project[]> {
    const params = new URLSearchParams();
    params.set("pagination.limit", "200");

    const data = await this.get(
      `/regen/ecocredit/v1/projects?${params.toString()}`
    );
    return (data.projects || []) as Project[];
  }

  async getProject(projectId: string): Promise<Project | null> {
    try {
      const data = await this.get(
        `/regen/ecocredit/v1/projects/${projectId}`
      );
      return (data.project || null) as Project | null;
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
