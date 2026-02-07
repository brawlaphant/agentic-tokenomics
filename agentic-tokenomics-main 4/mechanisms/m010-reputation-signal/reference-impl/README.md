# m010 reference implementation (v0 advisory)

This folder provides a **canonical computation** for m010 outputs so that different agents/runners
produce consistent numbers.

## Inputs
An input object:

- `as_of` (ISO-8601 string, Z-suffixed)
- `events[]` where each event includes:
  - `timestamp` (ISO-8601 Z)
  - `subject_type`, `subject_id`
  - `endorsement_level` (1..5)
  - `evidence.koi_links[]`
  - `evidence.ledger_refs[]`

## Outputs
### KPI block
- `signals_emitted` = count(events)
- `subjects_touched` = unique `(subject_type, subject_id)`
- `evidence_coverage_rate` = fraction of events with **both** KOI and Ledger evidence
- `median_event_latency_hours` = median(as_of - timestamp) in hours

### Score (optional)
A normalized `reputation_score_0_1` computed as:
1) per-event weight = endorsement_level / 5
2) time decay = exp(-lambda * age_hours) where lambda corresponds to ~14 day half-life
3) score = weighted average of decayed weights, normalized to [0,1]

This is advisory and intended for digest/reporting only (no enforcement).
