import { median } from "./m010_kpi.js";

/**
 * Exponential decay with ~14 day half-life by default.
 * halfLifeHours = 14 * 24 = 336 hours
 * lambda = ln(2) / halfLifeHours
 */
export function computeM010Score({ as_of, events, halfLifeHours = 336 }) {
  const asOf = new Date(as_of);
  const evs = events ?? [];
  if (!evs.length) return { reputation_score_0_1: 0.0 };

  const lambda = Math.log(2) / halfLifeHours;

  let wSum = 0;
  let dSum = 0;

  for (const e of evs) {
    const ts = new Date(e.timestamp);
    const ageH = (asOf - ts) / (1000*60*60);
    const decay = Math.exp(-lambda * Math.max(0, ageH));
    const w = Math.max(0, Math.min(1, (e.endorsement_level ?? 0) / 5));
    wSum += w * decay;
    dSum += decay;
  }

  const score = dSum ? (wSum / dSum) : 0.0;
  return { reputation_score_0_1: Number(Math.max(0, Math.min(1, score)).toFixed(4)) };
}
