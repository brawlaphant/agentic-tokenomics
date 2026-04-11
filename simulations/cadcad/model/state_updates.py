"""
State update functions for the Regen M012-M015 cadCAD simulation.

Each function maps policy outputs to state variable updates.
Based on docs/economics/economic-simulation-spec.md Section 2.4.

cadCAD state update signature:
    def s_<name>(params, substep, state_history, prev_state, policy_input):
        return ('<variable_name>', new_value)
"""


# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------

def _compute_periods_near_equilibrium(M_t, B_t, S, threshold, prev_periods):
    """Compute the updated consecutive near-equilibrium period count.

    This is the single source of truth for the near-equilibrium check:
        |M - B| < threshold * S

    Used by both ``s_supply_state`` and ``s_periods_near_equilibrium`` so that
    the logic is defined in exactly one place.
    """
    if S > 0 and abs(M_t - B_t) < threshold * S:
        return prev_periods + 1
    return 0


# ---------------------------------------------------------------------------
# Supply state updates (M012)
# ---------------------------------------------------------------------------

def s_supply(params, substep, state_history, prev_state, policy_input):
    """Update circulating supply from M012 mint/burn."""
    return ('S', policy_input['new_S'])


def s_minted(params, substep, state_history, prev_state, policy_input):
    """Record tokens minted this period."""
    return ('M_t', policy_input['M_t'])


def s_burned(params, substep, state_history, prev_state, policy_input):
    """Record tokens burned this period."""
    return ('B_t', policy_input['B_t'])


def s_cumulative_minted(params, substep, state_history, prev_state, policy_input):
    """Accumulate lifetime minted tokens."""
    return ('cumulative_minted', prev_state['cumulative_minted'] + policy_input['M_t'])


def s_cumulative_burned(params, substep, state_history, prev_state, policy_input):
    """Accumulate lifetime burned tokens."""
    return ('cumulative_burned', prev_state['cumulative_burned'] + policy_input['B_t'])


def s_r_effective(params, substep, state_history, prev_state, policy_input):
    """Update effective regrowth rate."""
    return ('r_effective', policy_input['r_effective'])


def s_supply_state(params, substep, state_history, prev_state, policy_input):
    """Update supply state machine (TRANSITION -> DYNAMIC -> EQUILIBRIUM)."""
    M_t = policy_input['M_t']
    B_t = policy_input['B_t']
    S = policy_input['new_S']
    threshold = params['equilibrium_threshold']
    required_periods = params['equilibrium_periods']

    current_state = prev_state['supply_state']

    periods_near_eq = _compute_periods_near_equilibrium(
        M_t, B_t, S, threshold, prev_state['periods_near_equilibrium']
    )

    # State transitions
    if current_state == 'TRANSITION':
        if B_t > 0:  # First burn occurred
            current_state = 'DYNAMIC'
    elif current_state == 'DYNAMIC':
        if periods_near_eq >= required_periods:
            current_state = 'EQUILIBRIUM'
    elif current_state == 'EQUILIBRIUM':
        if S > 0 and abs(M_t - B_t) >= threshold * S:
            current_state = 'DYNAMIC'
            periods_near_eq = 0

    return ('supply_state', current_state)


def s_periods_near_equilibrium(params, substep, state_history, prev_state, policy_input):
    """Track consecutive near-equilibrium periods.

    Delegates to ``_compute_periods_near_equilibrium`` to keep the
    equilibrium check in a single place.
    """
    return ('periods_near_equilibrium', _compute_periods_near_equilibrium(
        policy_input['M_t'],
        policy_input['B_t'],
        policy_input['new_S'],
        params['equilibrium_threshold'],
        prev_state['periods_near_equilibrium'],
    ))


# ---------------------------------------------------------------------------
# Fee and pool state updates (M013)
# ---------------------------------------------------------------------------

def s_total_fees_collected(params, substep, state_history, prev_state, policy_input):
    """Update total fees collected this period (REGEN)."""
    return ('total_fees_collected', policy_input['total_fees_regen'])


def s_total_fees_usd(params, substep, state_history, prev_state, policy_input):
    """Update total fees collected this period (USD)."""
    return ('total_fees_usd', policy_input['total_fees_usd'])


def s_burn_pool(params, substep, state_history, prev_state, policy_input):
    """Update burn pool balance."""
    return ('burn_pool_balance', policy_input['burn_allocation'])


def s_validator_fund(params, substep, state_history, prev_state, policy_input):
    """Update validator fund balance."""
    return ('validator_fund_balance', policy_input['validator_allocation'])


