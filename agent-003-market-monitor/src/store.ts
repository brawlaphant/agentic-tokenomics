import Database from "better-sqlite3";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DB_PATH = path.join(__dirname, "..", "agent-003.db");

/**
 * Local SQLite store for AGENT-003 state.
 *
 * Tracks trade observations (for trailing median + z-score), liquidity
 * snapshots, retirement summaries, and workflow execution history.
 * Intentionally lightweight — can be replaced with PostgreSQL later.
 */
class Store {
  private db: Database.Database;

  constructor() {
    this.db = new Database(DB_PATH);
    this.db.pragma("journal_mode = WAL");
    this.migrate();
  }

  private migrate() {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS workflow_executions (
        execution_id TEXT PRIMARY KEY,
        workflow_id TEXT NOT NULL,
        agent_id TEXT NOT NULL,
        status TEXT NOT NULL,
        started_at TEXT NOT NULL,
        completed_at TEXT,
        result TEXT
      );

      -- Trailing trade observations, used as the sample set for
      -- class/batch median + z-score in WF-MM-01. Each row is one
      -- observed trade; the analyzer reads rows matching the batch or
      -- class within the trailing window.
      CREATE TABLE IF NOT EXISTS trade_observations (
        trade_id TEXT PRIMARY KEY,
        class_id TEXT NOT NULL,
        batch_denom TEXT NOT NULL,
        price_per_credit REAL NOT NULL,
        quantity REAL NOT NULL,
        executed_at TEXT NOT NULL
      );

      -- Deduplication record for WF-MM-01 — we only alert once per
      -- (trade_id, severity). Re-alerts only escalate to a higher
      -- severity (enforced in the workflow).
      CREATE TABLE IF NOT EXISTS anomaly_alerts (
        trade_id TEXT PRIMARY KEY,
        severity TEXT NOT NULL,
        z_class REAL NOT NULL,
        z_batch REAL NOT NULL,
        detected_at TEXT NOT NULL,
        report TEXT NOT NULL
      );

      -- Snapshots of liquidity health per class, used by WF-MM-02
      -- to compare against the previous snapshot and surface trends.
      CREATE TABLE IF NOT EXISTS liquidity_snapshots (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        class_id TEXT NOT NULL,
        snapshot TEXT NOT NULL,
        health TEXT NOT NULL,
        captured_at TEXT NOT NULL
      );

      -- Summaries of retirement activity per class, used by WF-MM-03
      -- to compute a rolling demand index.
      CREATE TABLE IF NOT EXISTS retirement_summaries (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        class_id TEXT NOT NULL,
        window_hours INTEGER NOT NULL,
        total_quantity REAL NOT NULL,
        total_value_usd REAL NOT NULL,
        retirement_count INTEGER NOT NULL,
        demand_index REAL NOT NULL,
        summary TEXT NOT NULL,
        captured_at TEXT NOT NULL
      );

      CREATE INDEX IF NOT EXISTS idx_trade_class ON trade_observations(class_id);
      CREATE INDEX IF NOT EXISTS idx_trade_batch ON trade_observations(batch_denom);
      CREATE INDEX IF NOT EXISTS idx_trade_executed ON trade_observations(executed_at);
      CREATE INDEX IF NOT EXISTS idx_liquidity_class ON liquidity_snapshots(class_id);
      CREATE INDEX IF NOT EXISTS idx_retirement_class ON retirement_summaries(class_id);
    `);
  }

  // ── Trade observations (for z-score baseline) ──────────────

  recordTrade(obs: {
    tradeId: string;
    classId: string;
    batchDenom: string;
    pricePerCredit: number;
    quantity: number;
    executedAt: string;
  }): void {
    this.db
      .prepare(
        `INSERT OR IGNORE INTO trade_observations
         (trade_id, class_id, batch_denom, price_per_credit, quantity, executed_at)
         VALUES (?, ?, ?, ?, ?, ?)`
      )
      .run(
        obs.tradeId,
        obs.classId,
        obs.batchDenom,
        obs.pricePerCredit,
        obs.quantity,
        obs.executedAt
      );
  }

  getTradesByClass(classId: string, windowDays: number): number[] {
    const cutoff = new Date(Date.now() - windowDays * 86_400_000).toISOString();
    const rows = this.db
      .prepare(
        `SELECT price_per_credit as p FROM trade_observations
         WHERE class_id = ? AND executed_at >= ?`
      )
      .all(classId, cutoff) as { p: number }[];
    return rows.map((r) => r.p);
  }

  getTradesByBatch(batchDenom: string, windowDays: number): number[] {
    const cutoff = new Date(Date.now() - windowDays * 86_400_000).toISOString();
    const rows = this.db
      .prepare(
        `SELECT price_per_credit as p FROM trade_observations
         WHERE batch_denom = ? AND executed_at >= ?`
      )
      .all(batchDenom, cutoff) as { p: number }[];
    return rows.map((r) => r.p);
  }

  // ── Anomaly alerts ─────────────────────────────────────────

  hasAnomalyAlert(tradeId: string): boolean {
    const row = this.db
      .prepare("SELECT severity FROM anomaly_alerts WHERE trade_id = ?")
      .get(tradeId) as { severity: string } | undefined;
    return !!row;
  }

  getAnomalyAlertSeverity(tradeId: string): string | null {
    const row = this.db
      .prepare("SELECT severity FROM anomaly_alerts WHERE trade_id = ?")
      .get(tradeId) as { severity: string } | undefined;
    return row?.severity || null;
  }

  saveAnomalyAlert(alert: {
    tradeId: string;
    severity: string;
    zClass: number;
    zBatch: number;
    report: string;
  }): void {
    this.db
      .prepare(
        `INSERT OR REPLACE INTO anomaly_alerts
         (trade_id, severity, z_class, z_batch, detected_at, report)
         VALUES (?, ?, ?, ?, ?, ?)`
      )
      .run(
        alert.tradeId,
        alert.severity,
        alert.zClass,
        alert.zBatch,
        new Date().toISOString(),
        alert.report
      );
  }

  // ── Liquidity snapshots ────────────────────────────────────

  saveLiquiditySnapshot(classId: string, snapshot: string, health: string): void {
    this.db
      .prepare(
        `INSERT INTO liquidity_snapshots (class_id, snapshot, health, captured_at)
         VALUES (?, ?, ?, ?)`
      )
      .run(classId, snapshot, health, new Date().toISOString());
  }

  getLatestLiquiditySnapshot(
    classId: string
  ): { snapshot: string; health: string; captured_at: string } | null {
    const row = this.db
      .prepare(
        `SELECT snapshot, health, captured_at FROM liquidity_snapshots
         WHERE class_id = ? ORDER BY id DESC LIMIT 1`
      )
      .get(classId) as
      | { snapshot: string; health: string; captured_at: string }
      | undefined;
    return row || null;
  }

  // ── Retirement summaries ───────────────────────────────────

  saveRetirementSummary(row: {
    classId: string;
    windowHours: number;
    totalQuantity: number;
    totalValueUsd: number;
    retirementCount: number;
    demandIndex: number;
    summary: string;
  }): void {
    this.db
      .prepare(
        `INSERT INTO retirement_summaries
         (class_id, window_hours, total_quantity, total_value_usd,
          retirement_count, demand_index, summary, captured_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .run(
        row.classId,
        row.windowHours,
        row.totalQuantity,
        row.totalValueUsd,
        row.retirementCount,
        row.demandIndex,
        row.summary,
        new Date().toISOString()
      );
  }

  getBaselineDemand(classId: string, lookbackDays: number): number {
    // Average of the demand_index values captured in the prior window.
    // Used as the reference point for the new demand_index computation.
    const cutoff = new Date(Date.now() - lookbackDays * 86_400_000).toISOString();
    const row = this.db
      .prepare(
        `SELECT AVG(demand_index) as avg FROM retirement_summaries
         WHERE class_id = ? AND captured_at >= ?`
      )
      .get(classId, cutoff) as { avg: number | null };
    return row.avg ?? 0;
  }

  // ── Workflow executions ────────────────────────────────────

  logExecution(exec: {
    executionId: string;
    workflowId: string;
    agentId: string;
    status: string;
    startedAt: string;
    completedAt: string;
    result: string;
  }): void {
    this.db
      .prepare(
        `INSERT INTO workflow_executions
         (execution_id, workflow_id, agent_id, status, started_at, completed_at, result)
         VALUES (?, ?, ?, ?, ?, ?, ?)`
      )
      .run(
        exec.executionId,
        exec.workflowId,
        exec.agentId,
        exec.status,
        exec.startedAt,
        exec.completedAt,
        exec.result
      );
  }

  getExecutionCount(): number {
    const row = this.db
      .prepare("SELECT COUNT(*) as cnt FROM workflow_executions")
      .get() as { cnt: number };
    return row.cnt;
  }

  close(): void {
    this.db.close();
  }
}

export const store = new Store();
