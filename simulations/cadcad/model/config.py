"""
cadCAD experiment configuration for the Regen M012-M015 simulation.

Defines Partial State Update Blocks (PSUBs) that wire policy functions
to state update functions, and builds cadCAD Configuration objects.

The simulation pipeline each epoch:
  1. P1: Credit market generates transaction activity
  2. P2: Fees collected from transactions (M013)
  3. P3: Fees distributed to 4 pools (M013)
  4. P4: Mint/burn computed from supply gap and burn pool (M012)
  5. P5: Validator compensation from validator fund (M014)
  6. P6: Contribution rewards from community pool (M015)
  7. P7: Agent population dynamics (entry/exit, price)
"""

from cadCAD.configuration import Experiment
from cadCAD.configuration.utils import config_sim

from model.state_variables import initial_state
from model.params import baseline_params
from model.policies import (
    p_credit_market,
    p_fee_collection,
    p_fee_distribution,
    p_mint_burn,
    p_validator_compensation,
    p_contribution_rewards,
    p_agent_dynamics,
)
from model.state_updates import (
    # Supply
    s_supply, s_minted, s_burned, s_cumulative_minted, s_cumulative_burned,
    s_r_effective, s_supply_state, s_periods_near_equilibrium,
    # Fees / pools
    s_total_fees_collected, s_total_fees_usd, s_burn_pool, s_validator_fund,
    s_community_pool, s_agent_infra, s_cumulative_fees,
    # Validators
    s_active_validators, s_validator_income_period, s_validator_income_annual,
    s_validator_income_usd,
    # Rewards
    s_stability_committed, s_stability_allocation, s_activity_pool,
    s_total_activity_score, s_stability_utilization, s_reward_per_unit_activity,
    # Market
    s_credit_volume_weekly, s_regen_price, s_ecological_multiplier,
    # Transactions
    s_issuance_count, s_trade_count, s_retirement_count, s_transfer_count,
    s_issuance_value, s_trade_value, s_retirement_value, s_transfer_value,
    s_total_volume,
)

import copy


# ---------------------------------------------------------------------------
# Composite policy wrappers
#
# cadCAD 0.5.x requires each PSUB to have a single policy function.  We
# compose the seven logical policies into three composite policy functions
# that correspond to the three PSUBs, each passing outputs forward via the
# returned signal dict.
# ---------------------------------------------------------------------------

def _composite_market_and_fees(params, substep, state_history, prev_state):
    """PSUB-1 policy: market activity + fee collection + fee distribution."""
    # P1: Credit market
    market = p_credit_market(params, substep, state_history, prev_state)

    # P2: Fee collection (depends on market output)
    fees = p_fee_collection(params, substep, state_history, prev_state, market)

    # P3: Fee distribution (depends on fee output)
    dist = p_fee_distribution(params, substep, state_history, prev_state, fees)

    # Merge all signals
    result = {}
    result.update(market)
    result.update(fees)
    result.update(dist)
    return result


def _composite_supply_and_compensation(params, substep, state_history, prev_state):
    """PSUB-2 policy: mint/burn + validator compensation + contribution rewards.

    This PSUB reads pool balances written by PSUB-1 from prev_state.
    We pass pool balances forward via a constructed policy_input.
    """
    # Read pool balances set by PSUB-1 state updates
    pool_input = {
        'burn_allocation': prev_state['burn_pool_balance'],
        'validator_allocation': prev_state['validator_fund_balance'],
        'community_allocation': prev_state['community_pool_balance'],
        'issuance_value_usd': prev_state.get('issuance_value_usd', 0),
        'retirement_value_usd': prev_state.get('retirement_value_usd', 0),
        'trade_value_usd': prev_state.get('trade_value_usd', 0),
    }

    # P4: Mint/burn
    mint_burn = p_mint_burn(params, substep, state_history, prev_state, pool_input)

    # P5: Validator compensation
    val_comp = p_validator_compensation(params, substep, state_history, prev_state, pool_input)

    # P6: Contribution rewards
    rewards = p_contribution_rewards(params, substep, state_history, prev_state, pool_input)

    result = {}
    result.update(mint_burn)
    result.update(val_comp)
    result.update(rewards)
    return result


def _composite_agent_dynamics(params, substep, state_history, prev_state):
    """PSUB-3 policy: agent population dynamics (validators, stability, price)."""
    # Read latest compensation data from state
    agent_input = {
        'validator_income_usd': prev_state.get('validator_income_usd', 0),
    }
    return p_agent_dynamics(params, substep, state_history, prev_state, agent_input)


# ---------------------------------------------------------------------------
# Partial State Update Blocks (PSUBs)
# ---------------------------------------------------------------------------

