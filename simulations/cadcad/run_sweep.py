#!/usr/bin/env python3
"""
Run parameter sweeps for the Regen M012-M015 economic model.

Sweeps across r_base, burn_share, fee rates, stability rate, and weekly volume.

Usage:
    python run_sweep.py [--sweep SWEEP_NAME] [--epochs EPOCHS] [--seed SEED]
    python run_sweep.py --all
"""

import argparse
import sys
import os
import copy

import numpy as np
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from model.config import build_config
from model.params import baseline_params, sweep_params, get_sweep_param_set
from model.state_variables import initial_state


AVAILABLE_SWEEPS = [
    'r_base_sweep',
    'burn_share_sweep',
    'fee_rate_sweep',
    'stability_rate_sweep',
    'volume_sweep',
]


def run_single_config(params_override, T=260, seed=42):
    """Run a single simulation configuration and return the DataFrame."""
    np.random.seed(seed)
    exp = build_config(params_override=params_override, T=T, N=1)

    from cadCAD.engine import ExecutionMode, ExecutionContext, Executor

    exec_context = ExecutionContext(context=ExecutionMode().local_mode)
    simulation = Executor(exec_context=exec_context, configs=exp.configs)
    raw_system_events, _, _ = simulation.execute()

    df = pd.DataFrame(raw_system_events)

    # Keep only the final substep per timestep
    if 'substep' in df.columns:
        df = df.groupby(['run', 'timestep']).last().reset_index()
    elif len(df) > 0:
        counts = df.groupby('timestep').size()
        if counts.max() > 1:
            df = df.groupby('timestep').last().reset_index()

    return df


def extract_summary(df, label):
    """Extract key summary metrics from a simulation run."""
    final = df.iloc[-1]
    last_year = df[df['timestep'] >= (df['timestep'].max() - 52)]

    return {
        'label': label,
        'supply_final_M': final['S'] / 1e6,
        'mean_validator_income_usd': df['validator_income_usd'].mean(),
        'final_validator_income_usd': final['validator_income_usd'],
        'cumulative_burned_M': final['cumulative_burned'] / 1e6,
        'cumulative_minted_M': final['cumulative_minted'] / 1e6,
        'mean_activity_pool': df['activity_pool'].mean(),
        'final_supply_state': final['supply_state'],
        'eq_fraction_last_year': (
            (abs(last_year['M_t'] - last_year['B_t']) < 0.01 * last_year['S']).mean()
            if len(last_year) > 0 else 0.0
        ),
        'mean_stability_util': df['stability_utilization'].mean(),
        'zero_pool_periods': (df['activity_pool'] <= 0).sum(),
    }


def run_sweep(sweep_name, T=260, seed=42):
    """Run a parameter sweep and return summary results."""
    configs = get_sweep_param_set(sweep_name)

    results = []
    for i, override in enumerate(configs):
        # Create a label from the override values
        label_parts = [f"{k}={v}" for k, v in override.items()
                       if not isinstance(v, dict)]
        label = ", ".join(label_parts)
        print(f"  [{i+1}/{len(configs)}] {label}")

        df = run_single_config(override, T=T, seed=seed)
        summary = extract_summary(df, label)
        summary.update(override)
        results.append(summary)

    return pd.DataFrame(results)


def print_sweep_results(sweep_name, results_df):
    """Print formatted sweep results."""
    print(f"\n{'=' * 90}")
    print(f"PARAMETER SWEEP: {sweep_name}")
    print(f"{'=' * 90}")

    # Select columns to display based on sweep
    display_cols = ['label', 'supply_final_M', 'mean_validator_income_usd',
                    'cumulative_burned_M', 'mean_activity_pool',
                    'eq_fraction_last_year', 'zero_pool_periods']

    # Clean column names for display
    col_headers = {
        'label': 'Configuration',
        'supply_final_M': 'Supply (M)',
        'mean_validator_income_usd': 'Avg Val Inc ($)',
        'cumulative_burned_M': 'Cum Burn (M)',
        'mean_activity_pool': 'Avg Act Pool',
        'eq_fraction_last_year': 'Eq Frac',
        'zero_pool_periods': 'Zero Pools',
    }

    available_cols = [c for c in display_cols if c in results_df.columns]
    display_df = results_df[available_cols].copy()
    display_df.columns = [col_headers.get(c, c) for c in available_cols]

    try:
        from tabulate import tabulate
        print(tabulate(display_df, headers='keys', tablefmt='grid',
                       floatfmt=('.0f', '.1f', ',.0f', '.2f', ',.0f', '.2f', '.0f'),
                       showindex=False))
    except ImportError:
        print(display_df.to_string(index=False))

    # Key findings
    print("\n--- Key Findings ---")

    if 'mean_validator_income_usd' in results_df.columns:
        viable = results_df[results_df['mean_validator_income_usd'] >= 15_000]
        if len(viable) > 0:
            print(f"  Validator-sustainable configs: {len(viable)}/{len(results_df)}")
        else:
            print("  WARNING: No configuration meets validator sustainability threshold")

    if 'zero_pool_periods' in results_df.columns:
        all_positive = results_df[results_df['zero_pool_periods'] == 0]
        print(f"  Configs with always-positive activity pool: "
              f"{len(all_positive)}/{len(results_df)}")

    print()


def main():
    parser = argparse.ArgumentParser(description='Run parameter sweeps')
    parser.add_argument('--sweep', type=str, choices=AVAILABLE_SWEEPS,
                        help='Specific sweep to run')
    parser.add_argument('--all', action='store_true', help='Run all sweeps')
    parser.add_argument('--epochs', type=int, default=260, help='Epochs per run')
    parser.add_argument('--seed', type=int, default=42, help='Random seed')
    parser.add_argument('--csv', type=str, default=None,
                        help='Export results to CSV (prefix)')
    args = parser.parse_args()

    sweeps_to_run = AVAILABLE_SWEEPS if args.all else (
        [args.sweep] if args.sweep else AVAILABLE_SWEEPS
    )

    for sweep_name in sweeps_to_run:
        print(f"\nRunning sweep: {sweep_name}")
        results_df = run_sweep(sweep_name, T=args.epochs, seed=args.seed)
        print_sweep_results(sweep_name, results_df)

        if args.csv:
            csv_path = f"{args.csv}_{sweep_name}.csv"
            results_df.to_csv(csv_path, index=False)
            print(f"  Results saved to {csv_path}")


if __name__ == '__main__':
    main()
