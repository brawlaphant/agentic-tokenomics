// ============================================================
// Cosmos SDK staking + slashing + gov types
// ============================================================

export interface Validator {
  operator_address: string;
  consensus_pubkey: { "@type": string; key: string };
  jailed: boolean;
  status: string;
  tokens: string;
  delegator_shares: string;
  description: {
    moniker: string;
    identity: string;
    website: string;
    security_contact: string;
    details: string;
  };
  unbonding_height: string;
  unbonding_time: string;
  commission: {
    commission_rates: {
      rate: string;
      max_rate: string;
      max_change_rate: string;
    };
    update_time: string;
  };
  min_self_delegation: string;
}

export interface SigningInfo {
  address: string;
  start_height: string;
  index_offset: string;
  jailed_until: string;
  tombstoned: boolean;
  missed_blocks_counter: string;
}

export interface SlashingParams {
  signed_blocks_window: string;
  min_signed_per_window: string;
  downtime_jail_duration: string;
  slash_fraction_double_sign: string;
  slash_fraction_downtime: string;
}

export interface StakingPool {
  bonded_tokens: string;
  not_bonded_tokens: string;
}

export interface Proposal {
  id: string;
  status: string;
  voting_start_time: string;
  voting_end_time: string;
  content: { "@type": string; title: string; description: string };
}

export interface Vote {
  proposal_id: string;
  voter: string;
  option: string;
  options: { option: string; weight: string }[];
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
// Workflow-specific types
// ============================================================

export type AlertLevel = "NORMAL" | "HIGH" | "CRITICAL";

/** Validator performance scorecard (WF-VM-01) */
export interface ValidatorScorecard {
  operatorAddress: string;
  moniker: string;
  jailed: boolean;
  tokens: string;
  stakePct: number;
  commissionRate: number;
  uptimeScore: number;          // 0..scoreWeightUptime
  uptimeRatio: number;          // 0..1
  missedBlocks: number;
  signedBlocksWindow: number;
  governanceScore: number;      // 0..scoreWeightGovernance
  governanceParticipation: number; // 0..1
  proposalsConsidered: number;
  votesCast: number;
  stabilityScore: number;       // 0..scoreWeightStability
  compositeScore: number;       // 0..1000
  poaEligible: boolean;
}

export interface PerformanceAlert {
  operatorAddress: string;
  moniker: string;
  severity: AlertLevel;
  reason: string;
}

/** Delegation flow analysis (WF-VM-02) */
export interface DelegationFlow {
  operatorAddress: string;
  moniker: string;
  previousTokens: string;
  currentTokens: string;
  deltaUregen: string;           // signed
  deltaAbsUregen: string;
  isWhale: boolean;
  flowDirection: "INFLOW" | "OUTFLOW" | "FLAT";
  capturedAt: string;
}

export interface DelegationFlowSummary {
  windowLabel: string;
  totalInflowUregen: string;
  totalOutflowUregen: string;
  netFlowUregen: string;         // signed
  validatorsWithFlow: number;
  whaleFlowCount: number;
  topInflow: DelegationFlow | null;
  topOutflow: DelegationFlow | null;
  capturedAt: string;
}

/** Network decentralization (WF-VM-03) */
export interface DecentralizationSnapshot {
  validatorCount: number;
  bondedUregen: string;
  nakamotoCoefficient: number;
  giniIndex: number;
  top10SharePct: number;
  top20SharePct: number;
  largestShareValidator: string;
  largestSharePct: number;
  health: "HEALTHY" | "WARNING" | "CRITICAL";
  capturedAt: string;
}

// ============================================================
// Output types
// ============================================================

export interface OutputMessage {
  workflow: string;
  subjectId: string;
  title: string;
  content: string;
  alertLevel: AlertLevel;
  timestamp: Date;
}
