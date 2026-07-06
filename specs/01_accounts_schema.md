# Accounts Table Schema Specification

**Document ID**: `01_accounts_schema.md`  
**Status**: Active  
**Last Updated**: 2026-07-06  
**Related**: IMPLEMENTATION_PLAN Sprint 0 Gap #1, #2

---

## Overview

The `accounts` table is a centralized, type-agnostic container for all financial accounts linked to SimpleFIN. It replaces the previous single-purpose `debt_accounts` table with a more general schema that supports checking, savings, credit cards, and loans in a unified structure.

**Purpose**: Enable dashboard metrics (income, spending, debt paydown) to correctly attribute transactions and balances to account types, and provide a stable record of account-level metadata from SimpleFIN.

---

## Design Rationale

### Problem Statement
The existing `debt_accounts` table could only store credit cards and loans, leaving checking and savings accounts unmapped. This created:
1. Ambiguity in income attribution (which deposit accounts count as "income"?)
2. Inability to calculate debt ratio or interest metrics correctly
3. No way to track account metadata (organization/issuer, last sync timestamp) for non-debt accounts

### Solution: Unified Accounts Table
A single `accounts` table with an `account_type` enum serves all account categories:
- **Checking & Savings**: Income/spending source tracking
- **Credit Cards & Loans**: Debt balance and payoff calculations
- **Future expansion**: Investment accounts, HSA, etc.

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Separate from debt_accounts** | The `accounts` table is canonical for account metadata. The `debt_accounts` table (if retained) becomes a sparse, user-edited extension for APR + minimum_payment, with FK -> `accounts.id`. |
| **simplefin_account_id as secondary key** | SimpleFIN provides this as a stable identifier per account. Used for upsert logic during sync. |
| **organization field** | SimpleFIN returns issuer/bank name (e.g., "Chase", "Bank of America"). Useful for account grouping and debugging. |
| **balance as REAL** | Matches SimpleFIN's decimal precision. Null-safe for accounts that don't report balance. |
| **last_updated as ISO 8601 TEXT** | Matches transaction timestamps. Timezone-aware (always UTC). Tracks freshness of SimpleFIN data. |
| **account_type as ENUM** | Enables efficient filtering (e.g., "sum balances where type in ('checking', 'savings')") and clear intent. |

---

## Schema Definition

### SQL Table

```sql
CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  simplefin_account_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  account_type TEXT NOT NULL,  -- enum: checking, savings, credit_card, loan
  organization TEXT,           -- e.g., "Chase", "Bank of America", nullable for testing
  balance REAL,                -- Current balance in account currency; nullable if not available
  last_updated TEXT NOT NULL   -- ISO 8601 timestamp, UTC
);

CREATE INDEX idx_accounts_simplefin_id ON accounts(simplefin_account_id);
CREATE INDEX idx_accounts_type ON accounts(account_type);
```

### Column Definitions

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| `id` | TEXT | No | **Primary Key**. UUIDv4 or equivalent (e.g., `uuid()` in SQLite via extension or pre-generated). Acts as foreign key for `raw_transactions.account_id` and reference in dashboard logic. |
| `simplefin_account_id` | TEXT | No | **Unique**. SimpleFIN's internal account ID (stable per account for the lifetime of that login). Used for upsert on sync: if `simplefin_account_id` already exists, update balance/last_updated; else insert new row. |
| `name` | TEXT | No | **Required**. User-visible account name from SimpleFIN (e.g., "Chase Business Checking 1234"). May be edited by user in future; preserve original SimpleFIN name in raw data. |
| `account_type` | TEXT | No | **Required**. Enum: `checking`, `savings`, `credit_card`, `loan`. Controls which metrics the account feeds (e.g., income sums only from checking/savings; debt sums only from credit_card/loan). |
| `organization` | TEXT | Yes | **Optional**. SimpleFIN-provided issuer/bank name (e.g., "Chase", "BoA", "Discover"). Null if not returned by SimpleFIN. Useful for account grouping in UI and debugging sync issues. |
| `balance` | REAL | Yes | **Optional**. Current balance in account's native currency (assumed USD for MVP). SimpleFIN provides this. Null if the API doesn't return it (rare) or account is closed. **Sign convention**: positive for checking/savings/loans (owed to you/by you); negative for credit cards (amount owed). **Note**: Use absolute value in debt calculations; track sign semantics in app logic. |
| `last_updated` | TEXT | No | **Required**. ISO 8601 timestamp (UTC) of the last successful SimpleFIN sync for this account. Format: `2026-07-06T14:30:00Z`. Enables dashboard to surface "stale data" warnings if sync fails for >24h. |

---

