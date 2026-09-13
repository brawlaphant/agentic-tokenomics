/**
 * Shared numeric and parsing helpers used by more than one workflow.
 *
 * These started out duplicated across the three WF-MM workflows.
 * Centralizing them here is the DRY fix asked for by the PR #99
 * Gemini review, and it gives the unit-test suite a single source of
 * truth to pin so a regression in one workflow cannot silently diverge
 * from the rest.
 */

/**
 * Deterministic statistical median. Returns 0 for empty input so
 * callers do not need a special-case branch upstream — an empty
 * sample set unambiguously means "no signal" in every callsite.
 */
export function median(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0
    ? (sorted[mid - 1]! + sorted[mid]!) / 2
    : sorted[mid]!;
}

/**
 * Sample standard deviation (n-1 denominator). Returns 0 for
 * sample sizes below 2 because a single observation carries no
 * dispersion information.
 */
export function stddev(values: number[], mean: number): number {
  if (values.length <= 1) return 0;
  const sumSq = values.reduce((acc, v) => acc + (v - mean) ** 2, 0);
  return Math.sqrt(sumSq / (values.length - 1));
}

/**
 * Extract the credit class identifier from a Regen batch denom.
 *
 * Regen batch denoms follow the shape
 *   `<classId>-<projectId>-<startDate>-<endDate>-<serial>`
 * (for example, `C01-001-20240101-20241231-001`). The class id is
 * always the leading token before the first dash. Strings without a
 * dash are returned unchanged.
 */
export function classIdFromBatchDenom(denom: string): string {
  const idx = denom.indexOf("-");
  return idx > 0 ? denom.slice(0, idx) : denom;
}

/**
 * Whether a Cosmos denom corresponds to a USD-pegged stablecoin that
 * this MVP is willing to treat as 1:1 USD for pricing. Used to
 * filter out non-USD sell orders so they do not pollute the baseline
 * with arbitrary 1:1-valued noise.
 */
export function isUsdStableDenom(denom: string): boolean {
  const d = denom.toLowerCase();
  return d.includes("usdc") || d.includes("usdt") || d.includes("dai");
}

/**
 * Demand index on a bounded 0-100 scale. Inputs are rolling and a
 * class with no trailing activity gets 0. The index is intentionally
 * simple — it exists so the narrative layer has a single number to
 * anchor the "demand up / demand down" story. Defined here so the
 * unit-test suite can import it without pulling in the workflow's
 * Store singleton (which opens SQLite on load).
 */
export function computeDemandIndex(
  totalQuantity: number,
  retirementCount: number,
  uniqueRetirees: number
): number {
  const volumeComponent = Math.min(60, Math.log10(Math.max(1, totalQuantity)) * 20);
  const countComponent = Math.min(20, retirementCount * 2);
  const breadthComponent = Math.min(20, uniqueRetirees * 4);
  return Math.round(volumeComponent + countComponent + breadthComponent);
}
