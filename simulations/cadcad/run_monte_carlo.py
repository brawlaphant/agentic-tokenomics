#!/usr/bin/env python3
"""
Run Monte Carlo simulations for the Regen M012-M015 economic model.

Executes N independent runs with stochastic variation and computes
confidence intervals for key metrics.

Usage:
    python run_monte_carlo.py [--runs N] [--epochs EPOCHS] [--seed SEED]
"""

import argparse
import sys
import os

import numpy as np
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from model.config import build_config
from model.params import baseline_params


def run_monte_carlo(N=1000, T=260, seed=42):
    """
    Execute N Monte Carlo runs of the baseline simulation.

    Each run uses a different random seed derived from the base seed.
    Returns a DataFrame with all runs concatenated, distinguished by 'run' column.
    """
    from cadCAD.engine import ExecutionMode, ExecutionContext, Executor

    print(f"Running {N} Monte Carlo simulations ({T} epochs each)...")

    # Build config with N runs
    np.random.seed(seed)
    exp = build_config(T=T, N=N)

    exec_context = ExecutionContext(context=ExecutionMode().local_mode)
    simulation = Executor(exec_context=exec_context, configs=exp.configs)
    raw_system_events, _, _ = simulation.execute()

    df = pd.DataFrame(raw_system_events)

    # Keep only the final substep per timestep per run
    if 'substep' in df.columns:
        df = df.groupby(['run', 'timestep']).last().reset_index()
    elif len(df) > 0:
        group_cols = ['run', 'timestep'] if 'run' in df.columns else ['timestep']
        counts = df.groupby(group_cols).size()
        if counts.max() > 1:
            df = df.groupby(group_cols).last().reset_index()

    print(f"  Completed. Total records: {len(df)}")
    return df


def compute_confidence_intervals(df, confidence=0.95):
    """
    Compute confidence intervals for key metrics across Monte Carlo runs.

    Groups by timestep and computes percentile-based CIs.
    """
    alpha = 1 - confidence
    lo_pct = alpha / 2 * 100
    hi_pct = (1 - alpha / 2) * 100

    metrics = ['S', 'M_t', 'B_t', 'total_fees_collected', 'validator_income_usd',
               'activity_pool', 'stability_committed', 'regen_price_usd',
               'active_validators', 'stability_utilization']

    available_metrics = [m for m in metrics if m in df.columns]

    grouped = df.groupby('timestep')[available_metrics]

    ci_results = {}
    for metric in available_metrics:
        ci_results[metric] = {
            'mean': grouped[metric].mean(),
            'median': grouped[metric].median(),
            'std': grouped[metric].std(),
            'ci_lo': grouped[metric].quantile(lo_pct / 100),
            'ci_hi': grouped[metric].quantile(hi_pct / 100),
            'p5': grouped[metric].quantile(0.05),
            'p95': grouped[metric].quantile(0.95),
        }

    return ci_results


def compute_terminal_distributions(df):
    """Compute distributions of key metrics at the final timestep."""
    final_epoch = df['timestep'].max()
    terminal = df[df['timestep'] == final_epoch]

    metrics = {
        'supply_M': terminal['S'] / 1e6,
        'validator_income_usd': terminal['validator_income_usd'],
        'cumulative_burned_M': terminal['cumulative_burned'] / 1e6,
        'cumulative_minted_M': terminal['cumulative_minted'] / 1e6,
        'regen_price': terminal['regen_price_usd'],
        'active_validators': terminal['active_validators'],
        'stability_committed_M': terminal['stability_committed'] / 1e6,
    }

    results = {}
    for name, series in metrics.items():
        results[name] = {
            'mean': series.mean(),
            'median': series.median(),
            'std': series.std(),
            'p5': series.quantile(0.05),
            'p25': series.quantile(0.25),
            'p75': series.quantile(0.75),
            'p95': series.quantile(0.95),
            'min': series.min(),
            'max': series.max(),
        }

    return results


def evaluate_mc_success_criteria(df):
    """Evaluate success criteria across all Monte Carlo runs."""
    final_epoch = df['timestep'].max()
    terminal = df[df['timestep'] == final_epoch]
    n_runs = terminal['run'].nunique() if 'run' in terminal.columns else len(terminal)

    results = {}

    # 1. Validator sustainability (fraction of runs meeting threshold)
    runs_meeting_val = 0
    for run_id in terminal['run'].unique() if 'run' in terminal.columns else [0]:
        run_df = df[df['run'] == run_id] if 'run' in df.columns else df
        mean_income = run_df['validator_income_usd'].mean()
        if mean_income >= 15_000:
            runs_meeting_val += 1
    results['validator_sustainability'] = runs_meeting_val / max(n_runs, 1)

    # 2. Supply stability (fraction within [150M, 221M] at 95th percentile)
    supply_min = terminal['S'].quantile(0.025) / 1e6
    supply_max = terminal['S'].quantile(0.975) / 1e6
    results['supply_in_bounds'] = supply_min >= 150 and supply_max <= 221
    results['supply_ci'] = (supply_min, supply_max)

    # 3. Reward pool adequacy
    zero_pool_runs = 0
    for run_id in terminal['run'].unique() if 'run' in terminal.columns else [0]:
        run_df = df[df['run'] == run_id] if 'run' in df.columns else df
        if (run_df['activity_pool'] <= 0).any():
            zero_pool_runs += 1
    results['reward_pool_always_positive'] = 1.0 - zero_pool_runs / max(n_runs, 1)

    # 4. Stability tier solvency
    solvency_failures = 0
    for run_id in terminal['run'].unique() if 'run' in terminal.columns else [0]:
        run_df = df[df['run'] == run_id] if 'run' in df.columns else df
        committed = run_df[run_df['stability_committed'] > 0]
        if len(committed) > 0:
            solvent_frac = (committed['stability_utilization'] >= 0.95).mean()
            if solvent_frac < 0.95:
                solvency_failures += 1
    results['stability_solvency_rate'] = 1.0 - solvency_failures / max(n_runs, 1)

    return results


