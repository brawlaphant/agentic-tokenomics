#!/usr/bin/env python3
"""
Analyze simulation results and produce the equilibrium findings summary.

This module provides analysis functions used by the runner scripts and can
also be invoked standalone to analyze pre-computed CSV results.

Usage:
    python analysis.py [--baseline results_baseline.csv]
    python analysis.py --from-run  (run baseline then analyze)
"""

import argparse
import sys
import os

import numpy as np
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from model.params import baseline_params


# ---------------------------------------------------------------------------
# Equilibrium calculations (closed-form)
# ---------------------------------------------------------------------------

def compute_equilibrium_supply(
    C=221_000_000,
    burn_share=0.30,
    weekly_volume_usd=500_000,
    w_avg_fee_rate=0.01,
    r_effective=0.026,
    regen_price=0.05,
):
    """
    Compute closed-form equilibrium supply S*.

    At equilibrium: M[t] = B[t]
      r * (C - S*) = burn_share * V * w_avg / P

    S* = C - (burn_share * V * w_avg) / (r * P)
    """
    burn_flow = burn_share * weekly_volume_usd * w_avg_fee_rate
    regrowth_coeff = r_effective * regen_price

    if regrowth_coeff <= 0:
        return C  # No regrowth -> supply stays at cap

    S_star = C - burn_flow / regrowth_coeff
    return max(0, min(S_star, C))


def compute_convergence_time(
    S_0=224_000_000,
    S_star=219_846_154,
    r=0.026,
    epsilon_fraction=0.01,
):
    """
    Compute time to convergence (within epsilon of S*).

    S[t] - S* = (S_0 - S*) * (1 - r)^t
    t = log(epsilon / |S_0 - S*|) / log(1 - r)
    """
    gap = abs(S_0 - S_star)
    epsilon = epsilon_fraction * S_star

    if gap <= epsilon:
        return 0

    if r >= 1.0 or r <= 0:
        return float('inf')

    t = np.log(epsilon / gap) / np.log(1 - r)
    return max(0, t)


def compute_min_viable_volume(
    min_income_usd=15_000,
    n_validators=18,
    w_avg_fee_rate=0.01,
    validator_share=0.40,
    periods_per_year=52,
):
    """
    Compute minimum weekly volume for validator sustainability.

    V_weekly >= (I_min * N_val) / (52 * w_avg * val_share)
    """
    return (min_income_usd * n_validators) / (periods_per_year * w_avg_fee_rate * validator_share)


def compute_max_stability_commitments(
    weekly_volume_usd=500_000,
    w_avg_fee_rate=0.01,
    community_share=0.25,
    max_stability_share=0.30,
    stability_rate=0.06,
    periods_per_year=52,
    regen_price=0.05,
):
    """
    Compute maximum supportable stability tier commitments (in REGEN).

    commitments <= V_weekly * w_avg * comm_share * max_stab * periods / rate / P
    """
    annual_community_usd = weekly_volume_usd * w_avg_fee_rate * community_share * periods_per_year
    max_usd = annual_community_usd * max_stability_share / stability_rate
    return max_usd / regen_price


def compute_wash_trade_breakeven(
    fee_rate_cycle=0.021,  # buy 1% + transfer 0.1% + sell 1%
    purchase_weight=0.30,
):
    """
    Compute the reward-rate at which wash trading becomes profitable.

    Break-even: purchase_weight * activity_pool / total_score = fee_rate_cycle
    reward_rate_breakeven = fee_rate_cycle / purchase_weight
    """
    return fee_rate_cycle / purchase_weight


def compute_weighted_avg_fee_rate(params=None):
    """
    Compute weighted average fee rate across transaction types.

    Weights based on approximate volume share by type.
    """
    if params is None:
        params = baseline_params

    # Approximate volume weights (from spec agent population)
    weights = {
        'issuance': 0.35,
        'trade': 0.40,
        'retirement': 0.20,
        'transfer': 0.05,
    }

    w_avg = (
        weights['issuance'] * params['fee_rate_issuance_bps'] / 10_000 +
        weights['trade'] * params['fee_rate_trade_bps'] / 10_000 +
        weights['retirement'] * params['fee_rate_retirement_bps'] / 10_000 +
        weights['transfer'] * params['fee_rate_transfer_bps'] / 10_000
    )
    return w_avg


