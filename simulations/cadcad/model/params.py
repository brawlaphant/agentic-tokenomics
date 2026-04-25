"""
Parameter space for the Regen M012-M015 cadCAD simulation.

Contains baseline parameters, ranges for sweeps, and stress-test overrides.
Based on docs/economics/economic-simulation-spec.md Section 3.
"""

# ---------------------------------------------------------------------------
# Baseline parameters
# ---------------------------------------------------------------------------

baseline_params = {
    # === M012 Parameters (Supply Dynamics) ===
    'hard_cap': 221_000_000.0,          # C: hard cap in REGEN
    'base_regrowth_rate': 0.02,         # r_base: 2% per period
    'max_regrowth_rate': 0.10,          # Safety bound on r
    'staking_ratio': 0.30,              # Pre-PoA staking ratio
    'poa_active': True,                 # Whether PoA is active (post-transition)
    'equilibrium_threshold': 0.01,      # |M-B| < 1% of S for equilibrium
    'equilibrium_periods': 12,          # Consecutive periods for equilibrium detection

    # === M013 Parameters (Fee Routing) ===
    'fee_rate_issuance_bps': 200,       # 2% issuance fee
    'fee_rate_trade_bps': 100,          # 1% marketplace trade fee
    'fee_rate_retirement_bps': 50,      # 0.5% retirement fee
    'fee_rate_transfer_bps': 10,        # 0.1% transfer fee
    # Distribution shares: baseline uses SPEC Model A (30/40/25/5).
    # The OQ triage (PR #59) and governance proposals (PR #67) recommend
    # a reduced burn share of 15% with redistribution {15/30/50/5}.
    # The parameter sweep (burn_share_sweep) covers the full range [0, 0.35]
    # so both configurations are validated. Re-run baseline with --burn-share 0.15
    # to confirm sustainability at the proposed governance parameters.
    'burn_share': 0.30,                 # 30% to burn (Model A default)
    'validator_share': 0.40,            # 40% to validators
    'community_share': 0.25,            # 25% to community pool
    'agent_share': 0.05,               # 5% to agent infra
    'min_fee_regen': 1.0,              # Minimum fee floor (1 REGEN)

    # === M014 Parameters (Validator Governance) ===
    'min_validators': 15,
    'max_validators': 21,
    'validator_target': 18,
    'validator_bonus_share': 0.10,      # 10% performance bonus
    'min_viable_validator_income_usd': 15_000.0,  # $15K/year min
    'base_validator_churn': 0.05,       # 5% quarterly baseline churn
    'validator_application_rate': 1.0,  # 1 application per quarter (0.077/period)

    # === M015 Parameters (Contribution Rewards) ===
    'stability_annual_rate': 0.06,      # 6% annual stability tier return
    'max_stability_share': 0.30,        # 30% of community pool cap
    'periods_per_year': 52,             # Weekly epochs
    'activity_weights': {
        'purchase': 0.30,
        'retirement': 0.30,
        'facilitation': 0.20,
        'governance': 0.10,
        'proposals': 0.10,
    },
    'governance_vote_value': 1000.0,        # Proxy value per governance vote (USD-equivalent)
    'proposals_per_period': 2.0,            # Average proposals per period
    'proposal_value': 5000.0,               # Proxy value per proposal (USD-equivalent)
    'stability_adoption_rate': 5.0,         # New stability commitments per period
    'avg_stability_commitment': 25_000.0,   # Average commitment size (REGEN)
    'avg_stability_lock_periods': 52,       # Average lock = 1 year
    'stability_early_exit_rate': 0.001,     # 0.1% early exit per period

    # === Exogenous / Market Parameters ===
    'initial_weekly_volume_usd': 500_000.0,
    'volume_growth_rate': 0.005,            # 0.5% weekly growth
    'avg_credit_value_usd': 2_500.0,        # Average transaction value
    'credit_value_sigma': 0.8,              # Lognormal sigma for credit values
    'issuance_intensity': 2.5,              # Issuances per issuer per period
    'trade_intensity': 2.0,                 # Trades per buyer per period
    'retirement_intensity': 1.5,            # Retirements per retirer per period
    'transfer_intensity': 0.5,              # Transfers per agent per period
    'wash_trade_intensity': 10.0,           # Wash trades per wash trader per period
    'price_drift': 0.001,                   # Weekly price drift
    'price_volatility': 0.05,              # Weekly price volatility
    'price_mean_reversion_speed': 0.02,     # Mean reversion toward target
    'price_mean_reversion_target': 0.05,    # Long-run price target

    # === Simulation control ===
    'random_seed': 42,
}


# ---------------------------------------------------------------------------
# Parameter sweep configurations
# ---------------------------------------------------------------------------

