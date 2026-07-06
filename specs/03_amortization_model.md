# Amortization Model Specification

**Status**: Draft  
**Purpose**: Define opportunity-cost calculations for debt payoff scenarios  
**Scope**: Per-account and aggregated debt payoff projections with acceleration scenarios  
**Dependencies**: `debt_accounts` schema, APR/min-payment user-input fields, SimpleFIN balance sync

---

## 1. Core Payoff Calculation

### 1.1 Standard Amortization Formula

For a single debt account with:
- **B** = current balance (dollars)
- **r** = annual interest rate / APR (decimal, e.g., 0.18 for 18%)
- **M** = monthly payment (dollars)

**Calculate months to payoff:**
```
r_m = r / 12                              [monthly rate]
n = -ln(1 - r_m * B / M) / ln(1 + r_m)   [months until paid off]
```

**Conditions:**
- `n` must be finite and positive: requires `M > r_m * B` (payment exceeds monthly interest)
  - If `M <= r_m * B`, balance grows or stays flat → undefined payoff (flag as "unsustainable")
  - Example: $10k balance @ 18% APR, payment $100/mo → $150/mo interest → unsustainable
- If `r = 0` (no interest), use simplified: `n = B / M`

### 1.2 Monthly Interest Breakdown

For any month *i* (1-indexed):
- **Balance at start of month i**: B_i = B_{i-1} - (M - I_{i-1})
- **Interest accrued in month i**: I_i = B_i * r / 12
- **Principal paid in month i**: P_i = M - I_i

Cumulative interest over `n` months:
```
total_interest = sum(I_i for i = 1 to n)
```

**Closed-form approximation** (good for large n):
```
total_interest ≈ M * n - B
```

**Exact calculation**: iterate month-by-month (preferred for precision in code).

---

## 2. Scenario Template: Payment Acceleration

### 2.1 Single Debt Account Scenario

**Inputs:**
- Current debt state: B, r, M (from `debt_accounts` table)
- Acceleration amount: ΔX (dollars/month, e.g., $200 extra from cutting discretionary spend)

**Outputs:**

| Metric | Formula | Interpretation |
|--------|---------|-----------------|
| **New monthly payment** | M_new = M + ΔX | Total paid per month |
| **New payoff time** | n_new = -ln(1 - r_m * B / M_new) / ln(1 + r_m) | Months to zero balance |
| **Months saved** | Δn = n - n_new | How much faster paid off |
| **Baseline interest** | I_base = sum(interest over n months) | Total interest @ M payment |
| **Accelerated interest** | I_accel = sum(interest over n_new months) | Total interest @ M_new payment |
| **Interest saved** | ΔI = I_base - I_accel | Absolute dollar savings |
| **Interest saved (%)** | ΔI / I_base * 100 | Relative reduction |

### 2.2 Calculation Logic

```pseudocode
function calculate_scenario(account: DebtAccount, delta_x: number) -> Scenario {
  r_m = account.apr / 12
  M_new = account.minimum_payment + delta_x
  
  // Check sustainability
  if M_new <= r_m * account.balance {
    return {
      status: "unsustainable",
      reason: "Payment insufficient to cover interest"
    }
  }
  
  // Current payoff
  if account.apr == 0 {
    n = account.balance / account.minimum_payment
  } else {
    n = -log(1 - r_m * account.balance / account.minimum_payment) / log(1 + r_m)
  }
  I_base = iterate_interest(account.balance, r_m, account.minimum_payment, n)
  
  // Accelerated payoff
  if account.apr == 0 {
    n_new = account.balance / M_new
  } else {
    n_new = -log(1 - r_m * account.balance / M_new) / log(1 + r_m)
  }
  I_accel = iterate_interest(account.balance, r_m, M_new, n_new)
  
  return {
    account_id: account.id,
    account_name: account.name,
    months_saved: floor(n - n_new),
    months_to_payoff_baseline: ceil(n),
    months_to_payoff_accelerated: ceil(n_new),
    interest_saved_dollars: I_base - I_accel,
    interest_saved_percent: (I_base - I_accel) / I_base * 100 if I_base > 0 else 0,
    apr: account.apr,
    current_balance: account.balance
  }
}

function iterate_interest(balance: number, r_m: number, payment: number, months: number) -> number {
  total_interest = 0
  current_balance = balance
  for i in 1..ceil(months):
    monthly_interest = current_balance * r_m
    principal = min(payment - monthly_interest, current_balance)
    total_interest += monthly_interest
    current_balance -= principal
    if current_balance <= 0: break
  return total_interest
}
```

