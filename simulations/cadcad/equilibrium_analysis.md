# Equilibrium Analysis for Regen Economic Reboot (M012-M015)

## 1. Supply Equilibrium Derivation

### 1.1 Setup

The M012 supply dynamics are governed by:

```
S[t+1] = S[t] + M[t] - B[t]
```

Where:
- `S[t]` = circulating supply at period t (REGEN)
- `M[t]` = tokens minted (regrowth)
- `B[t]` = tokens burned (from fee revenue)
- `C` = 221,000,000 REGEN (hard cap)

Minting:
```
M[t] = r * max(0, C - S[t])
r = r_base * effective_multiplier * ecological_multiplier
```

Burning:
```
B[t] = burn_share * F[t]
F[t] = V * w_avg / P_regen    (total fees in REGEN per period)
```

Where:
- `V` = weekly transaction volume (USD)
- `w_avg` = weighted average fee rate (dimensionless)
- `P_regen` = REGEN price (USD)
- `burn_share` = fraction of fees routed to burn (0.30 baseline)

### 1.2 Equilibrium Condition

At equilibrium, `M[t] = B[t]`, so the supply is unchanging:

```
r * (C - S*) = burn_share * V * w_avg / P_regen
```

Solving for `S*`:

```
C - S* = (burn_share * V * w_avg) / (r * P_regen)

S* = C - (burn_share * V * w_avg) / (r * P_regen)
```

### 1.3 Baseline Equilibrium

With baseline parameters:
- C = 221,000,000 REGEN
- burn_share = 0.30
- V = $500,000/week
- w_avg = ~0.01 (weighted average across transaction types)
- r = 0.02 * 1.0 * 1.0 = 0.02 (r_base * eff_mult * eco_mult, with no stability commitments)
- P_regen = $0.05

```
S* = 221,000,000 - (0.30 * 500,000 * 0.01) / (0.02 * 0.05)
S* = 221,000,000 - 1,500 / 0.001
S* = 221,000,000 - 1,500,000
S* = 219,500,000 REGEN
```

With the effective multiplier at 1.3 (30% staking/stability ratio):
```
r = 0.02 * 1.3 * 1.0 = 0.026
S* = 221,000,000 - 1,500 / 0.0013
S* = 221,000,000 - 1,153,846
S* ≈ 219,846,154 REGEN
```

The equilibrium supply is approximately 219.85M REGEN, about 1.15M below the cap.

### 1.3.1 Governance Proposal Variant (burn_share = 0.15)

The governance proposal drafts (docs/governance/needs-governance-proposals.md) recommend
a reduced burn share of 15% with redistribution to community pool {15/30/50/5}. At this
burn share:

```
S* = 221,000,000 - (0.15 * 500,000 * 0.01) / (0.026 * 0.05)
S* = 221,000,000 - 750 / 0.0013
S* = 221,000,000 - 576,923
S* ≈ 220,423,077 REGEN
```

With 15% burn, equilibrium supply rises to ~220.42M — only 577K below the cap, vs 1.15M
with 30% burn. The deflationary effect is halved. Validator income increases because the
validator share rises from 40% to 30% of a larger non-burn pool. Run `python run_sweep.py
--sweep burn_share_sweep` to see the full sensitivity curve.

### 1.4 Sensitivity of S* to Key Parameters

Taking the partial derivative of `S*` with respect to each parameter:

```
∂S*/∂V = -burn_share * w_avg / (r * P)
       = -0.30 * 0.01 / (0.026 * 0.05)
       = -2.31 REGEN per $1/week of volume
```

This means a $100,000/week increase in volume reduces equilibrium supply by about 230,769 REGEN.

```
∂S*/∂(burn_share) = -V * w_avg / (r * P)
                  = -500,000 * 0.01 / (0.026 * 0.05)
                  = -3,846,154 REGEN per unit burn_share
```

A 5% increase in burn_share (0.30 -> 0.35) reduces equilibrium supply by ~192,308 REGEN.

```
∂S*/∂r = (burn_share * V * w_avg) / (r^2 * P)
       = 1,500 / (0.000676 * 0.05)
       = 44,378,698 REGEN per unit r
```

Doubling r from 0.026 to 0.052 would raise S* by about 576,923 REGEN (closer to cap).

```
∂S*/∂P = (burn_share * V * w_avg) / (r * P^2)
       = 1,500 / (0.026 * 0.0025)
       = 23,076,923 REGEN per USD of price
```

Doubling REGEN price from $0.05 to $0.10 halves the REGEN quantity burned, raising S* by ~576,923 REGEN.

**Summary of S* sensitivities:**

