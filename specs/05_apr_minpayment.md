# APR and Minimum Payment Data Specification

**Document ID**: `05_apr_minpayment.md`  
**Status**: Active  
**Last Updated**: 2026-07-06  
**Related**: `01_accounts_schema.md`, IMPLEMENTATION_PLAN Sprint 0 Debt Terms, Opportunity Cost Metrics

---

## Overview

APR (Annual Percentage Rate) and Minimum Payment are user-provided financial parameters critical to debt analysis and opportunity cost calculations. Unlike SimpleFIN-provided data (account names, balances, transactions), these fields are **user inputs** stored persistently in the `debt_accounts` table and edited via the Settings UI.

**Purpose**: Enable accurate interest calculations, debt payoff projections, and opportunity cost comparisons for credit cards and loans.

---

## Design Rationale

### Problem Statement
SimpleFIN does not provide:
- Annual Percentage Rate (APR) for credit cards or loans
- Minimum payment amounts

Without these inputs, the app cannot:
1. Calculate interest accrual over time
2. Project payoff timelines
3. Compare opportunity costs (e.g., "paying $100/month toward this CC vs. investing it")
4. Warn users about high-interest debt

### Solution: User-Maintained debt_accounts Extension

The `debt_accounts` table (sparse, user-maintained) extends `accounts` (SimpleFIN-canonical) with interest and payment data:
- One-to-one relationship with debt accounts (checking/savings accounts don't have APR/minimum_payment).
- User edits in Settings UI; no SimpleFIN re-sync needed.
- Supports lazy defaults (e.g., calculate minimum as 2% of balance if not provided).
- Invalidates cached metrics when updated, triggering recalculation.

---

## Data Storage

### debt_accounts Table Schema

```sql
CREATE TABLE debt_accounts (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL UNIQUE,     -- FK -> accounts.id (credit_card or loan only)
  apr_decimal REAL NOT NULL,            -- APR as decimal (0.2199 = 21.99%), always required
  minimum_payment_cents INTEGER,        -- Minimum payment in cents (stored as integer for precision)
  minimum_payment_cents_calculated BOOLEAN DEFAULT 0, -- TRUE if auto-calculated as 2% of balance
  source TEXT,                          -- "user_input" (future: "simplefin" if available)
  last_edited TEXT NOT NULL,            -- ISO 8601 UTC timestamp
  
  FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

CREATE INDEX idx_debt_accounts_account_id ON debt_accounts(account_id);
```

**Note on `minimum_payment_cents`**: Stored as INTEGER (cents) to avoid floating-point rounding errors in financial calculations. Conversion to dollars happens at display and input time.

### Column Definitions

| Column | Type | Nullable | Default | Description |
|--------|------|----------|---------|-------------|
| `id` | TEXT | No | — | **Primary Key**. UUIDv4 (e.g., `uuid()`). |
| `account_id` | TEXT | No | — | **Unique FK**. References `accounts.id`. Only debt accounts (credit_card, loan) can have debt_accounts rows. Database constraint enforces this in application layer. |
| `apr_decimal` | REAL | No | — | **Required**. APR as decimal fraction (e.g., 0.2199 for 21.99%). Must be >= 0 and <= 1.0 (validation catches 0-100 input and converts). |
| `minimum_payment_cents` | INTEGER | No | — | **Required**. Minimum monthly payment in cents (e.g., 5000 = $50.00). Must be >= 0. Can be 0 if user explicitly sets "no minimum" (rare). |
| `minimum_payment_cents_calculated` | BOOLEAN | No | 0 | **Tracking flag**. TRUE if minimum_payment was auto-calculated as 2% of current balance (and user has not overridden it). Used to recalculate on balance change. |
| `source` | TEXT | No | "user_input" | **Audit trail**. "user_input" for manual entry; reserved for "simplefin" if SimpleFIN API ever provides APR. |
| `last_edited` | TEXT | No | — | **ISO 8601 UTC**. Timestamp of user's last edit in Settings UI. Enables sorting "recently edited debt accounts" in dashboard. |

---

## Rust Type Definitions

```rust
// src/lib/types.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebtAccount {
    pub id: String,
    pub account_id: String,
    /// APR as decimal (0.2199 = 21.99%)
    pub apr_decimal: f64,
    /// Minimum payment in cents (e.g., 5000 = $50.00)
    pub minimum_payment_cents: i64,
    /// If true, this minimum was auto-calculated as 2% of balance
    pub minimum_payment_cents_calculated: bool,
    pub source: String, // "user_input" or "simplefin"
    pub last_edited: String, // ISO 8601 UTC
}

impl DebtAccount {
    /// Convert APR decimal to percentage for display (0.2199 -> 21.99)
    pub fn apr_percent(&self) -> f64 {
        self.apr_decimal * 100.0
    }

    /// Convert APR percentage (0-100) to decimal for storage
    pub fn apr_from_percent(percent: f64) -> f64 {
        percent / 100.0
    }

    /// Minimum payment in dollars (cents / 100)
    pub fn minimum_payment_dollars(&self) -> f64 {
        self.minimum_payment_cents as f64 / 100.0
    }

    /// Create DebtAccount with minimum auto-calculated as 2% of balance
    pub fn with_calculated_minimum(
        id: String,
        account_id: String,
        apr_decimal: f64,
        balance: f64,
    ) -> Self {
        let min_payment_cents = ((balance * 0.02) * 100.0).round() as i64;
        Self {
            id,
            account_id,
            apr_decimal,
            minimum_payment_cents: min_payment_cents,
            minimum_payment_cents_calculated: true,
            source: "user_input".to_string(),
            last_edited: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Command: Set or update APR and minimum payment for a debt account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDebtTermsRequest {
    pub account_id: String,
    /// APR as percentage (0-100), converted to decimal on the backend
    pub apr_percent: f64,
    /// Minimum payment in dollars, converted to cents on the backend (or None to auto-calculate)
    pub minimum_payment_dollars: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDebtTermsResponse {
    pub success: bool,
    pub debt_account: Option<DebtAccount>,
    pub error: Option<String>,
}
```

---

## TypeScript / Frontend Type Definitions

```typescript
// src/lib/types.ts

export interface DebtAccount {
  id: string;
  account_id: string;
  apr_decimal: number; // 0.2199 = 21.99%
  minimum_payment_cents: number; // 5000 = $50.00
  minimum_payment_cents_calculated: boolean;
  source: "user_input" | "simplefin";
  last_edited: string; // ISO 8601
}

export interface SetDebtTermsRequest {
  account_id: string;
  apr_percent: number; // 0-100, converted to decimal on backend
  minimum_payment_dollars?: number; // Optional; if not provided, auto-calc as 2%
}

export interface SetDebtTermsResponse {
  success: boolean;
  debt_account?: DebtAccount;
  error?: string;
}

// Helper: Format APR for display
export const formatApr = (apr_decimal: number): string => {
  return `${(apr_decimal * 100).toFixed(2)}%`;
};

// Helper: Format minimum payment
export const formatMinimumPayment = (cents: number): string => {
  return `$${(cents / 100).toFixed(2)}`;
};

// Helper: Calculate minimum payment as 2% of balance
export const calculateMinimumPaymentCents = (balanceDollars: number): number => {
  return Math.round(balanceDollars * 0.02 * 100);
};
```

---

## User Interface: Settings > Debt Terms

### Layout

```
Settings > Debt Terms

[List of debt accounts with collapsible per-account form]

For each account:
┌─────────────────────────────────────────────────┐
│ Account: Chase Sapphire CC (read-only)          │
│ Current Balance: $3,245.67 (read-only, synced)  │
│                                                  │
│ APR (%):              [21.99________]           │
│ Minimum Payment ($):  [$50.00_______]           │
│                       [✓ Auto-calculate 2%]    │
│                                                  │
│ [Save] [Clear Terms]                            │
└─────────────────────────────────────────────────┘
```

### Form Behavior

1. **Account Name & Balance**: Read-only, populated from `accounts` table (synced from SimpleFIN).
2. **APR Input**:
   - Type: `<input type="number" min="0" max="100" step="0.01" />`
   - Placeholder: "e.g., 21.99"
   - Validation: 0 ≤ APR ≤ 100 (enforced on save)
   - Display format: 2 decimal places (21.99%)
   - Required: Yes (cannot save without APR)

3. **Minimum Payment Input**:
   - Type: `<input type="number" min="0" step="0.01" />`
   - Placeholder: "e.g., 50.00"
   - Validation: >= 0 (enforced on save)
   - Display format: Currency ($50.00)
   - Required: No (can be empty if user opts for auto-calc)
   - Checkbox: **"Auto-calculate as 2% of balance"**
     - If checked: input field disabled; backend calculates on save.
     - If unchecked: user must provide explicit amount.

4. **Save Button**:
   - Validates APR and minimum payment.
   - Calls backend `set_debt_terms()` command.
   - Shows success/error toast.
   - Refreshes metrics cache.

5. **Clear Terms Button**:
   - Deletes the `debt_accounts` row (soft-delete or hard delete—decision needed).
   - Clears form fields.
   - Triggers metrics recalculation.

### Example Form JSON Payload (to Backend)

```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440002",
  "apr_percent": 21.99,
  "minimum_payment_dollars": 50.00
}
```

OR (with auto-calculation):

```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440002",
  "apr_percent": 21.99,
  "minimum_payment_dollars": null
}
```

---

## Defaults

### APR Default
- **No default**. User MUST provide APR for each debt account.
- If APR is missing, debt account is treated as "incomplete" and excluded from metrics calculations.
- UI shows a warning: "APR required to show interest calculations."

### Minimum Payment Default
- **If not provided**: Auto-calculated as **2% of current balance** (rounded to nearest cent).
- **If balance is 0**: Minimum payment defaults to $0.
- User can override auto-calculated value at any time.
- Recalculated on SimpleFIN sync (if balance changes significantly), unless user has set an explicit override.

**Example**:
- Account: Chase Sapphire CC
- Current Balance: $3,245.67
- User provides APR: 21.99%
- User leaves Minimum Payment blank, selects "Auto-calculate"
- Backend calculates: $3,245.67 × 2% = $64.91

---

## Validation Rules

### APR Validation

| Rule | Check | Error Message |
|------|-------|---------------|
| Presence | APR is required | "APR is required" |
| Range | 0 ≤ APR ≤ 100 | "APR must be between 0 and 100" |
| Numeric | APR is a valid number | "APR must be a valid number" |
| Precision | At most 4 decimal places | "APR precision limited to 4 decimals" (e.g., 21.9999%) |

### Minimum Payment Validation

| Rule | Check | Error Message |
|------|-------|---------------|
| Range | Minimum payment ≥ 0 | "Minimum payment must be $0 or greater" |
| Numeric | Valid number | "Minimum payment must be a valid number" |
| Precision | At most 2 decimal places (cents) | "Minimum payment precision limited to cents" |

### Account-Level Validation

| Rule | Check | Error Message |
|------|-------|---------------|
| Account exists | FK account_id exists in `accounts` | "Account not found" |
| Account is debt | Account type is credit_card or loan | "Only credit cards and loans can have debt terms" |
| Uniqueness | At most one `debt_accounts` row per `account_id` | (DB constraint; app prevents duplicate inserts) |

### Metrics-Level Validation

| Rule | Check | Consequence |
|------|-------|-------------|
| At least one APR set | At least one debt account has valid APR | Opportunity cost scenarios do not show if no APR; dashboard displays "No debt terms configured" warning |
| APR for payoff calc | If calculating payoff timeline, APR must be present | Skip that account from payoff projections; show as "N/A – APR not set" |

---

## Update Flow

### Command: set_debt_terms

**Tauri Command Signature** (Rust):

```rust
#[tauri::command]
pub async fn set_debt_terms(
    account_id: String,
    apr_percent: f64,
    minimum_payment_dollars: Option<f64>,
) -> Result<SetDebtTermsResponse, String> {
    // 1. Validate inputs
    if apr_percent < 0.0 || apr_percent > 100.0 {
        return Err("APR must be between 0 and 100".to_string());
    }
    if let Some(min_pmt) = minimum_payment_dollars {
        if min_pmt < 0.0 {
            return Err("Minimum payment must be >= 0".to_string());
        }
    }

    // 2. Verify account exists and is a debt account
    let account = db.get_account(&account_id)?;
    if !matches!(account.account_type, AccountType::CreditCard | AccountType::Loan) {
        return Err("Only credit cards and loans can have debt terms".to_string());
    }

    // 3. Convert inputs
    let apr_decimal = apr_percent / 100.0;
    let min_pmt_cents = if let Some(dollars) = minimum_payment_dollars {
        (dollars * 100.0).round() as i64
    } else {
        // Auto-calculate 2% of balance
        let balance = account.balance.unwrap_or(0.0);
        ((balance * 0.02) * 100.0).round() as i64
    };

    // 4. Upsert debt_accounts
    let debt_account = DebtAccount {
        id: generate_id(),
        account_id: account_id.clone(),
        apr_decimal,
        minimum_payment_cents: min_pmt_cents,
        minimum_payment_cents_calculated: minimum_payment_dollars.is_none(),
        source: "user_input".to_string(),
        last_edited: chrono::Utc::now().to_rfc3339(),
    };

    db.upsert_debt_account(&debt_account)?;

    // 5. Invalidate metrics cache
    cache.invalidate("debt_metrics");
    cache.invalidate("opportunity_cost");

    Ok(SetDebtTermsResponse {
        success: true,
        debt_account: Some(debt_account),
        error: None,
    })
}
```

**Error Cases**:

| Error | Cause | HTTP Status | Response |
|-------|-------|-------------|----------|
| Account not found | `account_id` doesn't exist | 404 | `{ success: false, error: "Account not found" }` |
| Invalid APR | APR < 0 or > 100 | 400 | `{ success: false, error: "APR must be between 0 and 100" }` |
| Invalid minimum payment | Minimum < 0 | 400 | `{ success: false, error: "Minimum payment must be >= 0" }` |
| Account is not debt | Account type is checking/savings | 400 | `{ success: false, error: "Only credit cards and loans can have debt terms" }` |
| Database error | SQL constraint violation, etc. | 500 | `{ success: false, error: "Failed to save debt terms" }` |

### Database Upsert Logic

```rust
// Pseudocode
pub fn upsert_debt_account(&self, debt_account: &DebtAccount) -> Result<(), String> {
    db.execute(
        "INSERT INTO debt_accounts 
         (id, account_id, apr_decimal, minimum_payment_cents, minimum_payment_cents_calculated, source, last_edited)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(account_id) DO UPDATE SET
           apr_decimal = excluded.apr_decimal,
           minimum_payment_cents = excluded.minimum_payment_cents,
           minimum_payment_cents_calculated = excluded.minimum_payment_cents_calculated,
           last_edited = excluded.last_edited",
        params![
            debt_account.id,
            debt_account.account_id,
            debt_account.apr_decimal,
            debt_account.minimum_payment_cents,
            debt_account.minimum_payment_cents_calculated,
            debt_account.source,
            debt_account.last_edited,
        ],
    )?;
    Ok(())
}
```

### Cache Invalidation

When `set_debt_terms()` succeeds:
1. Invalidate `debt_metrics` cache (interest calculations, payoff timelines).
2. Invalidate `opportunity_cost` cache (comparison scenarios).
3. Dashboard re-fetches metrics on next render.

---

## Display

### APR Display

- **Format**: 2 decimal places as percentage (e.g., 21.99%)
- **In Settings**: `[21.99%]` (read-only in list view; editable in form)
- **In Dashboard metrics**: "21.99% APR"
- **In calculations**: Use decimal form (0.2199)

### Minimum Payment Display

- **Format**: Currency with 2 decimal places (e.g., $50.00)
- **In Settings**: `[$50.00]` (read-only in list view; editable in form)
- **In Dashboard**: "Minimum: $50.00/month"
- **In calculations**: Use cents form (5000) for precision

### Accounts Without Debt Terms

If a debt account has no `debt_accounts` row (incomplete setup):

- **In Settings**: Display form with empty fields and warning "APR required to show interest calculations"
- **In Dashboard**: Show as "N/A – terms not set" instead of metrics
- **Example**: 
  ```
  Account: Tesla Auto Loan
  Balance: $28,500.00
  Metrics: N/A – terms not set
  [Edit debt terms in Settings]
  ```

---

## Query Examples

### Get Debt Terms for Account

```sql
SELECT 
  a.id, a.name, a.account_type, a.balance,
  da.apr_decimal, da.minimum_payment_cents, da.minimum_payment_cents_calculated, da.last_edited
FROM accounts a
LEFT JOIN debt_accounts da ON a.id = da.account_id
WHERE a.id = '550e8400-e29b-41d4-a716-446655440002';
```

**Result**:
```
id | name | account_type | balance | apr_decimal | minimum_payment_cents | minimum_payment_cents_calculated | last_edited
---|------|--------------|---------|-------------|------------------------|----------------------------------|-----------
550e8400-e29b-41d4-a716-446655440002 | Chase Sapphire CC | credit_card | 3245.67 | 0.2199 | 5000 | 0 | 2026-07-06T14:30:00Z
```

### All Debt Accounts with Terms

```sql
SELECT 
  a.id, a.name, a.balance, 
  da.apr_decimal, da.minimum_payment_cents
FROM accounts a
JOIN debt_accounts da ON a.id = da.account_id
WHERE a.account_type IN ('credit_card', 'loan')
ORDER BY da.last_edited DESC;
```

### Debt Accounts Missing APR (Incomplete)

```sql
SELECT a.id, a.name, a.account_type, a.balance
FROM accounts a
LEFT JOIN debt_accounts da ON a.id = da.account_id
WHERE a.account_type IN ('credit_card', 'loan')
  AND da.apr_decimal IS NULL;
```

### Interest Accrual Calculation

For a single month, using daily interest:

```
Daily Interest Rate = APR / 365
Daily Interest = Balance × Daily Rate
Monthly Interest ≈ Sum of daily interest over month
```

**SQL Example** (for past 30 days on Chase Sapphire):

```sql
WITH daily_rates AS (
  SELECT 
    a.id, a.name, a.balance,
    da.apr_decimal,
    (da.apr_decimal / 365.0) AS daily_rate
  FROM accounts a
  JOIN debt_accounts da ON a.id = da.account_id
  WHERE a.id = '550e8400-e29b-41d4-a716-446655440002'
)
SELECT 
  (SELECT balance FROM daily_rates) * 
  (SELECT daily_rate FROM daily_rates) * 30.0 AS estimated_monthly_interest;
```

---

## Example Data

```sql
INSERT INTO debt_accounts VALUES
  ('da-001', '550e8400-e29b-41d4-a716-446655440002', 0.2199, 5000, 0, 'user_input', '2026-07-06T14:30:00Z'),  -- Chase Sapphire, 21.99%, $50/month (explicit)
  ('da-002', '550e8400-e29b-41d4-a716-446655440003', 0.0699, 28500, 1, 'user_input', '2026-07-05T10:00:00Z');   -- Tesla Auto Loan, 6.99%, auto-calc 2% of $1.425M balance = $28,500
```

---

## Error Cases & Handling

### Case 1: Account Not Found

**Request**:
```json
{
  "account_id": "nonexistent",
  "apr_percent": 21.99,
  "minimum_payment_dollars": 50.00
}
```

**Response**:
```json
{
  "success": false,
  "error": "Account not found"
}
```

**UI Behavior**: Show error toast. Remain on form. User checks account list in Settings.

---

### Case 2: Invalid APR (Out of Range)

**Request**:
```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440002",
  "apr_percent": 150.0,
  "minimum_payment_dollars": 50.00
}
```

**Response**:
```json
{
  "success": false,
  "error": "APR must be between 0 and 100"
}
```

**UI Behavior**: Show inline error under APR field. Disable Save button.

---

### Case 3: Invalid Minimum Payment

**Request**:
```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440002",
  "apr_percent": 21.99,
  "minimum_payment_dollars": -10.00
}
```

**Response**:
```json
{
  "success": false,
  "error": "Minimum payment must be >= 0"
}
```

**UI Behavior**: Show inline error under Minimum Payment field. Disable Save button.

---

### Case 4: Non-Debt Account (Checking)

**Request**:
```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440000",
  "apr_percent": 21.99,
  "minimum_payment_dollars": 50.00
}
```

(Account ID is for "Chase Checking 4567", account_type = "checking")

**Response**:
```json
{
  "success": false,
  "error": "Only credit cards and loans can have debt terms"
}
```

**UI Behavior**: Settings > Debt Terms form only lists credit_card and loan accounts. This shouldn't happen in normal flow.

---

### Case 5: APR Not Set (Dashboard Warning)

**Scenario**: User has configured minimum payment but not APR.

**Dashboard Display**:
```
Opportunity Cost Scenarios
───────────────────────────
[Warning] Unable to calculate opportunity costs:
• Chase Sapphire CC: APR not configured
• Tesla Auto Loan: APR not configured

[Go to Settings > Debt Terms to configure APR]
```

---

## Relationship to Metrics

### Interest Accrual (Phase 2)

Once APR and balance are set, dashboard can show:
- Monthly interest accrued: `balance × (apr / 12)`
- Interest vs. principal in minimum payment
- Time to payoff at current minimum

**Metrics Code** (pseudocode):

```rust
fn calculate_interest_accrual(debt_account: &DebtAccount, account: &Account) -> Result<InterestMetric> {
    if debt_account.apr_decimal == 0.0 {
        return Err("APR must be > 0 for interest calculations".to_string());
    }
    
    let balance = account.balance.ok_or("Balance missing")?;
    let monthly_rate = debt_account.apr_decimal / 12.0;
    let monthly_interest = balance * monthly_rate;
    
    Ok(InterestMetric {
        monthly_interest,
        annual_interest: monthly_interest * 12.0,
        payoff_months_at_minimum: calculate_payoff_timeline(
            balance,
            debt_account.minimum_payment_cents as f64 / 100.0,
            monthly_rate,
        ),
    })
}
```

### Opportunity Cost (Phase 2)

With APR set, dashboard compares:
- Interest cost of carrying debt: `$X per month`
- Potential investment return: `$X per month at Y% yield`
- User decides: "Should I pay off this $3,245 CC debt or invest in ETFs?"

---

## Migration & Rollout

### Phase 1: Add debt_accounts Table
1. Run migration: Create `debt_accounts` table (this spec).
2. No existing data migrated (fresh table).

### Phase 2: Add Settings UI
1. Build Settings > Debt Terms form (React component).
2. Implement `set_debt_terms()` Tauri command.
3. Wire up validation and error handling.

### Phase 3: Dashboard Integration
1. Query `debt_accounts` for metrics calculations.
2. Show "N/A – terms not set" for incomplete accounts.
3. Display warnings if no APR found for opportunity cost scenarios.

### Phase 4: Metrics Calculations
1. Implement interest accrual (Phase 2).
2. Implement payoff projections.
3. Implement opportunity cost comparisons.

---

## Testing Checklist

- [ ] Insert debt account with APR=21.99%, minimum=$50; verify stored as decimal and cents.
- [ ] Update debt account; verify `last_edited` changes and old values overwrite.
- [ ] Upsert same account twice; verify no duplicate rows.
- [ ] Validate APR range: reject -1, 0.5, 100.5; accept 0, 21.99, 100.
- [ ] Validate minimum payment: reject -10, accept 0, 50.25.
- [ ] Auto-calculate minimum as 2% of balance; verify correct cents.
- [ ] Query debt_accounts JOIN accounts; verify all columns accessible.
- [ ] Delete debt account (if soft-delete: set flag; if hard-delete: cascade); verify can re-create.
- [ ] Display APR as 21.99%, minimum as $50.00 in UI.
- [ ] Show "N/A – APR not set" for accounts without debt_accounts row.
- [ ] Verify cache invalidation triggers after `set_debt_terms()`.
- [ ] Error cases: non-existent account, invalid APR, non-debt account type.

---

## Related Specifications

- **01_accounts_schema.md**: Schema for `accounts` table; debt accounts are FK references.
- **IMPLEMENTATION_PLAN**: Sprint 0, Debt Terms initiative.
- **02_tauri_commands.md** (forthcoming): Command signatures including `set_debt_terms()`.
- **04_metrics_calculations.md** (forthcoming): Interest accrual, payoff projections, opportunity cost.
- **UI_DESIGN.md** (forthcoming): Settings > Debt Terms form mockups.

