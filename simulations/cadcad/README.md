# cadCAD Economic Simulations for Regen M012-M015

Agent-based simulation model for validating the Regen Economic Reboot parameters before mainnet deployment. Implements the specification from [`docs/economics/economic-simulation-spec.md`](../../docs/economics/economic-simulation-spec.md).

## Mechanisms Modeled

| Mechanism | Description | Key Parameters |
|-----------|-------------|----------------|
| M012 | Fixed Cap Dynamic Supply | r_base=0.02, C=221M, S_0=224M |
| M013 | Value-Based Fee Routing | Issuance 2%, Trade 1%, Retirement 0.5%, Transfer 0.1% |
| M014 | Authority Validator Governance | 15-21 validators, base + 10% bonus |
| M015 | Contribution-Weighted Rewards | 6% stability tier, 30% cap, activity weights |

## Quick Start

```bash
cd simulations/cadcad
pip install -r requirements.txt
python run_baseline.py
```

## Available Scripts

### `run_baseline.py` — Baseline Simulation

Runs 260 epochs (5 years) with baseline parameters and evaluates 6 success criteria.

```bash
python run_baseline.py                    # Default: 260 epochs
python run_baseline.py --epochs 520       # 10-year simulation
python run_baseline.py --plot             # Show matplotlib plots
python run_baseline.py --save-plot out.png
python run_baseline.py --csv results.csv  # Export raw data
```

### `run_sweep.py` — Parameter Sweeps

Sweeps across individual parameters to test sensitivity.

```bash
python run_sweep.py --all                      # All 5 sweeps
python run_sweep.py --sweep r_base_sweep       # Single sweep
python run_sweep.py --sweep burn_share_sweep
python run_sweep.py --sweep fee_rate_sweep
python run_sweep.py --sweep stability_rate_sweep
python run_sweep.py --sweep volume_sweep
python run_sweep.py --all --csv sweep_results  # Export to CSV
```

Available sweeps:
- **r_base_sweep**: Base regrowth rate [0.005, 0.10]
- **burn_share_sweep**: Burn share [0.00, 0.35]
- **fee_rate_sweep**: Issuance and trade fee rate combinations
- **stability_rate_sweep**: Stability tier annual return [0.02, 0.12]
- **volume_sweep**: Weekly transaction volume [$50K, $10M]

### `run_monte_carlo.py` — Monte Carlo Simulation

Runs N independent stochastic simulations to compute confidence intervals.

```bash
python run_monte_carlo.py                 # Default: 1000 runs
python run_monte_carlo.py --runs 100      # Quick test
python run_monte_carlo.py --runs 10000    # Publication quality
python run_monte_carlo.py --plot
python run_monte_carlo.py --csv mc.csv
```

### `run_stress_tests.py` — Stress Scenarios

Tests 8 adversarial/failure scenarios.

```bash
python run_stress_tests.py --all          # All 8 scenarios
python run_stress_tests.py --scenario SC-001  # Single scenario
```

| ID | Scenario | Description |
|----|----------|-------------|
| SC-001 | Low Volume Crash | 90% volume drop for 1 year |
| SC-002 | Validator Exodus | 50% quarterly churn for 6 months |
| SC-003 | Wash Trading Attack | 30% fake volume from 10 attackers |
| SC-004 | Stability Bank Run | 80% early exits after price crash |
| SC-005 | Fee Avoidance | 50% volume moves off-chain |
| SC-006 | Governance Attack | Parameter governance frozen 3 months |
| SC-007 | Ecological Shock | Ecological multiplier drops to 0 |
| SC-008 | Multi-Factor Crisis | Volume + price + validator crash |

### `analysis.py` — Post-hoc Analysis

Computes equilibrium derivations and summary statistics.

```bash
python analysis.py --from-run             # Run baseline then analyze
python analysis.py --baseline results.csv # Analyze saved results
```

## Model Architecture

### Simulation Pipeline (per epoch)

```
P1: Credit Market Activity
    |
    v
P2: Fee Collection (M013)
    |
    v
P3: Fee Distribution (M013) -> [burn, validator, community, agent] pools
    |
    v
P4: Mint/Burn (M012) -> Supply update
    |
    v
P5: Validator Compensation (M014)
    |
    v
P6: Contribution Rewards (M015) -> stability + activity distribution
    |
    v
P7: Agent Dynamics -> validator churn, stability adoption, price update
```

### File Structure

```
simulations/cadcad/
  model/
    __init__.py           # Package docstring
    state_variables.py    # Initial state vector (44 variables)
    params.py             # Parameters, sweep configs, stress test configs
    policies.py           # 7 policy functions (P1-P7)
    state_updates.py      # State update functions
    config.py             # cadCAD experiment builder
  run_baseline.py         # Baseline simulation runner
  run_sweep.py            # Parameter sweep runner
  run_monte_carlo.py      # Monte Carlo runner
  run_stress_tests.py     # Stress test runner
  analysis.py             # Equilibrium analysis
  equilibrium_analysis.md # Closed-form derivations
  requirements.txt        # Python dependencies
  README.md               # This file
```

## Success Criteria

The simulation validates these criteria from the spec:

| # | Criterion | Threshold |
|---|-----------|-----------|
| 1 | Validator sustainability | Annual income >= $15,000/validator (5yr mean) |
| 2 | Supply stability | Supply within [150M, 221M] REGEN (95th percentile) |
| 3 | Equilibrium convergence | abs(M-B) < 1% of S within 5 years |
| 4 | Reward pool adequacy | Activity pool > 0 in all periods |
| 5 | Stability tier solvency | Obligations met >= 95% of periods |
| 6 | Attack resistance | All stress scenarios pass |

## Key Findings

From the equilibrium analysis (`equilibrium_analysis.md`):

1. **Equilibrium supply**: ~219.85M REGEN (about 1.15M below cap), within 0.1% in ~3.1 years
2. **Validator sustainability gap**: At $500K/week baseline volume, validator income is ~$5,778/yr — well below $15,000. Minimum viable volume is ~$1.3M/week.
3. **Bootstrap requirement**: A ~$250K declining subsidy over 3 years bridges the gap until volume grows.
4. **Stability tier capacity**: At baseline volume, supports 6.5M REGEN (2.94% of supply).
5. **Wash trading**: Deeply unprofitable — break-even requires 32x higher reward rate than baseline.
6. **System stability**: Asymptotically stable; self-correcting via regrowth/burn feedback loop.

## Interpreting Results

The simulation outputs include:

- **Supply trajectory**: How S evolves from 224M (above cap) toward equilibrium
- **Mint/burn balance**: When M[t] approaches B[t], the system is near equilibrium
- **Validator income**: Both REGEN and USD terms (USD depends on price path)
- **Activity pool**: Available REGEN for activity-based rewards each period
- **Stability utilization**: What fraction of the 30% cap is consumed by stability tier

Key things to watch:
- If supply stays above 221M for many epochs, the initial burn-down is slow
- If validator income stays below $15K, the system needs higher volume or adjusted parameters
- If activity pool hits zero, the system is over-committed to stability tier
- In stress tests, watch for validator count dropping below 15 (min set) or 10 (Byzantine risk)