---

## 3. Aggregation: Multiple Debt Accounts

### 3.1 Portfolio-Level Metrics

When user has N debt accounts (credit cards, personal loans, etc.):

| Metric | Formula | Interpretation |
|--------|---------|-----------------|
| **Total months saved** | Σ Δn_i | Sum across all accounts |
| **Total interest saved** | Σ ΔI_i | Sum across all accounts |
| **Weighted avg APR** | (Σ B_i * r_i) / Σ B_i | Account-balance-weighted average |
| **Total current balance** | Σ B_i | Total debt |
| **Total baseline interest** | Σ I_base_i | Sum of interest across all accounts |
| **Total accelerated interest** | Σ I_accel_i | Sum of interest with acceleration |

### 3.2 Example Aggregation

Scenario: Cut discretionary spending by $500/month across 2 credit cards.

| Account | Balance | APR | Min Pmt | Months (Current) | Interest (Current) | Months (Accel) | Interest (Accel) | Saved |
|---------|---------|-----|---------|-----|----------|----|---------|----|
| Card A (Chase) | $5,000 | 22% | $150 | 44.2 | $1,891 | 18.5 | $604 | $1,287 |
| Card B (Amex) | $3,000 | 18% | $90 | 51.3 | $931 | 21.1 | $267 | $664 |
| **TOTAL** | **$8,000** | **20.5%** (weighted) | **$240** | **95.5 mo** | **$2,822** | **39.6 mo** | **$871** | **$1,951** |

The $500/month acceleration saves ~$1,951 in interest and pays off the portfolio 4.7 years faster.

---

## 4. Data Sources & Schema

### 4.1 Input Data

All per-account parameters come from the `debt_accounts` table:

```sql
CREATE TABLE debt_accounts (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,           -- FK to accounts.id
  name TEXT NOT NULL,                 -- User-friendly name (e.g., "Chase Freedom")
  apr REAL NOT NULL DEFAULT 0.0,      -- Annual percentage rate (decimal; 18% = 0.18)
                                      -- SOURCE: user-input via set_debt_terms command
  minimum_payment REAL NOT NULL,      -- Monthly payment (dollars)
                                      -- SOURCE: user-input OR default to 2% of balance
  created_at TEXT NOT NULL,           -- ISO 8601 timestamp
  updated_at TEXT NOT NULL            -- Last modified (when user edits APR/payment)
);
```

### 4.2 Balance Source

- **B (balance)**: Synced from SimpleFIN `/accounts` response → stored in `accounts.balance`
- **Last sync**: recorded in `sync_log.sync_date` per account_id
- **Staleness check**: if last-sync > 24h, flag as stale in UI ("data as of [date]")

### 4.3 Default Minimum Payment

If user hasn't set `minimum_payment`, calculate as:
```
minimum_payment = max(balance * 0.02, 25)  [2% of balance, minimum $25]
```

Store this derived value in `debt_accounts.minimum_payment` (user can override).

### 4.4 APR Handling

**Why SimpleFIN doesn't provide APR:**
- SimpleFIN API only returns balance, type, org, account number
- Interest rates are set by issuer and vary per cardholder
- Must be user-entered via UI

**APR input validation:**
- Range: 0% ≤ APR ≤ 35% (flag outside range as warning)
- Precision: store as decimal to 4 places (0.0001 = 0.01%)
- Update mechanism: `set_debt_terms(account_id, apr, minimum_payment)` command

---

## 5. Scenario Templates

### 5.1 Standard Scenarios

The app should always display these three acceleration scenarios for each debt:

#### Scenario A: $200/month acceleration
```
Δ X = $200
Useful for: modest lifestyle optimization (skip lunches, cancel subscriptions)
```

#### Scenario B: $500/month acceleration
```
Δ X = $500
Useful for: significant spending cut (reduce dining, entertainment, travel)
```

#### Scenario C: Custom amount
```
Δ X = user-specified (e.g., $750, $1,200)
Entered via modal/input in Opportunity Cost card
```

### 5.2 Scenario Display Format

For each scenario, show to user:

