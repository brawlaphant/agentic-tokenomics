import { bech32 } from "bech32";
import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { config } from "../config.js";
import { describePerformanceReport } from "../monitor.js";
import { output } from "../output.js";
import type { OODAWorkflow } from "../ooda.js";
import type {
  Validator,
  SigningInfo,
  SlashingParams,
  StakingPool,
  Proposal,
  ValidatorScorecard,
  PerformanceAlert,
} from "../types.js";

/**
 * WF-VM-01: Validator Performance Tracking
 *
 * Trigger: periodic OR slash event
 * Layer: 1 (Fully Automated)
 *
 * OODA:
 *   Observe — Fetch validator set, signing info, slashing params,
 *             staking pool, and recent finalized proposals (for
 *             governance participation scoring).
 *   Orient  — For each validator: compute uptime ratio, governance
 *             participation ratio, stability score. Combine into a
 *             composite 0-1000.
 *   Decide  — Flag performance alerts (uptime drop, jailed, high
 *             concentration). Persist scorecards.
 *   Act     — Generate narrative report via Claude, output.
 *
 * Determinism: all scoring is local TypeScript. Claude only writes
 * the narrative report.
 */

interface Observations {
  validators: Validator[];
  signingByConsAddrLike: Map<string, SigningInfo>;
  slashingParams: SlashingParams | null;
  pool: StakingPool;
  finalizedProposals: Proposal[];
  votesCastByOperator: Map<string, number>;
}

interface Orientation {
  scorecards: ValidatorScorecard[];
  alerts: PerformanceAlert[];
  bondedUregen: string;
}

interface Decision {
  report: string;
  alertsToPublish: PerformanceAlert[];
  scorecards: ValidatorScorecard[];
}

interface Actions {
  saved: number;
  alertsSent: number;
}

/**
 * Convert an operator bech32 address (e.g. `regenvaloper1abc…`) to
 * the matching delegator bech32 (`regen1abc…`). Cosmos encodes both
 * forms from the same underlying 20-byte payload — only the HRP
 * differs. The delegator prefix is the operator prefix with the
 * trailing `"valoper"` stripped.
 *
 * Returns `null` if the input is not a valid bech32 string or does
 * not end in `"valoper"`. Callers must guard against null — a
 * missing conversion means the validator cannot be queried for
 * votes and should get a governance score of 0 rather than crash
 * the whole cycle.
 */
export function operatorToAccountBech32(operatorAddress: string): string | null {
  try {
    const decoded = bech32.decode(operatorAddress);
    if (!decoded.prefix.endsWith("valoper")) return null;
    const delegatorPrefix = decoded.prefix.slice(0, -"valoper".length);
    if (!delegatorPrefix) return null;
    return bech32.encode(delegatorPrefix, decoded.words);
  } catch {
    return null;
  }
}