| Parameter | Change | S* Change | Direction |
|-----------|--------|-----------|-----------|
| Volume 2x ($1M/wk) | +$500K/wk | -1.15M REGEN | More burn = lower equilibrium |
| Volume 0.5x ($250K/wk) | -$250K/wk | +577K REGEN | Less burn = higher equilibrium |
| Burn share +5% (0.35) | +0.05 | -192K REGEN | More burn fraction = lower |
| Regrowth rate 2x (0.052) | +0.026 | +577K REGEN | Faster regrowth = higher |
| REGEN price 2x ($0.10) | +$0.05 | +577K REGEN | Higher price = fewer REGEN burned |

## 2. Convergence Dynamics

### 2.1 Exponential Convergence

Starting from `S_0`, the supply converges to `S*` exponentially:

```
S[t] - S* ≈ (S_0 - S*) * (1 - r)^t
```

This assumes constant r and constant burn rate, which is approximately true near equilibrium.

### 2.2 Time to Convergence

Time to reach within epsilon of equilibrium:

```
t_converge = log(epsilon / |S_0 - S*|) / log(1 - r)
```

**From initial conditions (S_0 = 224M, S* ≈ 219.85M, r = 0.026):**

Note: Since S_0 > C, the initial phase is pure-burn (M[t] = 0) until S drops below C.
The initial burn-down phase lasts approximately:

```
Epochs to burn from 224M to 221M:
  Weekly burn ≈ burn_share * V * w_avg / P = 0.30 * 500,000 * 0.01 / 0.05 = 30,000 REGEN/week
  Gap = 224M - 221M = 3M REGEN
  Epochs ≈ 3,000,000 / 30,000 = 100 epochs (~2 years)
```

After reaching 221M, convergence to S* follows the exponential:

```
|221M - 219.85M| = 1.15M REGEN

For 1% convergence (epsilon = 0.01 * S* ≈ 2.2M):
  The gap (1.15M) is already smaller than epsilon (2.2M), so the system is
  within 1% of S* as soon as it drops below the cap.  t = 0 additional periods.

For 0.1% convergence (epsilon = 0.001 * S* ≈ 220K):
  t = log(epsilon / gap) / log(1 - r)
    = log(220K / 1.15M) / log(0.974)
    = log(0.191) / log(0.974)
    = (-1.655) / (-0.0263)
    ≈ 63 periods (~1.2 years)
```

**Total convergence time from activation:**
- Pure-burn phase: ~100 epochs (1.9 years)
- Already within 1% of S* at end of burn-down phase
- Exponential convergence to 0.1%: ~63 additional epochs (~1.2 years)
- Total to tight equilibrium: ~163 epochs (~3.1 years)

## 3. Stability Conditions

### 3.1 When is the system stable?

The equilibrium `S*` is stable (self-correcting) when:

1. **S < S***: Supply below equilibrium means M[t] > B[t] (larger gap = more minting), so supply increases toward S*.
2. **S > S***: Supply above equilibrium means M[t] < B[t] (smaller gap = less minting), so supply decreases toward S*.
3. **S > C**: Supply above cap means M[t] = 0 and B[t] > 0, so supply strictly decreases.

This is inherently stable as long as:
- `r > 0` (regrowth is active)
- `burn_share > 0` (some fees are burned)
- `V > 0` (there is transaction activity)

The eigenvalue of the linearized system is `(1 - r)`, which is between 0 and 1 for all valid r, confirming asymptotic stability.

### 3.2 Instability conditions

The system becomes unstable or degenerate when:
- `V → 0`: No transaction volume means no fees, no burns, and supply monotonically increases toward C. This is the volume death spiral.
- `burn_share = 0`: No burning means supply monotonically increases to C and stays there. The system is still "stable" at S = C but has no deflationary mechanism.
- `r = 0`: No regrowth means supply monotonically decreases (only burns). Eventually S → 0 if burning continues.
- `P_regen → 0`: Fees in REGEN terms explode, causing massive burns that drive S to 0.

## 4. Validator Sustainability Threshold

### 4.1 Minimum Viable Volume

For validators to earn at least `I_min` per year:

```
V_weekly >= (I_min * N_val) / (52 * w_avg * val_share)
```

At baseline:
```
V_weekly >= ($15,000 * 18) / (52 * 0.01 * 0.40)
V_weekly >= $270,000 / 0.208
V_weekly >= $1,298,077 ≈ $1.3M/week
```

**This is the critical finding:** At the proposed baseline volume of $500K/week, the fee-funded validator income is approximately $5,778/year — far below the $15,000 minimum. The system requires either:

1. **Higher volume** ($1.3M/week minimum), or
2. **Higher fee rates** (weighted average ~2.6% instead of 1%), or
3. **Fewer validators** (7 validators at $500K/week yields ~$14,857/year), or
4. **Bootstrap funding** (treasury subsidy that declines as volume grows), or
5. **Higher REGEN price** (does not help — fees are USD-denominated and converted)