# ---------------------------------------------------------------------------
# Simulation result analysis
# ---------------------------------------------------------------------------

def analyze_simulation(df, params=None):
    """
    Comprehensive analysis of a simulation result DataFrame.

    Returns a dict of analysis results.
    """
    if params is None:
        params = baseline_params

    results = {}

    # Basic statistics
    results['total_epochs'] = df['timestep'].max()
    results['years'] = results['total_epochs'] / 52

    # Supply analysis
    final = df.iloc[-1]
    results['initial_supply'] = df.iloc[0]['S']
    results['final_supply'] = final['S']
    results['supply_change'] = final['S'] - df.iloc[0]['S']
    results['supply_change_pct'] = results['supply_change'] / results['initial_supply'] * 100
    results['cumulative_minted'] = final['cumulative_minted']
    results['cumulative_burned'] = final['cumulative_burned']
    results['net_supply_change'] = final['cumulative_minted'] - final['cumulative_burned']

    # Equilibrium analysis
    last_52 = df[df['timestep'] >= (results['total_epochs'] - 52)]
    if len(last_52) > 0:
        results['last_year_avg_mint'] = last_52['M_t'].mean()
        results['last_year_avg_burn'] = last_52['B_t'].mean()
        results['last_year_mint_burn_ratio'] = (
            results['last_year_avg_mint'] / max(results['last_year_avg_burn'], 1e-9)
        )
        results['near_equilibrium_frac'] = (
            abs(last_52['M_t'] - last_52['B_t']) < 0.01 * last_52['S']
        ).mean()
    else:
        results['near_equilibrium_frac'] = 0.0

    # Fee analysis
    results['total_fees_regen'] = final['cumulative_fees']
    results['avg_weekly_fees_regen'] = df['total_fees_collected'].mean()
    results['avg_weekly_fees_usd'] = df['total_fees_usd'].mean()

    # Validator analysis
    results['avg_validator_income_usd'] = df['validator_income_usd'].mean()
    results['min_validator_income_usd'] = df['validator_income_usd'].min()
    results['max_validator_income_usd'] = df['validator_income_usd'].max()
    results['avg_validators'] = df['active_validators'].mean()
    results['min_validators'] = df['active_validators'].min()

    # Reward analysis
    results['avg_activity_pool'] = df['activity_pool'].mean()
    results['zero_pool_periods'] = (df[df['timestep'] > 0]['activity_pool'] <= 0).sum()
    results['avg_stability_util'] = df['stability_utilization'].mean()
    results['final_stability_committed'] = final['stability_committed']

    # Closed-form equilibrium
    w_avg = compute_weighted_avg_fee_rate(params)
    r_eff = df['r_effective'].iloc[-10:].mean() if len(df) > 10 else params['base_regrowth_rate']

    results['w_avg_fee_rate'] = w_avg
    results['r_effective_avg'] = r_eff
    results['S_star_theoretical'] = compute_equilibrium_supply(
        C=params['hard_cap'],
        burn_share=params['burn_share'],
        weekly_volume_usd=params['initial_weekly_volume_usd'],
        w_avg_fee_rate=w_avg,
        r_effective=r_eff,
        regen_price=df['regen_price_usd'].mean(),
    )
    results['min_viable_volume'] = compute_min_viable_volume(
        min_income_usd=params['min_viable_validator_income_usd'],
        n_validators=int(results['avg_validators']),
        w_avg_fee_rate=w_avg,
        validator_share=params['validator_share'],
    )
    results['max_stability_regen'] = compute_max_stability_commitments(
        weekly_volume_usd=params['initial_weekly_volume_usd'],
        w_avg_fee_rate=w_avg,
        community_share=params['community_share'],
        max_stability_share=params['max_stability_share'],
        stability_rate=params['stability_annual_rate'],
        regen_price=df['regen_price_usd'].mean(),
    )

    return results