```
Scenario: Cut discretionary spending by $500/month

📊 Impact on [Account Name] ($5,000 @ 18% APR):
  • Payoff in 18.5 months (was 44.2 months)
  • Save 25.7 months
  • Interest savings: $1,287 (68% less)

💰 Portfolio Impact ($8,000 total debt):
  • Payoff in 39.6 months (was 95.5 months)
  • Save 55.9 months (4.7 years!)
  • Interest savings: $1,951
```

---

## 6. Example Calculations

### Example 1: Single Account, $200/month Acceleration

**Inputs:**
- Balance: $5,000
- APR: 22% (0.22 annual, 0.01833 monthly)
- Current payment: $150/month
- Acceleration: $200/month
- New payment: $350/month

**Baseline (M = $150/month):**
```
r_m = 0.22 / 12 = 0.01833
n = -ln(1 - 0.01833 * 5000 / 150) / ln(1.01833)
  = -ln(1 - 0.6111) / ln(1.01833)
  = -ln(0.3889) / 0.0182
  = 0.9443 / 0.0182
  = 51.89 months ≈ 4 years 4 months

Iterated interest (month-by-month):
Month 1: I = 5000 * 0.01833 = $91.67, Principal = $150 - $91.67 = $58.33, Bal = $4,941.67
Month 2: I = 4941.67 * 0.01833 = $90.52, Principal = $59.48, Bal = $4,882.19
...
Month 52: I = $15.42, Principal = $134.58, Bal ≈ $0

Total interest (summed): approximately $2,649
```

**Accelerated (M = $350/month):**
```
n = -ln(1 - 0.01833 * 5000 / 350) / ln(1.01833)
  = -ln(1 - 0.2619) / ln(1.01833)
  = -ln(0.7381) / 0.0182
  = 0.3039 / 0.0182
  = 16.69 months ≈ 1 year 5 months

Iterated interest:
Month 1: I = 5000 * 0.01833 = $91.67, Principal = $258.33, Bal = $4,741.67
...
Month 17: I = ~$27, Principal = ~$323, Bal ≈ $0

Total interest (summed): approximately $1,153
```

**Scenario Result:**
```
Months saved: 51.89 - 16.69 = 35.2 months (2.9 years)
Interest saved: $2,649 - $1,153 = $1,496
Percent saved: 1496 / 2649 = 56.5%
```

### Example 2: Multiple Accounts, $500/month Allocation

**Scenario:** User decides to allocate $500/month extra, split across 2 cards proportional to balance.

**Inputs:**
- Card A: $5,000 @ 22%, $150/month → allocation = $312.50
- Card B: $3,000 @ 18%, $90/month → allocation = $187.50

**Card A with $312.50 extra:**
```
New payment: $150 + $312.50 = $462.50
n_new = -ln(1 - 0.01833 * 5000 / 462.50) / ln(1.01833)
      = -ln(0.802) / 0.0182
      = 11.8 months

Interest saved: $2,649 - $835 = $1,814
```

**Card B with $187.50 extra:**
```
New payment: $90 + $187.50 = $277.50
n_new = -ln(1 - 0.015 * 3000 / 277.50) / ln(1.015)
      = -ln(0.838) / 0.0149
      = 12.4 months

Interest saved: $931 - $518 = $413
```

**Portfolio Result:**
```
Total months saved: (44.2 - 11.8) + (51.3 - 12.4) = 32.4 + 38.9 = 71.3 months (5.9 years)
Total interest saved: $1,814 + $413 = $2,227
Weighted APR: (5000 * 0.22 + 3000 * 0.18) / 8000 = 1480 / 8000 = 18.5%
```

---

## 7. Edge Cases & Assumptions

### 7.1 Zero Interest (APR = 0)

**Case:** Promotional 0% APR card or student loan with no interest.

**Handling:**
```
n = B / M  (simple division)
total_interest = 0
scenario_interest_saved = 0
```

**Display:** "No interest charges; you control the payoff timeline."

### 7.2 Very Low Balance

**Case:** $50 balance @ 18% APR, $25/month payment.

**Handling:**
```
r_m = 0.015 (0.18 / 12)
n = -ln(1 - 0.015 * 50 / 25) / ln(1.015)
  = -ln(1 - 0.03) / 0.0149
  = -ln(0.97) / 0.0149
  = 0.0305 / 0.0149
  = 2.05 months

total_interest ≈ $0.76
```

**Display:** "Pay off in ~2 months; minimal interest."

### 7.3 Unsustainable Payment

