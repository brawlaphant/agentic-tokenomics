export function median(nums) {
  if (!nums.length) return null;
  const s = [...nums].sort((a,b) => a-b);
  const mid = Math.floor(s.length/2);
  return s.length % 2 ? s[mid] : (s[mid-1] + s[mid]) / 2;
}

export function computeM010KPI({ as_of, events }) {
  const asOf = new Date(as_of);
  const evs = events ?? [];
  const signals_emitted = evs.length;
  const subjects_touched = new Set(evs.map(e => `${e.subject_type}:${e.subject_id}`)).size;

  let bothEvidence = 0;
  const latencies = [];

  for (const e of evs) {
    const koiOk = (e.evidence?.koi_links ?? []).length > 0;
    const ledOk = (e.evidence?.ledger_refs ?? []).length > 0;
    if (koiOk && ledOk) bothEvidence += 1;

    const ts = new Date(e.timestamp);
    const hours = (asOf - ts) / (1000*60*60);
    if (Number.isFinite(hours)) latencies.push(hours);
  }

  const evidence_coverage_rate = signals_emitted ? bothEvidence / signals_emitted : 0.0;
  const median_event_latency_hours = median(latencies);

  return {
    mechanism_id: "m010",
    scope: "v0_advisory",
    as_of,
    signals_emitted,
    subjects_touched,
    evidence_coverage_rate: Number(evidence_coverage_rate.toFixed(4)),
    median_event_latency_hours: median_event_latency_hours === null ? null : Number(median_event_latency_hours.toFixed(2))
  };
}
