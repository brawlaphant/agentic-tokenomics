import Database from "better-sqlite3";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DB_PATH = path.join(__dirname, "..", "agent-004.db");

/**
 * Local SQLite store for AGENT-004 state.
 *
 * Tracks per-validator token snapshots (used as the delta source for
 * WF-VM-02 until a real tx-stream lands), scorecards, decentralization
 * snapshots, and workflow execution history.
 */
export class Store {
  private db: Database.Database;

  /**
   * `dbPath` defaults to the per-agent DB file on disk. Tests can
   * pass `":memory:"` (or a temp file) to avoid clobbering the shared
   * DB file and to run test suites in parallel without hitting the
   * `database is locked` failure mode.
   */
  constructor(dbPath: string = DB_PATH) {
    this.db = new Database(dbPath);
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

      -- Snapshot of a validator's staked tokens at a point in time.
      -- WF-VM-02 reads the most recent previous row per operator to
      -- derive the per-cycle delegation delta.
      CREATE TABLE IF NOT EXISTS validator_token_snapshots (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        operator_address TEXT NOT NULL,
        moniker TEXT NOT NULL,
        tokens TEXT NOT NULL,
        commission_rate TEXT NOT NULL,
        jailed INTEGER NOT NULL,
        captured_at TEXT NOT NULL
      );

      -- Snapshot of the commission rate timeline per validator. WF-VM-01
      -- uses this to count commission changes in the stability score.
      CREATE TABLE IF NOT EXISTS commission_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        operator_address TEXT NOT NULL,
        commission_rate TEXT NOT NULL,
        captured_at TEXT NOT NULL
      );

      -- Cached scorecards keyed by operator + captured_at so the
      -- narrative layer can look up the previous scorecard for a
      -- validator when reporting a change.
      CREATE TABLE IF NOT EXISTS scorecards (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        operator_address TEXT NOT NULL,
        composite_score INTEGER NOT NULL,
        scorecard TEXT NOT NULL,
        captured_at TEXT NOT NULL
      );

      -- Decentralization snapshots (WF-VM-03)
      CREATE TABLE IF NOT EXISTS decentralization_snapshots (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        nakamoto INTEGER NOT NULL,
        gini REAL NOT NULL,
        top10_pct REAL NOT NULL,
        health TEXT NOT NULL,
        snapshot TEXT NOT NULL,
        captured_at TEXT NOT NULL
      );