sweep_params = {
    'r_base_sweep': {
        'base_regrowth_rate': [0.005, 0.01, 0.015, 0.02, 0.03, 0.04, 0.06, 0.08, 0.10],
    },
    'burn_share_sweep': {
        'burn_share': [0.00, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35],
        # Adjust validator_share to maintain sum = 1.0 (community + agent stay fixed)
    },
    'fee_rate_sweep': {
        'fee_rate_issuance_bps': [100, 150, 200, 250, 300],
        'fee_rate_trade_bps': [50, 75, 100, 150, 200],
    },
    'stability_rate_sweep': {
        'stability_annual_rate': [0.02, 0.03, 0.04, 0.05, 0.06, 0.08, 0.10, 0.12],
    },
    'volume_sweep': {
        'initial_weekly_volume_usd': [
            50_000, 100_000, 250_000, 500_000,
            1_000_000, 2_500_000, 5_000_000, 10_000_000,
        ],
    },
}


def get_sweep_param_set(sweep_name: str) -> list[dict]:
    """Return a list of parameter overrides for a given sweep.

    For sweeps that require co-varying parameters (e.g. burn_share needs
    validator_share adjusted to maintain sum = 1.0), the adjustments are
    computed here.
    """
    configs = []
    if sweep_name == 'burn_share_sweep':
        # Baseline non-burn shares used as proportional weights
        base_vs = baseline_params['validator_share']
        base_cs = baseline_params['community_share']
        base_ags = baseline_params['agent_share']
        base_non_burn = base_vs + base_cs + base_ags  # 0.70 at baseline

        for bs in sweep_params['burn_share_sweep']['burn_share']:
            remaining = 1.0 - bs
            if remaining <= 0:
                # Degenerate: everything burned
                vs, cs, ags = 0.0, 0.0, 0.0
            else:
                # Redistribute remaining proportionally among validator/community/agent
                scale = remaining / base_non_burn
                vs = max(0.0, base_vs * scale)
                cs = max(0.0, base_cs * scale)
                ags = max(0.0, base_ags * scale)
            configs.append({
                'burn_share': bs,
                'validator_share': vs,
                'community_share': cs,
                'agent_share': ags,
            })
    elif sweep_name == 'fee_rate_sweep':
        for iss in sweep_params['fee_rate_sweep']['fee_rate_issuance_bps']:
            for trade in sweep_params['fee_rate_sweep']['fee_rate_trade_bps']:
                configs.append({
                    'fee_rate_issuance_bps': iss,
                    'fee_rate_trade_bps': trade,
                })
    else:
        sweep_def = sweep_params[sweep_name]
        keys = list(sweep_def.keys())
        if len(keys) == 1:
            key = keys[0]
            for val in sweep_def[key]:
                configs.append({key: val})
        else:
            # Multi-key: zip
            values = list(sweep_def.values())
            for combo in zip(*values):
                configs.append(dict(zip(keys, combo)))
    return configs


# ---------------------------------------------------------------------------
# Stress test parameter overrides
# ---------------------------------------------------------------------------

