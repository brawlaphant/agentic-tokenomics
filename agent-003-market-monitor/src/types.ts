// ============================================================
// Regen Ledger ecocredit marketplace types
// ============================================================

export interface CreditClass {
  id: string;
  admin: string;
  credit_type: CreditType;
  metadata: string;
}

export interface CreditType {
  abbreviation: string;
  name: string;
  unit: string;
  precision: number;
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

export interface BatchSupply {
  batch_denom: string;
  tradable_amount: string;
  retired_amount: string;
  cancelled_amount: string;
}

export interface SellOrder {
  id: string;
  seller: string;
  batch_denom: string;
  quantity: string;
  ask_denom: string;
  ask_amount: string;
  disable_auto_retire: boolean;
  expiration: string;
}

/** Synthesized trade record (from filled sell orders or tx logs) */
export interface Trade {
  id: string;
  batchDenom: string;
  classId: string;
  seller: string;
  buyer: string;
  quantity: number;
  pricePerCredit: number;   // USD-equivalent
  askDenom: string;
  executedAt: string;
}

/** Synthesized retirement record */
export interface Retirement {
  txHash: string;
  batchDenom: string;
  classId: string;
  retiree: string;
  quantity: number;
  jurisdiction: string | null;
  reason: string | null;
  retiredAt: string;
}

// ============================================================
// OODA loop types (shared shape mirrors agent-002)
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
// Workflow-specific types
// ============================================================

export type AlertLevel = "NORMAL" | "HIGH" | "CRITICAL";
export type AnomalySeverity = "INFO" | "WARNING" | "CRITICAL";

/** Price anomaly detection (WF-MM-01) */
export interface PriceAnomaly {
  tradeId: string;
  batchDenom: string;
  classId: string;
  seller: string;
  quantity: number;
  pricePerCredit: number;
  classMedian: number;
  batchMedian: number;
  zScoreVsClass: number;
  zScoreVsBatch: number;
  sampleSizeClass: number;
  sampleSizeBatch: number;
  severity: AnomalySeverity;
  detectedAt: string;
  confidence: number;
}

/** Liquidity monitoring (WF-MM-02) */
export interface LiquiditySnapshot {
  classId: string;
  totalListedQuantity: number;
  totalListedValueUsd: number;
  sellOrderCount: number;
  lowestAskUsd: number;
  highestAskUsd: number;
  medianAskUsd: number;
  meanAskUsd: number;
  depthUsd: number;           // sum of top-10 sell orders
  healthScore: number;        // 0-1
  health: "HEALTHY" | "DEGRADED" | "CRITICAL";
  capturedAt: string;
}

/** Retirement tracking (WF-MM-03) */
export interface RetirementSummary {
  classId: string;
  windowHours: number;
  retirementCount: number;
  totalQuantity: number;
  totalValueUsd: number;
  uniqueRetirees: number;
  topRetiree: string | null;
  topRetireeQuantity: number;
  pctWithJurisdiction: number;
  demandIndex: number;        // 0-100, relative to trailing baseline
  capturedAt: string;
}

// ============================================================
// Output types
// ============================================================

export interface OutputMessage {
  workflow: string;
  subjectId: string;   // trade id / class id / batch denom
  title: string;
  content: string;
  alertLevel: AlertLevel;
  timestamp: Date;
}