def print_mc_summary(df, ci_results, terminal_dist, criteria, N):
    """Print Monte Carlo simulation summary."""
    print("=" * 80)
    print(f"REGEN ECONOMIC SIMULATION — MONTE CARLO RESULTS ({N} runs)")
    print("=" * 80)

    # Terminal distributions
    print("\n--- Terminal Distributions (Year 5) ---")
    print(f"{'Metric':<25} {'Mean':>12} {'Median':>12} {'P5':>12} "
          f"{'P95':>12} {'Std':>12}")
    print("-" * 85)
    for name, stats in terminal_dist.items():
        print(f"  {name:<23} {stats['mean']:>12,.2f} {stats['median']:>12,.2f} "
              f"{stats['p5']:>12,.2f} {stats['p95']:>12,.2f} {stats['std']:>12,.2f}")

    # Success criteria
    print("\n--- Success Criteria (Monte Carlo) ---")
    print(f"  Validator sustainability (runs >= $15K):  "
          f"{criteria['validator_sustainability']:.1%}")
    print(f"  Supply in [150M, 221M] (95% CI):         "
          f"{'PASS' if criteria['supply_in_bounds'] else 'FAIL'} "
          f"[{criteria['supply_ci'][0]:.1f}M, {criteria['supply_ci'][1]:.1f}M]")
    print(f"  Reward pool always positive:              "
          f"{criteria['reward_pool_always_positive']:.1%}")
    print(f"  Stability tier solvency rate:             "
          f"{criteria['stability_solvency_rate']:.1%}")

    print("\n" + "=" * 80)


def plot_mc_results(ci_results, N, save_path=None):
    """Plot Monte Carlo confidence intervals."""
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print("matplotlib not available; skipping plots.")
        return

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    fig.suptitle(f'Monte Carlo Simulation ({N} runs) — 95% Confidence Intervals',
                 fontsize=14)

    plots = [
        ('S', 'Supply (REGEN)', 1e6, 'M REGEN'),
        ('validator_income_usd', 'Validator Annual Income', 1, 'USD'),
        ('total_fees_collected', 'Period Fee Revenue', 1, 'REGEN'),
        ('regen_price_usd', 'REGEN Price', 1, 'USD'),
    ]

    for idx, (metric, title, scale, unit) in enumerate(plots):
        ax = axes[idx // 2][idx % 2]
        if metric not in ci_results:
            continue

        data = ci_results[metric]
        epochs = data['mean'].index

        ax.plot(epochs, data['mean'] / scale, color='steelblue', label='Mean')
        ax.fill_between(epochs, data['ci_lo'] / scale, data['ci_hi'] / scale,
                        alpha=0.2, color='steelblue', label='95% CI')
        ax.fill_between(epochs, data['p5'] / scale, data['p95'] / scale,
                        alpha=0.1, color='orange', label='5th-95th pct')
        ax.set_title(title)
        ax.set_ylabel(unit)
        ax.set_xlabel('Epoch')
        ax.legend(fontsize=8)
        ax.grid(True, alpha=0.3)

    plt.tight_layout()

    if save_path:
        plt.savefig(save_path, dpi=150, bbox_inches='tight')
        print(f"Plot saved to {save_path}")
    else:
        plt.show()


def main():
    parser = argparse.ArgumentParser(description='Run Monte Carlo simulations')
    parser.add_argument('--runs', type=int, default=1000,
                        help='Number of Monte Carlo runs (default: 1000)')
    parser.add_argument('--epochs', type=int, default=260,
                        help='Epochs per run (default: 260)')
    parser.add_argument('--seed', type=int, default=42, help='Base random seed')
    parser.add_argument('--plot', action='store_true', help='Generate plots')
    parser.add_argument('--save-plot', type=str, default=None, help='Save plot to file')
    parser.add_argument('--csv', type=str, default=None, help='Export raw results to CSV')
    args = parser.parse_args()

    df = run_monte_carlo(N=args.runs, T=args.epochs, seed=args.seed)

    ci_results = compute_confidence_intervals(df)
    terminal_dist = compute_terminal_distributions(df)
    criteria = evaluate_mc_success_criteria(df)

    print_mc_summary(df, ci_results, terminal_dist, criteria, args.runs)

    if args.csv:
        df.to_csv(args.csv, index=False)
        print(f"\nRaw results exported to {args.csv}")

    if args.plot or args.save_plot:
        plot_mc_results(ci_results, args.runs, save_path=args.save_plot)


if __name__ == '__main__':
    main()