stress_test_params = {
    'SC-001': {
        'name': 'Low Credit Volume (90% Drop)',
        'description': 'Volume drops 90% at epoch 52, recovers partially by epoch 130',
        'volume_schedule': [
            (0, 51, 500_000),
            (52, 103, 50_000),
            (104, 129, 'linear_recovery'),  # 50K -> 250K
            (130, 520, 250_000),
        ],
        'overrides': {},
    },
    'SC-002': {
        'name': 'High Validator Churn (50%/quarter)',
        'description': 'Validator churn 10x baseline starting epoch 26 for 6 months',
        'volume_schedule': None,
        'overrides': {},
        'churn_schedule': [
            (0, 25, 0.05),
            (26, 51, 0.50),
            (52, 520, 0.05),
        ],
        'application_schedule': [
            (0, 51, 1.0),
            (52, 77, 3.0),
            (78, 520, 1.0),
        ],
    },
    'SC-003': {
        'name': 'Wash Trading Attack (30% of Volume)',
        'description': '10 wash traders generating 30% of volume from epoch 13',
        'volume_schedule': None,
        'overrides': {},
        'wash_trader_schedule': [
            (0, 12, 0),
            (13, 520, 10),
        ],
    },
    'SC-004': {
        'name': 'Stability Tier Bank Run (80% Early Exits)',
        'description': '80% of stability holders exit at epoch 78 after price crash',
        'volume_schedule': None,
        'overrides': {},
        'stability_bank_run_epoch': 78,
        'bank_run_exit_fraction': 0.80,
        'price_crash_epoch': 78,
        'price_crash_factor': 0.40,  # Price drops to 40% of current
    },
    'SC-005': {
        'name': 'Fee Avoidance (50% Off-Chain)',
        'description': 'Gradual migration to off-chain, 50% volume lost by epoch 52',
        'volume_schedule': [
            (0, 25, 500_000),
            (26, 51, 'linear_decline'),  # 500K -> 250K
            (52, 520, 250_000),
        ],
        'overrides': {},
    },
    'SC-006': {
        'name': 'Governance Attack on Parameters',
        'description': 'Governance deadlock: no parameter changes for 3 months at epoch 52',
        'volume_schedule': None,
        'overrides': {},
        'governance_freeze_start': 52,
        'governance_freeze_end': 65,
    },
    'SC-007': {
        'name': 'Ecological Multiplier Shock',
        'description': 'Ecological multiplier drops to 0 for 12 weeks at epoch 52',
        'volume_schedule': None,
        'overrides': {},
        'eco_mult_schedule': [
            (0, 51, 1.0),
            (52, 63, 0.0),
            (64, 520, 1.0),
        ],
    },
    'SC-008': {
        'name': 'Correlated Multi-Factor Crisis',
        'description': 'Volume crash + price crash + validator churn simultaneously',
        'volume_schedule': [
            (0, 51, 500_000),
            (52, 77, 100_000),
            (78, 103, 'linear_recovery'),  # 100K -> 300K
            (104, 520, 300_000),
        ],
        'overrides': {},
        'churn_schedule': [
            (0, 51, 0.05),
            (52, 77, 0.30),
            (78, 520, 0.05),
        ],
        'price_crash_epoch': 52,
        'price_crash_factor': 0.30,
    },
    # -----------------------------------------------------------------
    # Additional stress scenarios (SC-009 through SC-011)
    #
    # All three reuse the existing schedule hooks (volume_schedule,
    # eco_mult_schedule) so no policy or state-update code changes are
    # required. Each exercises a failure mode the original SC-001 through
    # SC-008 set does not cover:
    #   SC-009 — CHRONIC decline, not acute crash (vs SC-001/SC-005)
    #   SC-010 — ACUTE shock with FAST recovery (vs SC-001's slow recovery)
    #   SC-011 — OSCILLATION in the ecological multiplier (vs SC-007's binary shock)
    # -----------------------------------------------------------------
    'SC-009': {
        'name': 'Gradual Volume Decay (3-Year Linear Decline)',
        'description': (
            'Volume declines linearly from $500K to $100K per week over '
            'the full simulation horizon (156 epochs). Tests whether the '
            'system degrades gracefully under chronic stress rather than '
            'the acute shocks covered by SC-001 and SC-005. The absence '
            'of a sharp discontinuity distinguishes this scenario from '
            'every existing volume-shock case in the suite.'
        ),
        # Thirteen quarter-year segments step volume down from $500K to
        # $100K on a straight line. The runner resolves each (start, end,
        # value) tuple by epoch range, so the staircase IS the linear
        # decline at the resolution the simulation engine operates on.
        'volume_schedule': [
            (0, 11, 500_000),
            (12, 23, 470_000),
            (24, 35, 440_000),
            (36, 47, 410_000),
            (48, 59, 380_000),
            (60, 71, 340_000),
            (72, 83, 300_000),
            (84, 95, 260_000),
            (96, 107, 220_000),
            (108, 119, 180_000),
            (120, 131, 150_000),
            (132, 143, 120_000),
            (144, 520, 100_000),
        ],
        'overrides': {},
    },
    'SC-010': {
        'name': 'Flash Crash and Rapid Recovery (4-Week Shock)',
        'description': (
            'Volume drops 80% for 4 weeks starting at epoch 52, then '
            'recovers linearly over the next 4 weeks to 110% of the '
            'pre-shock baseline. Tests that short-duration shocks do '
            'not trigger cascading failures — validators must survive '
            'the dip without dropping below the Byzantine tolerance '
            'floor, and the stability tier must not bank-run under a '
            'brief price panic. Different from SC-001 (1-year slow '
            'recovery) and from SC-005 (slow permanent decline).'
        ),
        'volume_schedule': [
            (0, 51, 500_000),        # pre-shock baseline
            (52, 55, 100_000),       # 80% drop, 4 weeks
            (56, 59, 'linear_recovery'),  # linear bounce back
            (60, 520, 550_000),      # 110% post-shock, demand overshoots
        ],
        'overrides': {},
    },
    'SC-011': {
        'name': 'Ecological Multiplier Oscillation',
        'description': (
            'Ecological multiplier wobbles between 0.5 and 1.5 every '
            'four weeks for six months. Tests that the M012 supply '
            'curve remains stable under sustained oscillation rather '
            'than the binary on/off shock SC-007 models. A divergent '
            'response here would indicate a resonance between the '
            'regrowth rate and the oscillation period — a subtle '
            'failure mode the baseline and existing stress tests '
            'cannot detect.'
        ),
        'volume_schedule': None,
        'overrides': {},
        'eco_mult_schedule': [
            (0, 51, 1.0),
            (52, 55, 0.5),
            (56, 59, 1.5),
            (60, 63, 0.5),
            (64, 67, 1.5),
            (68, 71, 0.5),
            (72, 75, 1.5),
            (76, 520, 1.0),
        ],
    },
}