partial_state_update_blocks = [
    # PSUB 1: Market activity -> Fee collection -> Fee distribution
    #         Also records transaction volumes.
    {
        'policies': {
            'market_and_fees': _composite_market_and_fees,
        },
        'variables': {
            'total_fees_collected': s_total_fees_collected,
            'total_fees_usd': s_total_fees_usd,
            'burn_pool_balance': s_burn_pool,
            'validator_fund_balance': s_validator_fund,
            'community_pool_balance': s_community_pool,
            'agent_infra_balance': s_agent_infra,
            'cumulative_fees': s_cumulative_fees,
            'issuance_count': s_issuance_count,
            'trade_count': s_trade_count,
            'retirement_count': s_retirement_count,
            'transfer_count': s_transfer_count,
            'issuance_value_usd': s_issuance_value,
            'trade_value_usd': s_trade_value,
            'retirement_value_usd': s_retirement_value,
            'transfer_value_usd': s_transfer_value,
            'total_volume_usd': s_total_volume,
            'credit_volume_weekly_usd': s_credit_volume_weekly,
        },
    },
    # PSUB 2: Mint/burn + validator compensation + rewards
    {
        'policies': {
            'supply_and_compensation': _composite_supply_and_compensation,
        },
        'variables': {
            'S': s_supply,
            'M_t': s_minted,
            'B_t': s_burned,
            'cumulative_minted': s_cumulative_minted,
            'cumulative_burned': s_cumulative_burned,
            'r_effective': s_r_effective,
            'supply_state': s_supply_state,
            'periods_near_equilibrium': s_periods_near_equilibrium,
            'validator_income_period': s_validator_income_period,
            'validator_income_annual': s_validator_income_annual,
            'validator_income_usd': s_validator_income_usd,
            'stability_allocation': s_stability_allocation,
            'activity_pool': s_activity_pool,
            'total_activity_score': s_total_activity_score,
            'stability_utilization': s_stability_utilization,
            'reward_per_unit_activity': s_reward_per_unit_activity,
        },
    },
    # PSUB 3: Agent dynamics (validator churn, stability adoption, price)
    {
        'policies': {
            'agent_dynamics': _composite_agent_dynamics,
        },
        'variables': {
            'active_validators': s_active_validators,
            'stability_committed': s_stability_committed,
            'regen_price_usd': s_regen_price,
            'ecological_multiplier': s_ecological_multiplier,
        },
    },
]


# ---------------------------------------------------------------------------
# Configuration builder
# ---------------------------------------------------------------------------

def build_config(
    params_override: dict | None = None,
    initial_state_override: dict | None = None,
    T: int = 260,
    N: int = 1,
    M: dict | None = None,
):
    """
    Build a cadCAD configuration.

    Args:
        params_override: Dict of parameter overrides merged onto baseline.
        initial_state_override: Dict of initial state overrides.
        T: Number of timesteps (epochs). Default 260 = 5 years.
        N: Number of Monte Carlo runs. Default 1.
        M: Full parameter dict (if provided, params_override is ignored).

    Returns:
        A cadCAD Configuration list (ready for execution).
    """
    # Build parameter set
    if M is not None:
        sim_params = M
    else:
        sim_params = copy.deepcopy(baseline_params)
        if params_override:
            sim_params.update(params_override)

    # Build initial state
    sim_state = copy.deepcopy(initial_state)
    if initial_state_override:
        sim_state.update(initial_state_override)

    # cadCAD sim_config
    sim_config = config_sim({
        'T': range(T),
        'N': N,
        'M': sim_params,
    })

    exp = Experiment()
    exp.append_configs(
        initial_state=sim_state,
        partial_state_update_blocks=partial_state_update_blocks,
        sim_configs=sim_config,
    )
    return exp


def build_configs_for_sweep(
    sweep_overrides: list[dict],
    T: int = 260,
    N: int = 1,
):
    """
    Build multiple cadCAD configurations for a parameter sweep.

    Args:
        sweep_overrides: List of dicts, each containing parameter overrides
                         for one sweep point.
        T: Number of timesteps per configuration.
        N: Monte Carlo runs per configuration.

    Returns:
        A cadCAD Experiment with all configurations appended.
    """
    exp = Experiment()
    sim_state = copy.deepcopy(initial_state)

    for override in sweep_overrides:
        sim_params = copy.deepcopy(baseline_params)
        sim_params.update(override)

        sim_config = config_sim({
            'T': range(T),
            'N': N,
            'M': sim_params,
        })

        exp.append_configs(
            initial_state=sim_state,
            partial_state_update_blocks=partial_state_update_blocks,
            sim_configs=sim_config,
        )

    return exp