Note on REGEN price: Since fees are computed as a percentage of credit value in USD and then converted to REGEN, a higher REGEN price means fewer REGEN per fee but same USD value. Validator income in USD is independent of REGEN price. Only the REGEN quantity changes.

### 4.2 Volume-Income Relationship

| Weekly Volume | Annual Val Income ($/yr/val) | Meets $15K? |
|---------------|------------------------------|-------------|
| $250K | $2,889 | No |
| $500K | $5,778 | No |
| $750K | $8,667 | No |
| $1.0M | $11,556 | No |
| $1.3M | $15,022 | Yes (marginal) |
| $2.0M | $23,111 | Yes |
| $5.0M | $57,778 | Yes (comfortable) |

### 4.3 Recommended Bootstrap Model

A linear-declining treasury subsidy bridges the gap:

```
subsidy[t] = max(0, subsidy_initial * (1 - t / T_runway))

where:
  subsidy_initial = ($15,000 - fee_income) * N_val / 52  per week
  T_runway = 156 epochs (3 years)
```

At $500K/week baseline:
```
Weekly gap = ($15,000 - $5,778) * 18 / 52 = $3,192/week
Total bootstrap fund needed: $3,192 * 156 / 2 = $249,012

With 5% annual volume growth:
  Volume at year 3: $500K * (1.05)^3 = $578K
  Income at year 3: $6,681/yr — still below $15K
  Actual runway needed is longer, or growth must be faster.
```

## 5. Stability Tier Capacity

### 5.1 Maximum Supportable Commitments

The stability tier is sustainable when obligations do not exceed the 30% cap:

```
commitments * rate / periods_per_year <= community_inflow * max_stability_share

commitments <= (V * w_avg * community_share * max_stability_share * periods_per_year) / (rate * P)
```

Note: Commitments are in REGEN, so we must convert community_inflow from USD to REGEN.

At baseline:
```
Annual community USD = $500,000 * 0.01 * 0.25 * 52 = $65,000
30% cap in USD = $19,500
Max REGEN at 6% = $19,500 / 0.06 = $325,000 worth = 6,500,000 REGEN
```

### 5.2 Stability Tier by Volume Level

| Weekly Volume | Max Commitments (M REGEN) | As % of Supply |
|---------------|--------------------------|----------------|
| $100K | 1.3M | 0.59% |
| $500K | 6.5M | 2.94% |
| $1M | 13.0M | 5.88% |
| $2.5M | 32.5M | 14.71% |
| $5M | 65.0M | 29.41% |

At baseline volume, the stability tier is a niche feature supporting at most 2.94% of supply.

## 6. Wash Trading Break-Even

### 6.1 Attack Economics

A wash trader executing buy-transfer-sell cycles pays:
```
fee_per_cycle = value * (0.01 + 0.001 + 0.01) = value * 0.021 (2.1%)
```

And generates activity score:
```
score_per_cycle = value * 0.30 (only purchase weight applies)
```

The reward rate (REGEN per unit score) must exceed `0.021 / 0.30 = 0.07` (7%) for profitability.

### 6.2 Baseline Reward Rate

```
activity_pool_weekly ≈ $65,000/yr * 0.70 / 52 = $875/week
total_score ≈ $500,000 * 0.80 weight = 400,000 (approximate)
reward_rate = $875 / 400,000 = 0.0022 = 0.22%
```

The baseline reward rate (0.22%) is 32x below the break-even (7%). Wash trading is deeply unprofitable.

### 6.3 When Does It Become Profitable?

Wash trading becomes profitable when:
```
activity_pool / total_activity_score > 0.07
```

This requires either:
- Activity pool increases 32x (requires ~$16M/week volume), or
- Total legitimate activity drops 32x (near-zero legitimate participation), or
- Both increase/decrease partially

Under any realistic growth scenario, wash trading remains unprofitable because fee costs scale linearly with attack volume while rewards are diluted across all participants.

## 7. Summary of Key Findings

| Finding | Value | Implication |
|---------|-------|-------------|
| Equilibrium supply S* | ~219.85M REGEN | System converges to ~1.15M below cap |
| Time to equilibrium (0.1%) | ~3.1 years from activation | Includes ~1.9 year initial burn-down phase |
| Min viable volume | $1.3M/week | Current baseline ($500K) is insufficient for validators |
| Bootstrap fund needed | ~$250K over 3 years | Declining subsidy until volume grows |
| Max stability commitments | 6.5M REGEN at baseline vol | 2.94% of supply — niche feature at current scale |
| Wash trading break-even | 7% reward rate | 32x above baseline — deeply unprofitable |
| System stability | Asymptotically stable | Self-correcting as long as r > 0, V > 0, burn > 0 |