## Account Type Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    #[serde(rename = "checking")]
    Checking,
    #[serde(rename = "savings")]
    Savings,
    #[serde(rename = "credit_card")]
    CreditCard,
    #[serde(rename = "loan")]
    Loan,
}
```

### Enum Value Semantics

| Value | Meaning | Contributes To | Notes |
|-------|---------|-----------------|-------|
| `checking` | Deposit account, primarily for living expenses | Income, Spending | Default for direct deposits; usual transaction source. |
| `savings` | Savings/money-market deposit account | Income (for balance calc), sometimes excluded from spending | Treated like checking for momentum metrics (deposits count as income). |
| `credit_card` | Revolving credit (Visa, Mastercard, AMEX, etc.) | Debt (balance + paydown + interest) | Balance is amount owed (positive number, user sees as debt). Transactions posted to this account are "charges" (negative for spending). |
| `loan` | Installment debt (auto loan, personal loan, etc.) | Debt (balance + paydown + interest) | Balance is principal outstanding. Transactions are payments (principal + interest mixed; LLM categorizes interest separately). |

---

## Rust Type Definition

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub id: String,
    pub simplefin_account_id: String,
    pub name: String,
    pub account_type: AccountType,
    pub organization: Option<String>,
    pub balance: Option<f64>,
    pub last_updated: String, // ISO 8601
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    #[serde(rename = "checking")]
    Checking,
    #[serde(rename = "savings")]
    Savings,
    #[serde(rename = "credit_card")]
    CreditCard,
    #[serde(rename = "loan")]
    Loan,
}

impl Account {
    /// Convenience method: is this a debt account?
    pub fn is_debt(&self) -> bool {
        matches!(self.account_type, AccountType::CreditCard | AccountType::Loan)
    }

    /// Convenience method: is this an income/deposit account?
    pub fn is_deposit(&self) -> bool {
        matches!(self.account_type, AccountType::Checking | AccountType::Savings)
    }
}
```

---

## TypeScript / Frontend Type Definition

```typescript
// src/lib/types.ts

export type AccountType = "checking" | "savings" | "credit_card" | "loan";

export interface Account {
  id: string;
  simplefin_account_id: string;
  name: string;
  account_type: AccountType;
  organization?: string;
  balance?: number;
  last_updated: string; // ISO 8601
}

// Helpers
export const isDebtAccount = (account: Account): boolean =>
  account.account_type === "credit_card" || account.account_type === "loan";

export const isDepositAccount = (account: Account): boolean =>
  account.account_type === "checking" || account.account_type === "savings";
```

---

## Upsert Logic (SimpleFIN Sync)

When syncing accounts from SimpleFIN:

```rust
// Pseudocode
for sf_account in simplefin_accounts {
    let account_id = generate_or_reuse_id(sf_account.simplefin_id);
    
    db.execute(
        "INSERT INTO accounts 
         (id, simplefin_account_id, name, account_type, organization, balance, last_updated)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(simplefin_account_id) DO UPDATE SET
           name = excluded.name,
           balance = excluded.balance,
           last_updated = excluded.last_updated",
        params![
            account_id,
            sf_account.id,
            sf_account.name,
            map_sf_type_to_enum(sf_account.type), // e.g., "credit card" -> "credit_card"
            sf_account.institution,
            sf_account.balance,
            now_iso8601(),
        ],
    )?;
}
```

**Key Points**:
- `simplefin_account_id` is the unique key for upsert.
- `id` is generated on first insert (UUIDv4) and reused thereafter.
- `balance` and `last_updated` are overwritten on each sync.
- `name` may change if user renames in SimpleFIN; always sync the current name.

---

## Relationship to debt_accounts

### Option A: Keep debt_accounts as sparse extension (RECOMMENDED)

The `debt_accounts` table becomes a user-maintained lookup for interest-rate and minimum-payment overrides:

```sql
CREATE TABLE debt_accounts (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL UNIQUE,  -- FK -> accounts.id
  interest_rate REAL,                -- APR as decimal (e.g., 0.2199 = 21.99%)
  minimum_payment REAL,              -- Static minimum or auto-calculated
  source TEXT,                       -- "user_input", "simplefin" (if ever available)
  last_edited TEXT,                  -- ISO 8601, tracks when user last changed
  
  FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);
```

**Benefits**:
- Metrics code queries `accounts` for balance; joins `debt_accounts` for APR if needed.
- Users edit APR/minimum in Settings UI, which writes to `debt_accounts`.
- No duplication; `accounts` is canonical.

### Option B: Merge into accounts (FUTURE)

If APR/minimum_payment become critical per-account fields, absorb them into `accounts`:

```sql
ALTER TABLE accounts ADD COLUMN interest_rate REAL;      -- NULL for non-debt
ALTER TABLE accounts ADD COLUMN minimum_payment REAL;    -- NULL for non-debt
```

This is simpler for queries but increases schema coupling. **Deferred to Phase 2.**

---

## Validation Rules

### At Insert/Update

