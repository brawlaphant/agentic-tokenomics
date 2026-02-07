# m010 output schemas

These JSON Schemas define **canonical output shapes** for m010 (Reputation Signal) artifacts.

## Files
- `m010_kpi.schema.json` — schema for the KPI JSON block emitted by agents/digests.
- `m010_signal.schema.json` — schema for individual signal items (evidence + subject + timestamp).

## Notes
- These schemas are intended for **validation** and consistency across repos (Heartbeat, agent skills, etc.).
- v0 is advisory-only: schemas describe outputs, not enforcement.
