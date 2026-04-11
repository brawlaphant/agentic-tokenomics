"""
Policy functions for the Regen M012-M015 cadCAD simulation.

Seven policy functions (P1-P7) implement the economic logic specified in
docs/economics/economic-simulation-spec.md Section 2.3.

Policy functions compute actions (signals) based on current state. They run
before state update functions each epoch.
"""

import numpy as np


# ---------------------------------------------------------------------------
# P1: Credit Market Activity
# ---------------------------------------------------------------------------

def p_credit_market(params, substep, state_history, prev_state):
    """
    Generate credit market activity for this epoch.

    Volume is driven by:
    - Number of active agents (issuers, buyers, retirees)
    - Per-agent transaction intensity
    - Average transaction value (lognormal distribution)
    - Trend growth (volume_growth_rate per period)
    """
    timestep = prev_state['timestep']

    num_issuers = prev_state['num_issuers']
    num_buyers = prev_state['num_buyers']
    num_retirees = prev_state['num_retirees']
    num_wash_traders = prev_state['num_wash_traders']

    # Legitimate transaction counts
    issuance_count = max(1, int(num_issuers * params['issuance_intensity']))
    trade_count = max(1, int(num_buyers * params['trade_intensity']))
    retirement_count = max(1, int(num_retirees * params['retirement_intensity']))
    transfer_count = max(1, int((num_issuers + num_buyers) * params['transfer_intensity']))

    # Wash trading (adversarial)
    wash_trade_count = int(num_wash_traders * params['wash_trade_intensity'])

    avg_value = params['avg_credit_value_usd']
    sigma = params['credit_value_sigma']

    # Apply volume growth trend
    growth_factor = (1.0 + params['volume_growth_rate']) ** timestep
    scaled_avg = avg_value * growth_factor

    # Transaction values from lognormal distribution
    issuance_value = float(np.sum(np.random.lognormal(
        mean=np.log(max(scaled_avg * 2, 1)), sigma=sigma, size=issuance_count
    )))
    trade_value = float(np.sum(np.random.lognormal(
        mean=np.log(max(scaled_avg, 1)), sigma=sigma, size=trade_count
    )))
    retirement_value = float(np.sum(np.random.lognormal(
        mean=np.log(max(scaled_avg * 0.8, 1)), sigma=sigma, size=retirement_count
    )))
    transfer_value = float(np.sum(np.random.lognormal(
        mean=np.log(max(scaled_avg * 0.5, 1)), sigma=sigma, size=transfer_count
    )))

    # Wash trading volume
    if wash_trade_count > 0:
        wash_value = float(np.sum(np.random.lognormal(
            mean=np.log(max(scaled_avg * 0.3, 1)), sigma=sigma,
            size=max(1, wash_trade_count)
        )))
    else:
        wash_value = 0.0

    total_volume = issuance_value + trade_value + retirement_value + transfer_value + wash_value

    return {
        'issuance_count': issuance_count,
        'trade_count': trade_count + wash_trade_count,
        'retirement_count': retirement_count,
        'transfer_count': transfer_count,
        'issuance_value_usd': issuance_value,
        'trade_value_usd': trade_value + wash_value,
        'retirement_value_usd': retirement_value,
        'transfer_value_usd': transfer_value,
        'wash_trade_value_usd': wash_value,
        'total_volume_usd': total_volume,
    }


# ---------------------------------------------------------------------------
# P2: Fee Collection (M013)
# ---------------------------------------------------------------------------

def p_fee_collection(params, substep, state_history, prev_state, policy_input):
    """
    Calculate fees from credit market activity.

    fee = max(value * rate_bps / 10000, min_fee per transaction)

    All fees collected in REGEN using current REGEN/USD price.
    """
    regen_price = max(prev_state['regen_price_usd'], 0.001)

    issuance_rate = params['fee_rate_issuance_bps']
    trade_rate = params['fee_rate_trade_bps']
    retirement_rate = params['fee_rate_retirement_bps']
    transfer_rate = params['fee_rate_transfer_bps']

    # Fees in USD
    issuance_fees_usd = policy_input['issuance_value_usd'] * issuance_rate / 10_000
    trade_fees_usd = policy_input['trade_value_usd'] * trade_rate / 10_000
    retirement_fees_usd = policy_input['retirement_value_usd'] * retirement_rate / 10_000
    transfer_fees_usd = policy_input['transfer_value_usd'] * transfer_rate / 10_000

    total_fees_usd = issuance_fees_usd + trade_fees_usd + retirement_fees_usd + transfer_fees_usd

    # Convert to REGEN
    total_fees_regen = total_fees_usd / regen_price

    # Apply minimum fee floor per transaction
    min_fee_regen = params['min_fee_regen']
    total_transactions = (
        policy_input['issuance_count'] + policy_input['trade_count'] +
        policy_input['retirement_count'] + policy_input['transfer_count']
    )
    min_fee_total = total_transactions * min_fee_regen
    total_fees_regen = max(total_fees_regen, min_fee_total)

    return {
        'total_fees_regen': total_fees_regen,
        'total_fees_usd': total_fees_usd,
    }


