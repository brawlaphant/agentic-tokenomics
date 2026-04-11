import Database from "better-sqlite3";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DB_PATH = path.join(__dirname, "..", "agent-001.db");

/**
 * Local SQLite store for agent state.
 *
 * Tracks which credit classes, projects, and batches we've screened,
 * plus workflow execution history. Intentionally lightweight —
 * can be replaced with PostgreSQL for production.
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

      CREATE TABLE IF NOT EXISTS credit_class_screenings (
        class_id TEXT PRIMARY KEY,
        screening TEXT NOT NULL,
        screened_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS project_screenings (
        project_id TEXT PRIMARY KEY,
        screening TEXT NOT NULL,
        screened_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS batch_screenings (
        batch_denom TEXT PRIMARY KEY,
        screening TEXT NOT NULL,
        screened_at TEXT NOT NULL
      );

      CREATE INDEX IF NOT EXISTS idx_exec_workflow
        ON workflow_executions(workflow_id);
    `);
  }

  // ── Credit class screenings ───────────────────────────

  hasClassScreening(classId: string): boolean {
    const row = this.db
      .prepare("SELECT 1 FROM credit_class_screenings WHERE class_id = ?")
      .get(classId);
    return !!row;
  }

  saveClassScreening(classId: string, screening: string): void {
    this.db
      .prepare(
        `INSERT OR REPLACE INTO credit_class_screenings (class_id, screening, screened_at)
         VALUES (?, ?, ?)`
      )
      .run(classId, screening, new Date().toISOString());
  }

  getClassScreening(classId: string): string | null {
    const row = this.db
      .prepare(
        "SELECT screening FROM credit_class_screenings WHERE class_id = ?"
      )
      .get(classId) as { screening: string } | undefined;
    return row?.screening || null;
  }

  // ── Project screenings ────────────────────────────────

  hasProjectScreening(projectId: string): boolean {
    const row = this.db
      .prepare("SELECT 1 FROM project_screenings WHERE project_id = ?")
      .get(projectId);
    return !!row;
  }

  saveProjectScreening(projectId: string, screening: string): void {
    this.db
      .prepare(
        `INSERT OR REPLACE INTO project_screenings (project_id, screening, screened_at)
         VALUES (?, ?, ?)`
      )
      .run(projectId, screening, new Date().toISOString());
  }

  getProjectScreening(projectId: string): string | null {
    const row = this.db
      .prepare(
        "SELECT screening FROM project_screenings WHERE project_id = ?"
      )
      .get(projectId) as { screening: string } | undefined;
    return row?.screening || null;
  }

  // ── Batch screenings ──────────────────────────────────

  hasBatchScreening(batchDenom: string): boolean {
    const row = this.db
      .prepare("SELECT 1 FROM batch_screenings WHERE batch_denom = ?")
      .get(batchDenom);
    return !!row;
  }

  saveBatchScreening(batchDenom: string, screening: string): void {
    this.db
      .prepare(
        `INSERT OR REPLACE INTO batch_screenings (batch_denom, screening, screened_at)
         VALUES (?, ?, ?)`
      )
      .run(batchDenom, screening, new Date().toISOString());
  }

  getBatchScreening(batchDenom: string): string | null {
    const row = this.db
      .prepare(
        "SELECT screening FROM batch_screenings WHERE batch_denom = ?"
      )
      .get(batchDenom) as { screening: string } | undefined;
    return row?.screening || null;
  }

  // ── Workflow executions ───────────────────────────────

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
