#!/usr/bin/env python3
"""
Run baseline simulation for the Regen M012-M015 economic model.

Simulates 260 epochs (5 years) with baseline parameters and outputs
a summary table of key metrics against success criteria.

Usage:
    python run_baseline.py [--epochs EPOCHS] [--seed SEED] [--plot]
"""

import argparse
import sys
import os

import numpy as np
import pandas as pd

# Ensure the package is importable
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from model.config import build_config
from model.params import baseline_params


def run_simulation(T=260, seed=42, N=1):
    """Execute the baseline simulation and return results as a DataFrame."""
    np.random.seed(seed)

    exp = build_config(T=T, N=N)

    from cadCAD.engine import ExecutionMode, ExecutionContext, Executor

    exec_context = ExecutionContext(context=ExecutionMode().local_mode)
    simulation = Executor(exec_context=exec_context, configs=exp.configs)
    raw_system_events, _, _ = simulation.execute()

    df = pd.DataFrame(raw_system_events)

    # Keep only the final substep per timestep (cadCAD produces one row per PSUB)
    if 'substep' in df.columns:
        df = df.groupby(['run', 'timestep']).last().reset_index()
    elif len(df) > 0:
        # Infer substeps: multiple rows per timestep
        counts = df.groupby('timestep').size()
        if counts.max() > 1:
            df = df.groupby('timestep').last().reset_index()

    return df


def evaluate_success_criteria(df):
    """Evaluate the 6 success criteria from the simulation spec."""
    results = {}

    # 1. Validator sustainability: Annual income >= $15,000 (mean over 5yr)
    mean_validator_income = df['validator_income_usd'].mean()
    results['validator_sustainability'] = {
        'metric': 'Mean annual validator income (USD)',
        'value': f'${mean_validator_income:,.0f}',
        'threshold': '>= $15,000',
        'pass': mean_validator_income >= 15_000,
    }

    # 2. Supply stability: S within [150M, 221M] (95th percentile)
    s_5 = df['S'].quantile(0.025)
    s_95 = df['S'].quantile(0.975)
    supply_stable = s_5 >= 150_000_000 and s_95 <= 221_000_000
    results['supply_stability'] = {
        'metric': 'Supply 95% CI',
        'value': f'[{s_5/1e6:.1f}M, {s_95/1e6:.1f}M]',
        'threshold': '[150M, 221M]',
        'pass': supply_stable,
    }

    # 3. Equilibrium convergence: |M-B| < 1% of S within 5 years
    df_last_year = df[df['timestep'] >= 208]  # Last year
    if len(df_last_year) > 0:
        near_eq = (abs(df_last_year['M_t'] - df_last_year['B_t']) <
                   0.01 * df_last_year['S'])
        eq_fraction = near_eq.mean()
    else:
        eq_fraction = 0.0

    results['equilibrium_convergence'] = {
        'metric': 'Fraction of last year near equilibrium',
        'value': f'{eq_fraction:.1%}',
        'threshold': '> 50%',
        'pass': eq_fraction > 0.50,
    }

    # 4. Reward pool adequacy: activity_pool > 0 in all active periods
    active_periods = df[df['timestep'] > 0]
    zero_pool_periods = (active_periods['activity_pool'] <= 0).sum()
    total_periods = len(active_periods)
    results['reward_pool_adequacy'] = {
        'metric': 'Periods with zero activity pool',
        'value': f'{zero_pool_periods} / {total_periods}',
        'threshold': '0 zero-pool periods',
        'pass': zero_pool_periods == 0,
    }

    # 5. Stability tier solvency: obligations met in >= 95% of periods
    df_with_stability = df[df['stability_committed'] > 0]
    if len(df_with_stability) > 0:
        solvent = (df_with_stability['stability_utilization'] >= 0.95).mean()
    else:
        solvent = 1.0  # No commitments = trivially solvent

    results['stability_solvency'] = {
        'metric': 'Stability obligation coverage rate',
        'value': f'{solvent:.1%}',
        'threshold': '>= 95%',
        'pass': solvent >= 0.95,
    }

    # 6. Attack resistance: Placeholder (evaluated in stress tests)
    results['attack_resistance'] = {
        'metric': 'Stress test pass rate',
        'value': 'See run_stress_tests.py',
        'threshold': 'All 8 scenarios pass',
        'pass': None,
    }

    return results