      CREATE INDEX IF NOT EXISTS idx_tokens_operator ON validator_token_snapshots(operator_address);
      CREATE INDEX IF NOT EXISTS idx_commission_operator ON commission_history(operator_address);
      CREATE INDEX IF NOT EXISTS idx_scorecards_operator ON scorecards(operator_address);
    `);
  }

  // ── Token snapshots (for WF-VM-02 delta source) ────────────

  recordTokenSnapshot(row: {
    operatorAddress: string;
    moniker: string;
    tokens: string;
    commissionRate: string;
    jailed: boolean;
  }): void {
    this.db
      .prepare(
        `INSERT INTO validator_token_snapshots
         (operator_address, moniker, tokens, commission_rate, jailed, captured_at)
         VALUES (?, ?, ?, ?, ?, ?)`
      )
      .run(
        row.operatorAddress,
        row.moniker,
        row.tokens,
        row.commissionRate,
        row.jailed ? 1 : 0,
        new Date().toISOString()
      );
  }

  getPreviousTokenSnapshot(
    operatorAddress: string,
    excludeLatest: boolean
  ): { tokens: string; captured_at: string } | null {
    // When excludeLatest is true, skip the most recent row (the one we
    // just wrote this cycle) and return the one before it.
    const offset = excludeLatest ? 1 : 0;
    const row = this.db
      .prepare(
        `SELECT tokens, captured_at FROM validator_token_snapshots
         WHERE operator_address = ? ORDER BY id DESC LIMIT 1 OFFSET ?`
      )
      .get(operatorAddress, offset) as
      | { tokens: string; captured_at: string }
      | undefined;
    return row || null;
  }

  // ── Commission history ─────────────────────────────────────

  recordCommissionIfChanged(
    operatorAddress: string,
    commissionRate: string
  ): boolean {
    const latest = this.db
      .prepare(
        `SELECT commission_rate FROM commission_history
         WHERE operator_address = ? ORDER BY id DESC LIMIT 1`
      )
      .get(operatorAddress) as { commission_rate: string } | undefined;
    if (latest && latest.commission_rate === commissionRate) return false;

    this.db
      .prepare(
        `INSERT INTO commission_history
         (operator_address, commission_rate, captured_at)
         VALUES (?, ?, ?)`
      )
      .run(operatorAddress, commissionRate, new Date().toISOString());
    return true;
  }

  countCommissionChangesSince(
    operatorAddress: string,
    sinceIso: string
  ): number {
    // Count every distinct commission-rate row recorded in the
    // trailing window. The trick is the "is the first row in the
    // window actually a change" question: if there's an older row
    // from *before* the window, the first in-window row *is* a change
    // (the rate transitioned from the older one). If there is no
    // older row, the first in-window row is the baseline read and
    // should not be counted.
    //
    // The previous implementation counted `cnt - 1`, which dropped a
    // real commission change whenever the baseline read fell outside
    // the window — e.g. "validator changed commission once in the
    // last 30 days but last recorded 40 days ago" returned 0.
    const inWindow = this.db
      .prepare(
        `SELECT COUNT(*) AS cnt FROM commission_history
         WHERE operator_address = ? AND captured_at >= ?`
      )
      .get(operatorAddress, sinceIso) as { cnt: number };

    if (inWindow.cnt === 0) return 0;

    const hasBaseline = this.db
      .prepare(
        `SELECT 1 FROM commission_history
         WHERE operator_address = ? AND captured_at < ? LIMIT 1`
      )
      .get(operatorAddress, sinceIso) as { 1: number } | undefined;

    // When a baseline row exists outside the window, every in-window
    // row is a change. Otherwise the first in-window row is the
    // baseline and the remaining rows are changes.
    return hasBaseline ? inWindow.cnt : Math.max(0, inWindow.cnt - 1);
  }

  // ── Scorecards ─────────────────────────────────────────────

  saveScorecard(row: {
    operatorAddress: string;
    compositeScore: number;
    scorecard: string;
  }): void {
    this.db
      .prepare(
        `INSERT INTO scorecards
         (operator_address, composite_score, scorecard, captured_at)
         VALUES (?, ?, ?, ?)`
      )
      .run(
        row.operatorAddress,
        row.compositeScore,
        row.scorecard,
        new Date().toISOString()
      );
  }

  getLatestScorecard(
    operatorAddress: string
  ): { compositeScore: number; scorecard: string; capturedAt: string } | null {
    const row = this.db
      .prepare(
        `SELECT composite_score, scorecard, captured_at FROM scorecards
         WHERE operator_address = ? ORDER BY id DESC LIMIT 1`
      )
      .get(operatorAddress) as
      | { composite_score: number; scorecard: string; captured_at: string }
      | undefined;
    if (!row) return null;
    return {
      compositeScore: row.composite_score,
      scorecard: row.scorecard,
      capturedAt: row.captured_at,
    };
  }

  // ── Decentralization snapshots ─────────────────────────────

  saveDecentralizationSnapshot(row: {
    nakamoto: number;
    gini: number;
    top10Pct: number;
    health: string;
    snapshot: string;
  }): void {
    this.db
      .prepare(
        `INSERT INTO decentralization_snapshots
         (nakamoto, gini, top10_pct, health, snapshot, captured_at)
         VALUES (?, ?, ?, ?, ?, ?)`
      )
      .run(
        row.nakamoto,
        row.gini,
        row.top10Pct,
        row.health,
        row.snapshot,
        new Date().toISOString()
      );
  }

  getLatestDecentralizationSnapshot(): {
    nakamoto: number;
    gini: number;
    top10_pct: number;
    health: string;
    snapshot: string;
    captured_at: string;
  } | null {
    // `LIMIT 1 OFFSET 0` intentionally — the caller invokes this in
    // the `orient` phase before the current cycle's snapshot has
    // been written, so the newest row in the table is the actual
    // "previous" cycle's snapshot. An `OFFSET 1` here would skip
    // that row and compare against the cycle before last, which
    // shifts the trend analysis by one full cycle and returns null
    // on the agent's second run (when only one row exists).
    const row = this.db
      .prepare(
        `SELECT nakamoto, gini, top10_pct, health, snapshot, captured_at
         FROM decentralization_snapshots ORDER BY id DESC LIMIT 1`
      )
      .get() as
      | {
          nakamoto: number;
          gini: number;
          top10_pct: number;
          health: string;
          snapshot: string;
          captured_at: string;
        }
      | undefined;
    return row || null;
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