**Case:** $10,000 balance @ 18% APR, $100/month payment.

**Baseline:**
```
r_m = 0.015
monthly_interest = 10000 * 0.015 = $150
$100 payment < $150 interest → balance grows
```

**Handling:**
```
if minimum_payment <= monthly_interest {
  status = "UNSUSTAINABLE"
  message = "Minimum payment ($100) doesn't cover interest ($150). "
            "Increase payment to at least $150/month to make progress."
  months_to_payoff = null
  interest_saved = 0
}
```

**UI:** Display alert card with red background, actionable next steps.

### 7.4 Acceleration Beyond Payoff

**Case:** User specifies $5,000/month extra on a $5,000 balance.

**Handling:**
```
new_payment = minimum_payment + extra
if new_payment >= balance {
  months_to_payoff = 1
  # Calculate interest for 1 month only
  interest = balance * r_m
}
```

**Display:** "You could pay this off next month (~$X interest)."

### 7.5 Rounding & Precision

**Rules:**
- Months: round up to integer (ceiling) for display
  - e.g., 16.69 months → "17 months"
- Interest: round to nearest cent
  - e.g., $1,493.67
- APR: display as "18.50%" with 2 decimal places
- Interest saved %: round to 1 decimal (e.g., 56.5%)

### 7.6 Data Freshness

**Scenario:** User hasn't synced SimpleFIN in 30 days; balance is stale.

**Handling:**
```
if (now - last_sync) > 24 hours {
  stale = true
  display_note = "⚠ Data as of [date]. Sync for current balances."
}
```

**Behavior:**
- Still calculate scenarios, but show warning
- Highlight in card footer: "Last updated: [date]"
- Provide quick-sync button adjacent to metric card

---

## 8. API Contract

### 8.1 Command: `set_debt_terms`

**Purpose:** User sets or updates APR and minimum payment for a debt account.

```typescript
// Rust signature (approx; finalize in spec #2)
#[tauri::command]
pub fn set_debt_terms(
  account_id: String,
  apr: f64,           // 0.18 for 18%
  minimum_payment: Option<f64>,  // None → use default (2% of balance)
) -> Result<DebtAccount, AppError>;
```

**Validation:**
- `apr`: 0.0 ≤ apr ≤ 0.35; reject if outside (AppError::InvalidAPR)
- `minimum_payment`: if Some, must be > 0; reject if ≤ 0 (AppError::InvalidPayment)
- `account_id`: must exist in `debt_accounts`; reject if not (AppError::NotFound)

**Side effects:**
- Update `debt_accounts.apr` and `debt_accounts.minimum_payment`
- Update `debt_accounts.updated_at` to now
- Invalidate any cached scenario results

### 8.2 Command: `get_opportunity_scenarios`

**Purpose:** Fetch pre-calculated payoff scenarios for a debt or portfolio.

```typescript
#[tauri::command]
pub fn get_opportunity_scenarios(
  account_id: Option<String>,  // None → aggregate all debt accounts
  custom_amount: Option<f64>,  // e.g., $750/month; None → use defaults
) -> Result<Vec<Scenario>, AppError>;

struct Scenario {
  scenario_type: ScenarioType,  // "standard_200" | "standard_500" | "custom"
  acceleration_amount: f64,     // dollars/month
  
  // Per-account detail
  accounts: Vec<ScenarioAccount>,
  
  // Portfolio aggregates
  portfolio_total_balance: f64,
  portfolio_months_saved: f64,
  portfolio_interest_saved: f64,
  portfolio_weighted_apr: f64,
  
  // Human-readable summary
  summary_line: String,  // e.g., "Pay off $8k debt 4.7 years faster, save $1,951"
}

struct ScenarioAccount {
  account_id: String,
  account_name: String,
  apr: f64,
  current_balance: f64,
  months_to_payoff_baseline: u32,
  months_to_payoff_accelerated: u32,
  months_saved: u32,
  interest_saved_dollars: f64,
  interest_saved_percent: f64,
}
```

**Behavior:**
- If `custom_amount` is None: return 2 scenarios ($200 and $500)
- If `custom_amount` is Some: return 1 scenario (the custom amount)
- If account_id is None: aggregate all; if Some: single account only
- Return in priority order: lowest acceleration first

---

## 9. Testing & Validation

### 9.1 Unit Test Cases

