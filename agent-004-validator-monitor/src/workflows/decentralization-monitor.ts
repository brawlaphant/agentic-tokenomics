import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { config } from "../config.js";
import { describeDecentralizationSnapshot } from "../monitor.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type {
  Validator,
  DecentralizationSnapshot,
  AlertLevel,
} from "../types.js";

/**
 * WF-VM-03: Network Decentralization Monitoring
 *
 * Trigger: periodic (every cycle) OR validator set change
 * Layer: 1 (alerts) / Layer 3 (if action needed)
 *
 * OODA:
 *   Observe — Fetch active validator set.
 *   Orient  — Compute Nakamoto coefficient, Gini index, top-N shares.
 *             Classify health tier.
 *   Decide  — Generate narrative via Claude.
 *   Act     — Persist snapshot, output, alert on threshold breach.
 */

interface Observations {
  validators: Validator[];
}

interface Orientation {
  snapshot: DecentralizationSnapshot;
  previous: DecentralizationSnapshot | null;
}

interface Decision {
  snapshot: DecentralizationSnapshot;
  previous: DecentralizationSnapshot | null;
  report: string;
  alertLevel: AlertLevel;
}

interface Actions {
  saved: number;
  alertsSent: number;
}

/** Smallest number of validators whose cumulative share of the total
 * bonded stake strictly exceeds 33.4%. Consensus breaks at 2/3+1, so
 * the canonical Cosmos-era Nakamoto coefficient uses 33% as the
 * halt-threshold proxy. */
export function nakamotoCoefficient(sortedDescTokens: bigint[], total: bigint): number {
  if (total === 0n || sortedDescTokens.length === 0) return 0;
  const threshold = (total * 334n) / 1000n; // 33.4%
  let cumulative = 0n;
  for (let i = 0; i < sortedDescTokens.length; i++) {
    cumulative += sortedDescTokens[i]!;
    if (cumulative > threshold) return i + 1;
  }
  return sortedDescTokens.length;
}

/** Gini index of a distribution. 0 = perfect equality, 1 = perfect
 * inequality. Operates on ascending-sorted numbers and uses the
 * textbook formula. */
export function giniIndex(sortedAscTokens: number[]): number {
  const n = sortedAscTokens.length;
  if (n === 0) return 0;
  let sum = 0;
  let cumulative = 0;
  for (let i = 0; i < n; i++) {
    cumulative += sortedAscTokens[i]!;
    sum += (2 * (i + 1) - n - 1) * sortedAscTokens[i]!;
  }
  if (cumulative === 0) return 0;
  return Math.abs(sum / (n * cumulative));
}

export function topNSharePct(
  sortedDescTokens: bigint[],
  total: bigint,
  n: number
): number {
  if (total === 0n) return 0;
  let acc = 0n;
  for (let i = 0; i < Math.min(n, sortedDescTokens.length); i++) {
    acc += sortedDescTokens[i]!;
  }
  return Number((acc * 10000n) / total) / 100;
}

export function classifyHealth(
  nakamoto: number,
  gini: number,
  largestSharePct: number
): DecentralizationSnapshot["health"] {
  if (
    nakamoto <= config.validator.nakamotoCriticalFloor ||
    largestSharePct >= config.validator.criticalConcentrationPct
  ) {
    return "CRITICAL";
  }
  if (
    nakamoto <= config.validator.nakamotoWarningFloor ||
    gini >= config.validator.giniWarningCeiling ||
    largestSharePct >= config.validator.warningConcentrationPct
  ) {
    return "WARNING";
  }
  return "HEALTHY";
}

export function createDecentralizationMonitorWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-VM-03",
    name: "Network Decentralization Monitoring",

    async observe(): Promise<Observations> {
      const validators = await ledger.getValidators();
      return { validators };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const capturedAt = new Date().toISOString();
      const active = obs.validators.filter((v) => !v.jailed);

      const tokensBig = active.map((v) => BigInt(v.tokens || "0"));
      const total = tokensBig.reduce((acc, t) => acc + t, 0n);
      const sortedDesc = [...tokensBig].sort((a, b) => (a > b ? -1 : a < b ? 1 : 0));

      // Convert uregen → JS number without pre-dividing: integer
      // division by 1_000_000n floors validators with less than 1
      // REGEN to zero and loses every fractional REGEN for larger
      // stakes, which is load-bearing for Gini. Number.MAX_SAFE_INTEGER
      // is 2^53-1 ≈ 9e15 uregen ≈ 9 trillion REGEN, so the whole
      // uregen value fits safely — no pre-scaling needed.
      const tokensNum = active.map((v) => Number(BigInt(v.tokens || "0")));
      const sortedAscNum = [...tokensNum].sort((a, b) => a - b);

      const nakamoto = nakamotoCoefficient(sortedDesc, total);
      const gini = giniIndex(sortedAscNum);
      const top10 = topNSharePct(sortedDesc, total, 10);
      const top20 = topNSharePct(sortedDesc, total, 20);

      let largest = 0n;
      let largestMoniker = "(unknown)";
      let largestPct = 0;
      for (const v of active) {
        const t = BigInt(v.tokens || "0");
        if (t > largest) {
          largest = t;
          largestMoniker = v.description.moniker;
          largestPct = total > 0n ? Number((t * 10000n) / total) / 100 : 0;
        }
      }

      const health = classifyHealth(nakamoto, gini, largestPct);

      const snapshot: DecentralizationSnapshot = {
        validatorCount: active.length,
        bondedUregen: total.toString(),
        nakamotoCoefficient: nakamoto,
        giniIndex: gini,
        top10SharePct: top10,
        top20SharePct: top20,
        largestShareValidator: largestMoniker,
        largestSharePct: largestPct,
        health,
        capturedAt,
      };

      // Read the PREVIOUS snapshot BEFORE this cycle's save so the
      // narrative layer has a real trend reference.
      const prevRow = store.getLatestDecentralizationSnapshot();
      let previous: DecentralizationSnapshot | null = null;
      if (prevRow) {
        try {
          previous = JSON.parse(prevRow.snapshot) as DecentralizationSnapshot;
        } catch {
          previous = null;
        }
      }

      return { snapshot, previous };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const { snapshot, previous } = orientation;
      const report = await describeDecentralizationSnapshot(snapshot, previous);

      const alertLevel: AlertLevel =
        snapshot.health === "CRITICAL"
          ? "CRITICAL"
          : snapshot.health === "WARNING"
            ? "HIGH"
            : "NORMAL";

      return { snapshot, previous, report, alertLevel };
    },

    async act(decision: Decision): Promise<Actions> {
      const { snapshot, report, alertLevel } = decision;

      store.saveDecentralizationSnapshot({
        nakamoto: snapshot.nakamotoCoefficient,
        gini: snapshot.giniIndex,
        top10Pct: snapshot.top10SharePct,
        health: snapshot.health,
        snapshot: JSON.stringify(snapshot),
      });

      await output({
        workflow: "WF-VM-03",
        subjectId: "network",
        title: `${snapshot.health} — nakamoto ${snapshot.nakamotoCoefficient}, gini ${snapshot.giniIndex.toFixed(3)}`,
        content: report,
        alertLevel,
        timestamp: new Date(),
      });

      return {
        saved: 1,
        alertsSent: alertLevel !== "NORMAL" ? 1 : 0,
      };
    },
  };
}