# ---------------------------------------------------------------------------
# P3: Fee Distribution (M013)
# ---------------------------------------------------------------------------

def p_fee_distribution(params, substep, state_history, prev_state, policy_input):
    """
    Split fees to burn, validator, community, and agent pools.

    Invariant: burn_share + validator_share + community_share + agent_share = 1.0
    """
    fees = policy_input['total_fees_regen']

    burn_share = params['burn_share']
    validator_share = params['validator_share']
    community_share = params['community_share']
    agent_share = params['agent_share']

    # Validate share sum unity (within floating point tolerance)
    share_sum = burn_share + validator_share + community_share + agent_share
    assert abs(share_sum - 1.0) < 1e-6, \
        f"Share Sum Unity violated: {share_sum}"

    return {
        'burn_allocation': fees * burn_share,
        'validator_allocation': fees * validator_share,
        'community_allocation': fees * community_share,
        'agent_allocation': fees * agent_share,
    }


# ---------------------------------------------------------------------------
# P4: Mint/Burn Computation (M012)
# ---------------------------------------------------------------------------

def p_mint_burn(params, substep, state_history, prev_state, policy_input):
    """
    M012 supply algorithm:

    M[t] = r * max(0, C - S[t])    (regrowth, floored at 0 when S > C)
    B[t] = burn_allocation          (from M013 fee routing)

    r = r_base * effective_multiplier * ecological_multiplier

    effective_multiplier depends on PoA phase:
    - Pre-PoA:  clamp(1 + staking_ratio, 1.0, 2.0)
    - Post-PoA: clamp(1 + stability_committed / S, 1.0, 2.0)
    """
    S = prev_state['S']
    C = params['hard_cap']
    r_base = params['base_regrowth_rate']

    # Effective multiplier (phase-gated)
    if params['poa_active']:
        stability_ratio = prev_state['stability_committed'] / max(S, 1.0)
        effective_multiplier = 1.0 + stability_ratio
    else:
        staking_ratio = params['staking_ratio']
        effective_multiplier = 1.0 + staking_ratio

    effective_multiplier = min(max(effective_multiplier, 1.0), 2.0)

    # Ecological multiplier
    ecological_multiplier = prev_state['ecological_multiplier']
    ecological_multiplier = max(ecological_multiplier, 0.0)

    # Composite regrowth rate
    r = r_base * effective_multiplier * ecological_multiplier
    r = min(r, params['max_regrowth_rate'])

    # Minting: only when S < C (gap > 0)
    gap = C - S
    if gap > 0:
        M_t = r * gap
    else:
        M_t = 0.0  # No minting above cap

    # Burning: from fee revenue
    B_t = policy_input['burn_allocation']

    # Enforce supply bounds
    new_S = S + M_t - B_t

    # Non-negative supply invariant
    if new_S < 0:
        B_t = S + M_t
        new_S = 0.0

    # Cap inviolability invariant
    if new_S > C:
        M_t = max(0, C - S + B_t)
        new_S = min(S + M_t - B_t, C)

    return {
        'M_t': M_t,
        'B_t': B_t,
        'new_S': new_S,
        'r_effective': r,
    }


# ---------------------------------------------------------------------------
# P5: Validator Compensation (M014)
# ---------------------------------------------------------------------------

def p_validator_compensation(params, substep, state_history, prev_state, policy_input):
    """
    Distribute validator fund to active validators.

    base_compensation = fund * (1 - bonus_share) / active_validators
    performance_bonus = fund * bonus_share / active_validators (avg)
    """
    validator_fund = policy_input['validator_allocation']
    active_validators = prev_state['active_validators']

    if active_validators == 0:
        return {
            'validator_income_period': 0.0,
            'validator_income_annual': 0.0,
            'validator_income_usd': 0.0,
        }

    bonus_share = params['validator_bonus_share']
    base_pool = validator_fund * (1.0 - bonus_share)
    bonus_pool = validator_fund * bonus_share

    base_per_validator = base_pool / active_validators
    avg_bonus = bonus_pool / active_validators

    income_per_period = base_per_validator + avg_bonus
    income_annual = income_per_period * params['periods_per_year']
    income_usd = income_annual * prev_state['regen_price_usd']

    return {
        'validator_income_period': income_per_period,
        'validator_income_annual': income_annual,
        'validator_income_usd': income_usd,
    }


# ---------------------------------------------------------------------------
# P6: Contribution Rewards Distribution (M015)
# ---------------------------------------------------------------------------