1. **Baseline payoff (no acceleration):** Compare calculated `n` against loan amortization tables
2. **Acceleration scenario:** Verify `months_saved` is positive; `interest_saved` is positive and < baseline_interest
3. **Edge cases:**
   - APR = 0: n = B/M (no-interest scenario)
   - Unsustainable payment: M < r_m * B (error/alert)
   - Single-month payoff: M ≥ B (n = 1)
4. **Aggregation:** Sum of parts = total (no floating-point divergence > $0.01)
5. **Rounding:** Ceiling(n) for months; cents precision for dollars
6. **Data staleness:** Flag scenarios > 24h old; include last_sync in result

### 9.2 Integration Tests

- Fetch debt accounts from DB, call `get_opportunity_scenarios`, verify result shape
- Edit APR/payment via `set_debt_terms`, verify updated_at changes and cached results invalidate
- Multi-account scenarios: confirm aggregates match per-account sums

### 9.3 Example Test Vector

```
Input: {
  balance: 5000,
  apr: 0.22,
  minimum_payment: 150,
  acceleration: 200
}

Expected output:
  months_baseline: 52
  months_accelerated: 17
  months_saved: 35
  interest_baseline: 2649
  interest_accelerated: 1153
  interest_saved: 1496
  
Tolerance: ±2 months (rounding), ±$10 interest (iterated precision)
```

---

## 10. Implementation Notes

### 10.1 Code Structure (Rust side)

```rust
// src/models.rs
pub struct DebtAccount {
  pub id: String,
  pub account_id: String,
  pub name: String,
  pub apr: f64,
  pub minimum_payment: f64,
  pub updated_at: DateTime<Utc>,
}

pub struct Scenario {
  pub scenario_type: ScenarioType,
  pub accounts: Vec<ScenarioAccount>,
  pub portfolio_metrics: PortfolioMetrics,
  pub summary_line: String,
}

// src/amortization.rs
pub fn calculate_months_to_payoff(balance: f64, apr: f64, monthly_payment: f64) -> Result<f64, AmortError> {
  // ... formula
}

pub fn calculate_total_interest(balance: f64, apr: f64, monthly_payment: f64, months: f64) -> Result<f64, AmortError> {
  // ... iteration
}

pub fn generate_scenarios(
  debts: Vec<DebtAccount>,
  accelerations: Vec<f64>,
) -> Result<Vec<Scenario>, AppError> {
  // ... orchestration
}
```

### 10.2 Frontend Integration (TypeScript)

```typescript
// src/lib/calculations.ts
export interface ScenarioOutput {
  monthsSaved: number;
  interestSaved: number;
  interestSavedPercent: number;
  baselineMonths: number;
  acceleratedMonths: number;
}

export function formatScenarioSummary(scenario: ScenarioOutput): string {
  // e.g., "Save $1,951 in interest and 4.7 years"
}

// In OpportunityCostCard.tsx:
const { data: scenarios } = useQuery(
  ['opportunity_scenarios', selectedCustomAmount],
  () => invoke('get_opportunity_scenarios', { custom_amount: selectedCustomAmount })
);
```

### 10.3 Caching Strategy

- Cache scenarios for max 24 hours after calculation
- Invalidate immediately on `set_debt_terms` call (APR/payment change)
- Invalidate on new SimpleFIN sync (balance change)
- Store cache in-memory (not DB) with TTL

---

## 11. Future Extensions

These are out of scope for MVP but should be designed with forward compatibility:

1. **Payoff strategy** (debt snowball vs. avalanche): order accounts by APR vs. balance; recalculate with strategy-aware allocation
2. **Variable interest rates:** per-month APR array instead of constant
3. **Lump-sum payments:** one-time extra payment (e.g., tax refund) and its payoff impact
4. **Recurring transfers:** model linked transfer accounts (e.g., savings → debt)
5. **Credit utilization impact:** hint that paying down high-utilization cards improves credit score
6. **Multi-currency:** support debt in USD, EUR, etc. with FX rates

---

## Summary

This spec formalizes the mathematical foundation of the Momentum app's opportunity-cost engine. By nailing the amortization formula, scenario aggregation, and data flow, we unblock parallel frontend+backend work and ensure consistent calculations across UI and backend.

**Key takeaway:** Debt payoff scenarios are driven by four inputs (balance, APR, payment, acceleration), produce three key outputs (months saved, interest saved, payoff date), and scale cleanly to multi-account portfolios via summation.