def s_community_pool(params, substep, state_history, prev_state, policy_input):
    """Update community pool balance."""
    return ('community_pool_balance', policy_input['community_allocation'])


def s_agent_infra(params, substep, state_history, prev_state, policy_input):
    """Update agent infrastructure fund balance."""
    return ('agent_infra_balance', policy_input['agent_allocation'])


def s_cumulative_fees(params, substep, state_history, prev_state, policy_input):
    """Accumulate lifetime fee revenue."""
    return ('cumulative_fees', prev_state['cumulative_fees'] + policy_input['total_fees_regen'])


# ---------------------------------------------------------------------------
# Validator state updates (M014)
# ---------------------------------------------------------------------------

def s_active_validators(params, substep, state_history, prev_state, policy_input):
    """Update active validator count."""
    return ('active_validators', policy_input['new_active_validators'])


def s_validator_income_period(params, substep, state_history, prev_state, policy_input):
    """Update per-validator period income."""
    return ('validator_income_period', policy_input['validator_income_period'])


def s_validator_income_annual(params, substep, state_history, prev_state, policy_input):
    """Update annualized per-validator income (REGEN)."""
    return ('validator_income_annual', policy_input['validator_income_annual'])


def s_validator_income_usd(params, substep, state_history, prev_state, policy_input):
    """Update annualized per-validator income (USD)."""
    return ('validator_income_usd', policy_input['validator_income_usd'])


# ---------------------------------------------------------------------------
# Reward state updates (M015)
# ---------------------------------------------------------------------------

def s_stability_committed(params, substep, state_history, prev_state, policy_input):
    """Update total stability tier commitments."""
    return ('stability_committed', policy_input['new_stability_committed'])


def s_stability_allocation(params, substep, state_history, prev_state, policy_input):
    """Update this period's stability allocation."""
    return ('stability_allocation', policy_input['stability_allocation'])


def s_activity_pool(params, substep, state_history, prev_state, policy_input):
    """Update this period's activity-based pool."""
    return ('activity_pool', policy_input['activity_pool'])


def s_total_activity_score(params, substep, state_history, prev_state, policy_input):
    """Update aggregate activity score."""
    return ('total_activity_score', policy_input['total_activity_score'])


def s_stability_utilization(params, substep, state_history, prev_state, policy_input):
    """Update stability tier utilization ratio."""
    return ('stability_utilization', policy_input['stability_utilization'])


def s_reward_per_unit_activity(params, substep, state_history, prev_state, policy_input):
    """Update reward per unit of activity score."""
    return ('reward_per_unit_activity', policy_input['reward_per_unit_activity'])


# ---------------------------------------------------------------------------
# Market state updates
# ---------------------------------------------------------------------------

def s_credit_volume_weekly(params, substep, state_history, prev_state, policy_input):
    """Update weekly credit volume."""
    return ('credit_volume_weekly_usd', policy_input['total_volume_usd'])


def s_regen_price(params, substep, state_history, prev_state, policy_input):
    """Update REGEN price."""
    return ('regen_price_usd', policy_input['new_regen_price_usd'])


def s_ecological_multiplier(params, substep, state_history, prev_state, policy_input):
    """Update ecological multiplier (passthrough; set by stress tests)."""
    return ('ecological_multiplier', prev_state['ecological_multiplier'])


# ---------------------------------------------------------------------------
# Transaction state updates
# ---------------------------------------------------------------------------

def s_issuance_count(params, substep, state_history, prev_state, policy_input):
    return ('issuance_count', policy_input['issuance_count'])


def s_trade_count(params, substep, state_history, prev_state, policy_input):
    return ('trade_count', policy_input['trade_count'])


def s_retirement_count(params, substep, state_history, prev_state, policy_input):
    return ('retirement_count', policy_input['retirement_count'])


def s_transfer_count(params, substep, state_history, prev_state, policy_input):
    return ('transfer_count', policy_input['transfer_count'])


def s_issuance_value(params, substep, state_history, prev_state, policy_input):
    return ('issuance_value_usd', policy_input['issuance_value_usd'])


def s_trade_value(params, substep, state_history, prev_state, policy_input):
    return ('trade_value_usd', policy_input['trade_value_usd'])


def s_retirement_value(params, substep, state_history, prev_state, policy_input):
    return ('retirement_value_usd', policy_input['retirement_value_usd'])


def s_transfer_value(params, substep, state_history, prev_state, policy_input):
    return ('transfer_value_usd', policy_input['transfer_value_usd'])


def s_total_volume(params, substep, state_history, prev_state, policy_input):
    return ('total_volume_usd', policy_input['total_volume_usd'])