def print_equilibrium_summary(results):
    """Print the equilibrium findings summary table."""
    print("\n" + "=" * 78)
    print("EQUILIBRIUM ANALYSIS SUMMARY")
    print("=" * 78)

    print("\n--- Supply Equilibrium ---")
    print(f"  Theoretical S*:        {results['S_star_theoretical']/1e6:.2f}M REGEN")
    print(f"  Simulated final S:     {results['final_supply']/1e6:.2f}M REGEN")
    print(f"  Cumulative minted:     {results['cumulative_minted']/1e6:.2f}M REGEN")
    print(f"  Cumulative burned:     {results['cumulative_burned']/1e6:.2f}M REGEN")
    print(f"  Net supply change:     {results['net_supply_change']/1e6:+.2f}M REGEN")
    print(f"  Near-equilibrium (last yr): {results['near_equilibrium_frac']:.1%}")

    print("\n--- Fee Revenue ---")
    print(f"  Weighted avg fee rate: {results['w_avg_fee_rate']*100:.2f}%")
    print(f"  Avg weekly fees:       {results['avg_weekly_fees_regen']:,.0f} REGEN "
          f"(${results['avg_weekly_fees_usd']:,.0f})")
    print(f"  Total fees (lifetime): {results['total_fees_regen']:,.0f} REGEN")

    print("\n--- Validator Sustainability ---")
    print(f"  Avg validator income:  ${results['avg_validator_income_usd']:,.0f}/yr")
    print(f"  Min validator income:  ${results['min_validator_income_usd']:,.0f}/yr")
    print(f"  Min viable volume:     ${results['min_viable_volume']:,.0f}/week")
    print(f"  Avg validator count:   {results['avg_validators']:.1f}")
    print(f"  Min validator count:   {int(results['min_validators'])}")

    print("\n--- Stability Tier ---")
    print(f"  Avg utilization:       {results['avg_stability_util']:.1%}")
    print(f"  Final committed:       {results['final_stability_committed']/1e6:.2f}M REGEN")
    print(f"  Max supportable:       {results['max_stability_regen']/1e6:.2f}M REGEN")

    print("\n--- Activity Rewards ---")
    print(f"  Avg activity pool:     {results['avg_activity_pool']:,.0f} REGEN/period")
    print(f"  Zero-pool periods:     {results['zero_pool_periods']}")

    # Key findings
    print("\n--- KEY FINDINGS ---")
    findings = []
    if results['avg_validator_income_usd'] < 15_000:
        findings.append(
            f"WARNING: Avg validator income (${results['avg_validator_income_usd']:,.0f}) "
            f"is below $15,000 minimum. Min viable volume: "
            f"${results['min_viable_volume']:,.0f}/week."
        )
    else:
        findings.append(
            f"Validator sustainability: PASS (${results['avg_validator_income_usd']:,.0f}/yr)"
        )

    if results['near_equilibrium_frac'] > 0.5:
        findings.append(
            f"Supply converges to equilibrium (~{results['final_supply']/1e6:.1f}M)"
        )
    else:
        findings.append(
            f"Supply has not yet reached equilibrium "
            f"(near-eq fraction: {results['near_equilibrium_frac']:.0%})"
        )

    if results['zero_pool_periods'] == 0:
        findings.append("Activity reward pool always positive: PASS")
    else:
        findings.append(
            f"WARNING: {results['zero_pool_periods']} periods with zero activity pool"
        )

    wash_breakeven = compute_wash_trade_breakeven()
    findings.append(
        f"Wash trading break-even reward rate: {wash_breakeven:.1%} "
        f"(baseline is far below this)"
    )

    for f in findings:
        print(f"  - {f}")

    print("\n" + "=" * 78)


def main():
    parser = argparse.ArgumentParser(description='Analyze simulation results')
    parser.add_argument('--baseline', type=str, default=None,
                        help='Path to baseline results CSV')
    parser.add_argument('--from-run', action='store_true',
                        help='Run baseline simulation then analyze')
    parser.add_argument('--epochs', type=int, default=260, help='Epochs for --from-run')
    parser.add_argument('--seed', type=int, default=42, help='Seed for --from-run')
    args = parser.parse_args()

    if args.from_run:
        from run_baseline import run_simulation
        print("Running baseline simulation for analysis...")
        df = run_simulation(T=args.epochs, seed=args.seed)
    elif args.baseline:
        df = pd.read_csv(args.baseline)
    else:
        print("Please specify --baseline <csv> or --from-run")
        sys.exit(1)

    results = analyze_simulation(df)
    print_equilibrium_summary(results)


if __name__ == '__main__':
    main()
