"""
State variables for the Regen M012-M015 cadCAD simulation.

All state variables with initial values as defined in the economic simulation spec
(docs/economics/economic-simulation-spec.md, Section 2.2).

Units:
  - Supply values in REGEN (not uregen). 1 REGEN = 1,000,000 uregen.
  - Monetary values in USD unless otherwise noted.
  - Time in epochs (1 epoch = 1 week).
"""

# ---------------------------------------------------------------------------
# Initial state vector
# ---------------------------------------------------------------------------

initial_state = {
    # === Supply State (M012) ===
    'S': 224_000_000.0,          # Current circulating supply (REGEN). Exceeds C at launch.
    'M_t': 0.0,                  # Tokens minted this period
    'B_t': 0.0,                  # Tokens burned this period
    'r_effective': 0.0,          # Current effective regrowth rate
    'supply_state': 'TRANSITION',  # {INFLATIONARY, TRANSITION, DYNAMIC, EQUILIBRIUM}
    'periods_near_equilibrium': 0, # Consecutive periods where |M-B| < threshold
    'cumulative_minted': 0.0,    # Lifetime tokens minted
    'cumulative_burned': 0.0,    # Lifetime tokens burned

    # === Fee and Pool State (M013) ===
    'total_fees_collected': 0.0,     # Fees collected this period (REGEN)
    'total_fees_usd': 0.0,          # Fees collected this period (USD)
    'burn_pool_balance': 0.0,        # Burn pool for this period
    'validator_fund_balance': 0.0,   # Validator fund for this period
    'community_pool_balance': 0.0,   # Community pool for this period
    'agent_infra_balance': 0.0,      # Agent infrastructure fund for this period
    'cumulative_fees': 0.0,          # Lifetime fee revenue (REGEN)

    # === Validator State (M014) ===
    'active_validators': 18,         # Current active validator count
    'validator_income_period': 0.0,  # Per-validator income this period (REGEN)
    'validator_income_annual': 0.0,  # Annualized per-validator income (REGEN)
    'validator_income_usd': 0.0,     # Annualized per-validator income (USD)

    # === Reward State (M015) ===
    'stability_committed': 0.0,      # Total REGEN in stability tier
    'stability_allocation': 0.0,     # This period's stability distribution (REGEN)
    'activity_pool': 0.0,            # This period's activity-based pool (REGEN)
    'total_activity_score': 0.0,     # Sum of all participant activity scores
    'stability_utilization': 0.0,    # stability_allocation / (community_inflow * max_stab)
    'reward_per_unit_activity': 0.0, # REGEN reward per unit of activity score

    # === Market and Ecological State ===
    'credit_volume_weekly_usd': 500_000.0,  # Weekly credit transaction volume (USD)
    'regen_price_usd': 0.05,                # REGEN/USD price
    'ecological_multiplier': 1.0,           # Ecological oracle input (1.0 = disabled v0)

    # === Transaction counts (per period) ===
    'issuance_count': 0,
    'trade_count': 0,
    'retirement_count': 0,
    'transfer_count': 0,

    # === Transaction values (per period, USD) ===
    'issuance_value_usd': 0.0,
    'trade_value_usd': 0.0,
    'retirement_value_usd': 0.0,
    'transfer_value_usd': 0.0,
    'total_volume_usd': 0.0,

    # === Agent Population State ===
    'num_issuers': 20,
    'num_buyers': 50,
    'num_retirees': 30,
    'num_holders': 500,
    'num_stability_holders': 0,
    'num_governance_participants': 40,
    'num_wash_traders': 0,
}