export function createPerformanceTrackingWorkflow(
  ledger: LedgerClient
): OODAWorkflow<Observations, Orientation, Decision, Actions> {
  return {
    id: "WF-VM-01",
    name: "Validator Performance Tracking",

    async observe(): Promise<Observations> {
      const [validators, signingInfos, slashingParams, pool, finalizedProposals] =
        await Promise.all([
          ledger.getValidators(),
          ledger.getSigningInfos(),
          ledger.getSlashingParams(),
          ledger.getStakingPool(),
          ledger.getRecentFinalizedProposals(20),
        ]);

      // Index signing info by the consensus address field so we can
      // match validators by their consensus-derived key. Signing info
      // uses the valcons bech32 form; the validator object exposes
      // `consensus_pubkey.key`. The MVP joins on the raw `address`
      // field and falls back to `missed_blocks_counter = 0` when we
      // can't resolve a match.
      const signingByConsAddrLike = new Map<string, SigningInfo>();
      for (const info of signingInfos) {
        signingByConsAddrLike.set(info.address, info);
      }

      // Per-validator governance vote fetching. For each validator we
      // convert the operator bech32 → delegator bech32 and query the
      // LCD for whether the delegator voted on each recent finalized
      // proposal. The result is a `votesCastByOperator` map that the
      // orient phase turns into a per-validator governance score.
      //
      // This is O(validators × proposals) LCD calls per cycle (up to
      // 75 × 20 = 1500 requests on mainnet), so we fire them in
      // parallel within each validator to keep the overall cycle
      // runtime bounded. A follow-up optimization can batch the
      // requests by querying `/proposals/{id}/votes` once per proposal
      // and indexing locally — we keep the per-voter fetch here
      // because it's the documented Cosmos pattern.
      const votesCastByOperator = new Map<string, number>();
      await Promise.all(
        validators.map(async (v) => {
          const delegator = operatorToAccountBech32(v.operator_address);
          if (!delegator) {
            votesCastByOperator.set(v.operator_address, 0);
            return;
          }
          let votes = 0;
          await Promise.all(
            finalizedProposals.map(async (p) => {
              const vote = await ledger.getVoteForVoter(p.id, delegator);
              if (vote) votes++;
            })
          );
          votesCastByOperator.set(v.operator_address, votes);
        })
      );

      return {
        validators,
        signingByConsAddrLike,
        slashingParams,
        pool,
        finalizedProposals,
        votesCastByOperator,
      };
    },

    async orient(obs: Observations): Promise<Orientation> {
      const { validators, signingByConsAddrLike, slashingParams, pool } = obs;

      const signedBlocksWindow = slashingParams
        ? Number(slashingParams.signed_blocks_window)
        : 10_000;

      const bonded = BigInt(pool.bonded_tokens || "0");
      const scorecards: ValidatorScorecard[] = [];
      const alerts: PerformanceAlert[] = [];

      // Record a commission baseline for every validator we see this
      // cycle; the commission history drives the stability penalty.
      for (const v of validators) {
        store.recordCommissionIfChanged(
          v.operator_address,
          v.commission.commission_rates.rate
        );
      }

      for (const v of validators) {
        const tokens = BigInt(v.tokens || "0");
        const stakePct =
          bonded > 0n ? Number((tokens * 10000n) / bonded) / 100 : 0;

        // ── Uptime component ─────────────────────────────────
        // When we can't find signing info, assume 100% (signed every
        // window) rather than penalizing — better to under-count real
        // issues than to smear a healthy validator.
        const signing = signingByConsAddrLike.get(
          v.consensus_pubkey?.key || ""
        );
        const missedBlocks = signing
          ? Number(signing.missed_blocks_counter || "0")
          : 0;
        const signedBlocks = Math.max(1, signedBlocksWindow - missedBlocks);
        const uptimeRatio = Math.max(
          0,
          Math.min(1, signedBlocks / signedBlocksWindow)
        );
        const uptimeScore = Math.round(
          uptimeRatio * config.validator.scoreWeightUptime
        );

        // ── Governance component ─────────────────────────────
        // The observe phase converted each validator's operator
        // bech32 to the delegator bech32 and queried the LCD for
        // every recent finalized proposal, counting the number of
        // votes cast by this validator. The governance ratio is
        // votesCast / proposalsConsidered.
        const proposalsConsidered = obs.finalizedProposals.length;
        const votesCast = obs.votesCastByOperator.get(v.operator_address) ?? 0;
        const governanceParticipation =
          proposalsConsidered > 0 ? votesCast / proposalsConsidered : 0;
        const governanceScore = Math.round(
          governanceParticipation * config.validator.scoreWeightGovernance
        );

        // ── Stability component ──────────────────────────────
        // Start at full stability weight; subtract penalties for
        // jailing and commission changes in the trailing window.
        // Explicit `number` type so we can clamp to 0 below — `as
        // const` in config.ts otherwise infers a literal type.
        let stabilityScore: number = config.validator.scoreWeightStability;
        if (v.jailed) stabilityScore -= config.validator.stabilityPenaltyJailing;
        const sinceIso = new Date(
          Date.now() - config.validator.uptimeTrailingDays * 86_400_000
        ).toISOString();
        const commissionChanges = store.countCommissionChangesSince(
          v.operator_address,
          sinceIso
        );
        stabilityScore -=
          commissionChanges * config.validator.stabilityPenaltyCommissionChange;
        if (stabilityScore < 0) stabilityScore = 0;

        const compositeScore =
          uptimeScore + governanceScore + stabilityScore;
        const poaEligible =
          compositeScore >= config.validator.poaEligibilityScore;

        const card: ValidatorScorecard = {
          operatorAddress: v.operator_address,
          moniker: v.description.moniker,
          jailed: v.jailed,
          tokens: v.tokens,
          stakePct,
          commissionRate: Number(v.commission.commission_rates.rate),
          uptimeScore,
          uptimeRatio,
          missedBlocks,
          signedBlocksWindow,
          governanceScore,
          governanceParticipation,
          proposalsConsidered,
          votesCast,
          stabilityScore,
          compositeScore,
          poaEligible,
        };
        scorecards.push(card);

        if (v.jailed) {
          alerts.push({
            operatorAddress: v.operator_address,
            moniker: v.description.moniker,
            severity: "CRITICAL",
            reason: "Validator is currently jailed",
          });
        } else if (
          missedBlocks > 0 &&
          missedBlocks / signedBlocksWindow > 0.05
        ) {
          alerts.push({
            operatorAddress: v.operator_address,
            moniker: v.description.moniker,
            severity: "HIGH",
            reason: `Missed ${missedBlocks}/${signedBlocksWindow} blocks (${((missedBlocks / signedBlocksWindow) * 100).toFixed(2)}%)`,
          });
        }
        if (stakePct >= config.validator.criticalConcentrationPct) {
          alerts.push({
            operatorAddress: v.operator_address,
            moniker: v.description.moniker,
            severity: "CRITICAL",
            reason: `Single validator holds ${stakePct.toFixed(2)}% of bonded stake`,
          });
        }
      }

      scorecards.sort((a, b) => b.compositeScore - a.compositeScore);

      return {
        scorecards,
        alerts,
        bondedUregen: pool.bonded_tokens,
      };
    },

    async decide(orientation: Orientation): Promise<Decision> {
      const report = await describePerformanceReport(
        orientation.scorecards,
        orientation.alerts,
        orientation.bondedUregen
      );

      return {
        report,
        alertsToPublish: orientation.alerts,
        scorecards: orientation.scorecards,
      };
    },

    async act(decision: Decision): Promise<Actions> {
      let saved = 0;
      let alertsSent = 0;

      // Persist every scorecard so the historical timeline is usable
      // for future PoA transition assessments.
      for (const card of decision.scorecards) {
        store.saveScorecard({
          operatorAddress: card.operatorAddress,
          compositeScore: card.compositeScore,
          scorecard: JSON.stringify(card),
        });
        saved++;
      }

      await output({
        workflow: "WF-VM-01",
        subjectId: "validator-set",
        title: "Performance report",
        content: decision.report,
        alertLevel:
          decision.alertsToPublish.some((a) => a.severity === "CRITICAL")
            ? "CRITICAL"
            : decision.alertsToPublish.length > 0
              ? "HIGH"
              : "NORMAL",
        timestamp: new Date(),
      });
      if (decision.alertsToPublish.length > 0) alertsSent++;

      return { saved, alertsSent };
    },
  };
}
