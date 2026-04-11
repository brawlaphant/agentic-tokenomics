import { LedgerClient } from "../ledger.js";
import { store } from "../store.js";
import { config } from "../config.js";
import { describePerformanceReport } from "../monitor.js";
import { output } from "../output.js";
import {
  operatorToDelegator,
  consensusPubkeyToConsAddress,
} from "../bech32.js";
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

// operator→delegator bech32 conversion lives in `../bech32.js` so the
// performance-tracking workflow and the governance-vote tracker can
// share one implementation. See PR #81's review for why the old
// inline stub (which returned "") broke uptime and governance scoring.

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

      // Governance vote fetching — one LCD call per recent finalized
      // proposal rather than one per (validator × proposal). The old
      // O(validators × proposals) fan-out was up to ~1500 requests
      // per cycle on mainnet (75 validators × 20 proposals) and
      // would reliably trip rate limits on public LCD endpoints.
      //
      // We instead pull every vote on each proposal via
      // `/cosmos/gov/v1beta1/proposals/{id}/votes` (paginated), index
      // locally by delegator address, and count matches for each
      // validator's delegator-bech32. Cost is O(proposals) LCD calls
      // per cycle — 20 on mainnet — which is well inside any public
      // endpoint's budget.
      const votesByProposalThenVoter = new Map<
        string,
        Set<string>
      >();
      await Promise.all(
        finalizedProposals.map(async (p) => {
          const votes = await ledger.getVotesForProposal(p.id);
          const voters = new Set<string>();
          for (const vote of votes) {
            if (vote.voter) voters.add(vote.voter);
          }
          votesByProposalThenVoter.set(p.id, voters);
        })
      );

      const votesCastByOperator = new Map<string, number>();
      for (const v of validators) {
        const delegator = operatorToDelegator(v.operator_address);
        if (!delegator) {
          votesCastByOperator.set(v.operator_address, 0);
          continue;
        }
        let votes = 0;
        for (const voters of votesByProposalThenVoter.values()) {
          if (voters.has(delegator)) votes++;
        }
        votesCastByOperator.set(v.operator_address, votes);
      }

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

      // Compute the trailing-window cutoff once — it does not depend
      // on the validator, so there is no point recomputing it per
      // iteration the way the original loop did.
      const sinceIso = new Date(
        Date.now() - config.validator.uptimeTrailingDays * 86_400_000
      ).toISOString();

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
        // Derive the regenvalcons1… address from the validator's
        // consensus pubkey so it matches the keys in
        // signingByConsAddrLike (which are `info.address`, i.e. the
        // valcons bech32). Previously we were joining a base64
        // pubkey string against a bech32 address and the lookup
        // always missed — every validator was reporting 0 missed
        // blocks, which masked real downtime. When the derivation or
        // the lookup still fails we fall back to 100% uptime rather
        // than penalizing a validator we cannot classify.
        const pubkeyBase64 = v.consensus_pubkey?.key || "";
        const consAddr = consensusPubkeyToConsAddress(pubkeyBase64);
        const signing = consAddr
          ? signingByConsAddrLike.get(consAddr)
          : undefined;
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