def p_contribution_rewards(params, substep, state_history, prev_state, policy_input):
    """
    M015 reward distribution:

    1. Stability tier: min(commitments * rate / periods_per_year, community_inflow * max_share)
    2. Activity pool: community_inflow - stability_allocation
    3. Activity scoring: weighted sum of participant activities
    """
    community_inflow = policy_input['community_allocation']
    stability_committed = prev_state['stability_committed']
    periods_per_year = params['periods_per_year']
    stability_rate = params['stability_annual_rate']
    max_stability_share = params['max_stability_share']

    # Stability tier obligation
    stability_obligation = stability_committed * stability_rate / periods_per_year
    stability_cap = community_inflow * max_stability_share
    stability_allocation = min(stability_obligation, stability_cap)

    # Activity pool (remaining after stability allocation)
    activity_pool = max(0.0, community_inflow - stability_allocation)

    # Aggregate activity scoring
    weights = params['activity_weights']
    total_score = (
        policy_input.get('issuance_value_usd', 0) * weights['purchase'] +
        policy_input.get('retirement_value_usd', 0) * weights['retirement'] +
        policy_input.get('trade_value_usd', 0) * weights['facilitation'] +
        prev_state['num_governance_participants'] * params['governance_vote_value'] * weights['governance'] +
        params['proposals_per_period'] * params['proposal_value'] * weights['proposals']
    )

    reward_per_unit = activity_pool / max(total_score, 1e-9) if total_score > 0 else 0.0
    stability_utilization = stability_allocation / max(stability_cap, 1e-9) if stability_cap > 0 else 0.0

    return {
        'stability_allocation': stability_allocation,
        'activity_pool': activity_pool,
        'total_activity_score': total_score,
        'reward_per_unit_activity': reward_per_unit,
        'stability_utilization': stability_utilization,
        'stability_shortfall': max(0.0, stability_obligation - stability_cap),
    }


# ---------------------------------------------------------------------------
# P7: Agent Population Dynamics
# ---------------------------------------------------------------------------

def p_agent_dynamics(params, substep, state_history, prev_state, policy_input):
    """
    Agent entry/exit based on economic signals:
    - Validators churn based on income adequacy
    - Stability holders enter/exit based on return attractiveness
    """
    # --- Validator dynamics ---
    active_validators = prev_state['active_validators']
    validator_income_usd = policy_input.get('validator_income_usd', 0.0)
    min_viable_income = params['min_viable_validator_income_usd']

    if validator_income_usd < min_viable_income:
        churn_probability = params['base_validator_churn'] * 1.5
    else:
        churn_probability = params['base_validator_churn'] * 0.5

    # Quarterly churn applied per period (divide by ~13 periods per quarter)
    period_churn_prob = min(churn_probability / 13.0, 1.0)
    validators_leaving = int(np.random.binomial(active_validators, period_churn_prob))
    validators_joining = int(np.random.poisson(params['validator_application_rate'] / 13.0))

    new_validators = max(
        params['min_validators'],
        min(params['max_validators'], active_validators - validators_leaving + validators_joining)
    )

    # --- Stability tier dynamics ---
    stability_committed = prev_state['stability_committed']
    stability_util = prev_state.get('stability_utilization', 0.0)
    stability_rate = params['stability_annual_rate']

    if stability_util >= 0.9:
        # Attractive: near-full returns being paid
        new_stability = float(np.random.poisson(params['stability_adoption_rate'])) * params['avg_stability_commitment']
    elif stability_util >= 0.5:
        new_stability = float(np.random.poisson(params['stability_adoption_rate'] * 0.5)) * params['avg_stability_commitment']
    else:
        new_stability = 0.0

    # Maturations (tokens unlocking)
    maturation_rate = 1.0 / max(params['avg_stability_lock_periods'], 1)
    maturations = stability_committed * maturation_rate

    # Early exits
    early_exits = stability_committed * params['stability_early_exit_rate']

    new_stability_committed = max(0.0, stability_committed + new_stability - maturations - early_exits)

    # --- Price dynamics (GBM with mean reversion) ---
    current_price = prev_state['regen_price_usd']
    mu = params['price_drift']
    sigma = params['price_volatility']
    mr_speed = params['price_mean_reversion_speed']
    mr_target = params['price_mean_reversion_target']

    # Mean-reverting GBM
    drift = mu + mr_speed * (np.log(mr_target) - np.log(max(current_price, 1e-6)))
    shock = sigma * np.random.normal()
    new_price = current_price * np.exp(drift + shock)
    new_price = max(0.001, min(new_price, 10.0))

    return {
        'new_active_validators': new_validators,
        'new_stability_committed': new_stability_committed,
        'validators_leaving': validators_leaving,
        'validators_joining': validators_joining,
        'new_regen_price_usd': new_price,
    }