def print_summary(df, results):
    """Print a formatted summary of the simulation results."""
    print("=" * 78)
    print("REGEN ECONOMIC SIMULATION — BASELINE RESULTS")
    print("=" * 78)

    print(f"\nSimulation: {df['timestep'].max()} epochs "
          f"({df['timestep'].max() / 52:.1f} years)")
    print(f"Parameters: Baseline (r_base={baseline_params['base_regrowth_rate']}, "
          f"burn_share={baseline_params['burn_share']}, "
          f"vol=${baseline_params['initial_weekly_volume_usd']:,.0f}/wk)")

    # Key metrics over time
    print("\n--- Key Metrics (Final Epoch) ---")
    final = df.iloc[-1]
    print(f"  Supply:                {final['S']/1e6:.2f}M REGEN "
          f"(cap: {baseline_params['hard_cap']/1e6:.0f}M)")
    print(f"  Supply State:          {final['supply_state']}")
    print(f"  Minted (last period):  {final['M_t']:,.0f} REGEN")
    print(f"  Burned (last period):  {final['B_t']:,.0f} REGEN")
    print(f"  Cumulative Minted:     {final['cumulative_minted']/1e6:.2f}M REGEN")
    print(f"  Cumulative Burned:     {final['cumulative_burned']/1e6:.2f}M REGEN")
    print(f"  Effective r:           {final['r_effective']:.4f}")
    print(f"  REGEN Price:           ${final['regen_price_usd']:.4f}")
    print(f"  Active Validators:     {int(final['active_validators'])}")
    print(f"  Validator Income/yr:   ${final['validator_income_usd']:,.0f} USD")
    print(f"  Stability Committed:   {final['stability_committed']/1e6:.2f}M REGEN")
    print(f"  Weekly Fees:           {final['total_fees_collected']:,.0f} REGEN")
    print(f"  Activity Pool:         {final['activity_pool']:,.0f} REGEN")

    # Success criteria
    print("\n--- Success Criteria ---")
    print(f"{'Criterion':<30} {'Value':<25} {'Threshold':<20} {'Result':<8}")
    print("-" * 83)
    for name, r in results.items():
        status = "PASS" if r['pass'] else ("FAIL" if r['pass'] is not None else "N/A")
        print(f"  {name:<28} {r['value']:<25} {r['threshold']:<20} {status:<8}")

    # Summary statistics
    print("\n--- Summary Statistics (All Periods) ---")
    metrics = ['S', 'M_t', 'B_t', 'total_fees_collected', 'validator_income_usd',
               'activity_pool', 'stability_committed', 'regen_price_usd']
    print(f"{'Metric':<25} {'Mean':>15} {'Std':>15} {'Min':>15} {'Max':>15}")
    print("-" * 85)
    for m in metrics:
        if m in df.columns:
            print(f"  {m:<23} {df[m].mean():>15,.2f} {df[m].std():>15,.2f} "
                  f"{df[m].min():>15,.2f} {df[m].max():>15,.2f}")

    print("\n" + "=" * 78)


def plot_results(df, save_path=None):
    """Generate time-series plots of key metrics."""
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print("matplotlib not available; skipping plots.")
        return

    fig, axes = plt.subplots(3, 2, figsize=(14, 12))
    fig.suptitle('Regen Economic Simulation — Baseline (5 Year)', fontsize=14)

    epochs = df['timestep']

    # Supply
    ax = axes[0, 0]
    ax.plot(epochs, df['S'] / 1e6, label='Supply', color='steelblue')
    ax.axhline(y=221, color='red', linestyle='--', alpha=0.7, label='Hard Cap (221M)')
    ax.set_ylabel('Supply (M REGEN)')
    ax.set_title('M012: Circulating Supply')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Mint vs Burn
    ax = axes[0, 1]
    ax.plot(epochs, df['M_t'], label='Minted', color='green', alpha=0.7)
    ax.plot(epochs, df['B_t'], label='Burned', color='red', alpha=0.7)
    ax.set_ylabel('REGEN / period')
    ax.set_title('M012: Minting vs Burning')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Fee Revenue
    ax = axes[1, 0]
    ax.plot(epochs, df['total_fees_collected'], color='orange', alpha=0.7)
    ax.set_ylabel('Fees (REGEN)')
    ax.set_title('M013: Period Fee Revenue')
    ax.grid(True, alpha=0.3)

    # Validator Income
    ax = axes[1, 1]
    ax.plot(epochs, df['validator_income_usd'], color='purple', alpha=0.7)
    ax.axhline(y=15_000, color='red', linestyle='--', alpha=0.7, label='Min Viable ($15K)')
    ax.set_ylabel('USD / year')
    ax.set_title('M014: Per-Validator Annual Income')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Stability & Activity Pool
    ax = axes[2, 0]
    ax.plot(epochs, df['stability_committed'] / 1e6, label='Stability Committed',
            color='teal', alpha=0.7)
    ax.set_ylabel('M REGEN')
    ax.set_title('M015: Stability Tier Commitments')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # REGEN Price
    ax = axes[2, 1]
    ax.plot(epochs, df['regen_price_usd'], color='gold', alpha=0.7)
    ax.set_ylabel('USD')
    ax.set_title('REGEN Price (GBM + Mean Reversion)')
    ax.grid(True, alpha=0.3)

    for ax_row in axes:
        for ax in ax_row:
            ax.set_xlabel('Epoch (weeks)')

    plt.tight_layout()

    if save_path:
        plt.savefig(save_path, dpi=150, bbox_inches='tight')
        print(f"Plot saved to {save_path}")
    else:
        plt.show()


def main():
    parser = argparse.ArgumentParser(description='Run baseline Regen economic simulation')
    parser.add_argument('--epochs', type=int, default=260, help='Number of epochs (default: 260)')
    parser.add_argument('--seed', type=int, default=42, help='Random seed (default: 42)')
    parser.add_argument('--plot', action='store_true', help='Generate plots')
    parser.add_argument('--save-plot', type=str, default=None, help='Save plot to file')
    parser.add_argument('--csv', type=str, default=None, help='Export results to CSV')
    args = parser.parse_args()

    print(f"Running baseline simulation: {args.epochs} epochs, seed={args.seed}")
    df = run_simulation(T=args.epochs, seed=args.seed)

    results = evaluate_success_criteria(df)
    print_summary(df, results)

    if args.csv:
        df.to_csv(args.csv, index=False)
        print(f"\nResults exported to {args.csv}")

    if args.plot or args.save_plot:
        plot_results(df, save_path=args.save_plot)


if __name__ == '__main__':
    main()