1. **id**: Must be non-empty UUID or generated.
2. **simplefin_account_id**: Must be non-empty and globally unique.
3. **name**: Must be non-empty and ≤255 characters.
4. **account_type**: Must be one of the four enum values (validate in app before insert).
5. **organization**: Optional; if provided, ≤100 characters.
6. **balance**: Must be a valid number (may be negative for credit cards).
7. **last_updated**: Must be valid ISO 8601 UTC timestamp.

### Application Level

- **No duplicate SimpleFIN IDs**: Handled by UNIQUE constraint; application must catch constraint violation and update instead.
- **Type-aware balance semantics**: App must document and enforce sign conventions (positive for deposits/loans, "positive" for CC debt owed).
- **Stale data detection**: Dashboard queries `max(last_updated)` and warns if > 24h old.

---

## Example Data

```sql
INSERT INTO accounts VALUES 
  ('550e8400-e29b-41d4-a716-446655440000', 'sf_001', 'Chase Checking 4567', 'checking', 'Chase', 5234.50, '2026-07-06T14:30:00Z'),
  ('550e8400-e29b-41d4-a716-446655440001', 'sf_002', 'BoA Savings', 'savings', 'Bank of America', 12500.00, '2026-07-06T14:30:00Z'),
  ('550e8400-e29b-41d4-a716-446655440002', 'sf_003', 'Chase Sapphire CC', 'credit_card', 'Chase', 3245.67, '2026-07-06T14:30:00Z'),
  ('550e8400-e29b-41d4-a716-446655440003', 'sf_004', 'Tesla Auto Loan', 'loan', 'Tesla Financial', 28500.00, '2026-07-06T14:30:00Z');
```

---

## Query Examples

### Income Calculation (Last 30 Days)

```sql
SELECT SUM(rt.amount) AS total_income
FROM raw_transactions rt
JOIN accounts a ON rt.account_id = a.id
WHERE a.account_type IN ('checking', 'savings')
  AND rt.amount > 0
  AND date(rt.posted_date) >= date('now', '-30 days')
  AND rt.category NOT IN ('transfers');
```

### Debt Snapshot

```sql
SELECT 
  a.name,
  a.account_type,
  a.balance,
  COALESCE(da.interest_rate, 0) AS apr
FROM accounts a
LEFT JOIN debt_accounts da ON a.id = da.account_id
WHERE a.account_type IN ('credit_card', 'loan');
```

### Stale Data Warning

```sql
SELECT COUNT(*) AS stale_accounts
FROM accounts
WHERE datetime(last_updated) < datetime('now', '-24 hours');
```

---

## Migration from debt_accounts

### Phase 1: Create New Schema
1. Create `accounts` table (this spec).
2. Populate from SimpleFIN on first sync.

### Phase 2: Backfill Existing Debt Accounts
```sql
INSERT INTO accounts 
SELECT 
  id,
  simplefin_account_id,
  account_name AS name,
  LOWER(account_type) AS account_type,  -- Assume "credit_card" format
  NULL AS organization,
  current_balance AS balance,
  last_updated
FROM debt_accounts
WHERE simplefin_account_id NOT IN (SELECT simplefin_account_id FROM accounts);
```

### Phase 3: Deprecate debt_accounts
- Keep `debt_accounts` for user-provided APR/minimum_payment (sparse FK relation).
- Remove non-APR columns; update queries to use `accounts` as primary.
- Document in DEVELOPMENT.md.

---

## Testing Checklist

- [ ] Insert four account types; verify enum values stored correctly.
- [ ] Upsert same `simplefin_account_id` twice; verify balance overwrites (not duplicate).
- [ ] Query `is_debt()` and `is_deposit()` helpers return correct results per type.
- [ ] Null `organization` and `balance` handled gracefully in queries.
- [ ] `last_updated` reflects sync timestamp; can filter for stale data.
- [ ] Foreign key constraint from `raw_transactions.account_id` enforced.
- [ ] Serialization to JSON (Tauri command response) preserves all fields.

---

## Future Extensions

| Extension | Rationale | Phase |
|-----------|-----------|-------|
| `currency` field | Support multi-currency accounts (e.g., EUR if SimpleFIN adds EU banks). | Phase 2 |
| `account_number_last_4` | For UI disambiguation (e.g., "Chase Checking ...1234" vs "...5678"). | Phase 2 |
| `is_active` flag | Track closed accounts without deleting history. | Phase 2 |
| `categorization_config` (JSON) | Per-account category rules (e.g., ignore certain merchants). | Phase 2+ |

---

## Related Specifications

- **IMPLEMENTATION_PLAN**: Sprint 0 Gap #1, #2 (accounts schema + decision on debt_accounts)
- **README.md**: Lines 14-17 (checking/savings/credit distinction), Lines 191-202 (debt_accounts schema)
- **02_tauri_commands.md**: Command signatures for `get_accounts()`, `sync_simplefin()`
- **03_simplefin_auth.md**: Access URL provisioning and account sync strategy
