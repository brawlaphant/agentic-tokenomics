// ============================================================
// Regen Ledger ecocredit types
// ============================================================

export interface CreditClass {
  id: string;
  admin: string;
  credit_type: CreditType;
  metadata: string;
  issuers?: string[];
}

export interface CreditType {
  abbreviation: string;
  name: string;
  unit: string;
  precision: number;
}

export interface Project {
  id: string;
  class_id: string;
  jurisdiction: string;
  metadata: string;
  admin: string;
  reference_id: string;
}

export interface CreditBatch {
  denom: string;
  project_id: string;
  issuer: string;
  start_date: string;
  end_date: string;
  total_amount: string;
  metadata: string;
  open: boolean;
}

// ============================================================
// Screening types
// ============================================================

export type Recommendation = "APPROVE" | "CONDITIONAL" | "REJECT";

export interface ScreeningFactors {
  methodology_quality: number;   // 0-1000
  reputation: number;            // 0-1000
  novelty: number;               // 0-1000
  completeness: number;          // 0-1000
}

export interface ScreeningResult {
  score: number;                 // 0-1000 composite
  confidence: number;            // 0-1000
  recommendation: Recommendation;
  factors: ScreeningFactors;
  rationale: string;
}

// ============================================================
// OODA loop types
// ============================================================

export interface OODAExecution<TObserve, TOrient, TDecide, TAct> {
  executionId: string;
  workflowId: string;
  status: "running" | "completed" | "failed" | "escalated";
  observations: TObserve;
  orientation: TOrient | null;
  decision: TDecide | null;
  actions: TAct | null;
  startedAt: Date;
  completedAt: Date | null;
  error: string | null;
}

// ============================================================
// Output types
// ============================================================

export type AlertLevel = "NORMAL" | "HIGH" | "CRITICAL";

export interface OutputMessage {
  workflow: string;
  entityId: string;
  title: string;
  content: string;
  alertLevel: AlertLevel;
  timestamp: Date;
}
