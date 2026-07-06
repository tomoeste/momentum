# Tauri Command Signatures & RPC Contract

**Document**: Formal specification for the React frontend ↔ Rust backend command interface  
**Status**: Frozen specification — frontend and backend may proceed in parallel  
**Last Updated**: 2026-07-06  

---

## 1. Overview

This document defines the complete RPC contract between the React/TypeScript frontend and the Rust backend via Tauri commands. All requests and responses are serialized as JSON and must be strictly type-compatible between TypeScript and Rust (via `serde`).

**Constraints**:
- All command handlers run on the main thread unless explicitly marked `async`
- Error propagation is **not** automatic; each handler must explicitly return `Result<T, AppError>`
- DTOs must remain stable — breaking changes require versioning (future: `/v2/`)
- SimpleFIN credentials stored in system keychain, never logged or transmitted in command payloads

---

## 2. Shared Error Enum

All commands return `Result<T, AppError>` where `AppError` is defined once and reused.

### AppError Enum (Rust + TypeScript)

**Rust definition** (placed in `src-tauri/src/error.rs`):

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum AppError {
    /// Database operation failed (open, query, migration)
    #[serde(rename = "Database")]
    Database { message: String },

    /// SimpleFIN API call failed (auth, network, rate limit)
    #[serde(rename = "SimpleFin")]
    SimpleFin { message: String, status_code: Option<u16> },

    /// LLM categorization request failed (Ollama or Claude API)
    #[serde(rename = "Llm")]
    Llm { message: String, source: String },

    /// User input validation failed (invalid dates, out-of-range values)
    #[serde(rename = "Validation")]
    Validation { field: String, message: String },

    /// Configuration missing or invalid (keychain access, env vars)
    #[serde(rename = "Config")]
    Config { message: String },

    /// Unrecoverable internal error (panic, unexpected state)
    #[serde(rename = "Internal")]
    Internal { message: String },

    /// Keychain operation failed (read, write, delete credentials)
    #[serde(rename = "Keychain")]
    Keychain { message: String },

    /// Requested resource not found (transaction, account, sync record)
    #[serde(rename = "NotFound")]
    NotFound { resource: String, id: String },
}

impl AppError {
    pub fn database(msg: impl Into<String>) -> Self {
        AppError::Database {
            message: msg.into(),
        }
    }

    pub fn simplefin(msg: impl Into<String>, status: Option<u16>) -> Self {
        AppError::SimpleFin {
            message: msg.into(),
            status_code: status,
        }
    }

    pub fn llm(msg: impl Into<String>, src: impl Into<String>) -> Self {
        AppError::Llm {
            message: msg.into(),
            source: src.into(),
        }
    }

    pub fn validation(field: impl Into<String>, msg: impl Into<String>) -> Self {
        AppError::Validation {
            field: field.into(),
            message: msg.into(),
        }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        AppError::Config {
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::Internal {
            message: msg.into(),
        }
    }

    pub fn keychain(msg: impl Into<String>) -> Self {
        AppError::Keychain {
            message: msg.into(),
        }
    }

    pub fn not_found(resource: impl Into<String>, id: impl Into<String>) -> Self {
        AppError::NotFound {
            resource: resource.into(),
            id: id.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Database { message } => write!(f, "Database: {}", message),
            AppError::SimpleFin { message, .. } => write!(f, "SimpleFIN: {}", message),
            AppError::Llm { message, source } => write!(f, "LLM ({}): {}", source, message),
            AppError::Validation { field, message } => write!(f, "Validation ({}): {}", field, message),
            AppError::Config { message } => write!(f, "Config: {}", message),
            AppError::Internal { message } => write!(f, "Internal: {}", message),
            AppError::Keychain { message } => write!(f, "Keychain: {}", message),
            AppError::NotFound { resource, id } => write!(f, "{} not found: {}", resource, id),
        }
    }
}

impl std::error::Error for AppError {}
```

**TypeScript definition** (placed in `src/lib/tauri-commands.ts`):

```typescript
export type AppErrorType =
  | "Database"
  | "SimpleFin"
  | "Llm"
  | "Validation"
  | "Config"
  | "Internal"
  | "Keychain"
  | "NotFound";

export interface AppError {
  type: AppErrorType;
  details:
    | { message: string }
    | { message: string; status_code?: number }
    | { message: string; source: string }
    | { field: string; message: string }
    | { resource: string; id: string };
}
```

---

## 3. Shared DTO Types

All requests and responses use these types. These must be kept synchronized between Rust and TypeScript.

### Core Data Types

#### Account

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,                    // UUID, internal key
    pub simplefin_account_id: String,  // SimpleFIN account ID (e.g., "1234567")
    pub name: String,                  // Display name (e.g., "Chase Checking")
    pub account_type: AccountType,     // checking | savings | credit | loan
    pub organization: String,          // Issuer (e.g., "Chase Bank")
    pub balance: f64,                  // Current balance (cents: 12345 = $123.45)
    pub last_updated: i64,             // Unix timestamp (seconds)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Checking,
    Savings,
    Credit,
    Loan,
}
```

```typescript
export type AccountType = "checking" | "savings" | "credit" | "loan";

export interface Account {
  id: string;
  simplefin_account_id: string;
  name: string;
  account_type: AccountType;
  organization: string;
  balance: number; // cents; 12345 = $123.45
  last_updated: number; // Unix timestamp (seconds)
}
```

#### Transaction

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,                         // SimpleFIN txn ID (internal key)
    pub account_id: String,                 // FK to Account
    pub posted_date: String,                // ISO 8601 date (YYYY-MM-DD)
    pub amount: f64,                        // cents; negative = debit
    pub merchant: String,                   // Display description
    pub raw_merchant: Option<String>,       // Unprocessed merchant string
    pub category: Option<String>,           // Categorization result (or null if pending)
    pub confidence: Option<f64>,            // 0.0..1.0 (null if pending)
    pub is_manual: bool,                    // User-overridden flag
    pub source: Option<String>,             // "ollama" | "api" | "user" | null
}
```

```typescript
export interface Transaction {
  id: string;
  account_id: string;
  posted_date: string; // YYYY-MM-DD
  amount: number; // cents; negative = debit
  merchant: string;
  raw_merchant?: string;
  category?: string;
  confidence?: number; // 0.0..1.0
  is_manual: boolean;
  source?: "ollama" | "api" | "user";
}
```

#### TransactionDetail

Extended view with secondary category and notes (for detail/edit modal).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDetail {
    pub id: String,
    pub account_id: String,
    pub posted_date: String,
    pub amount: f64,
    pub merchant: String,
    pub raw_merchant: Option<String>,
    pub category: Option<String>,
    pub secondary_category: Option<String>,
    pub confidence: Option<f64>,
    pub is_manual: bool,
    pub source: Option<String>,
    pub user_notes: Option<String>,
}
```

```typescript
export interface TransactionDetail extends Transaction {
  secondary_category?: string;
  user_notes?: string;
}
```

#### DashboardMetrics

Aggregated metrics for a time period (week or month).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub period: MetricsPeriod,                      // Week | Month
    pub period_start: String,                       // ISO 8601 date (YYYY-MM-DD)
    pub period_end: String,                         // ISO 8601 date (YYYY-MM-DD)
    pub income: f64,                                // cents, deposits to checking/savings
    pub spending: f64,                              // cents, absolute value
    pub debt_paydown: f64,                          // cents, principal paid
    pub interest_paid: f64,                         // cents, interest charges
    pub debt_ratio: f64,                            // 0.0..1.0+, (total_debt_balance / total_assets)
    pub interest_as_pct_income: f64,               // 0.0..100.0+
    pub sparkline_data: Vec<SparklinePoint>,       // Last 28 daily data points
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum MetricsPeriod {
    Week,
    Month,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparklinePoint {
    pub date: String,                              // YYYY-MM-DD
    pub income: f64,                               // cents
    pub spending: f64,                             // cents
    pub debt_paydown: f64,                         // cents
    pub interest: f64,                             // cents
}
```

```typescript
export type MetricsPeriod = "Week" | "Month";

export interface SparklinePoint {
  date: string; // YYYY-MM-DD
  income: number;
  spending: number;
  debt_paydown: number;
  interest: number;
}

export interface DashboardMetrics {
  period: MetricsPeriod;
  period_start: string; // YYYY-MM-DD
  period_end: string;
  income: number; // cents
  spending: number;
  debt_paydown: number;
  interest_paid: number;
  debt_ratio: number;
  interest_as_pct_income: number;
  sparkline_data: SparklinePoint[];
}
```

#### Scenario

Opportunity-cost scenario showing savings potential.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,                         // "scenario_200" | "scenario_500" (template ID)
    pub label: String,                      // "$200 extra/month" (human-readable)
    pub extra_monthly_payment: f64,         // cents
    pub months_saved: u32,                  // Months to pay off all debt
    pub interest_saved: f64,                // cents (baseline - accelerated)
    pub total_payoff_cost: f64,             // cents (interest + principal under accelerated plan)
}
```

```typescript
export interface Scenario {
  id: string;
  label: string;
  extra_monthly_payment: number; // cents
  months_saved: u32;
  interest_saved: number; // cents
  total_payoff_cost: number;
}
```

#### SyncStatus

Reflects the last sync attempt or current in-progress state.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub is_syncing: bool,
    pub last_sync_timestamp: Option<i64>,           // Unix timestamp (seconds), null if never
    pub last_sync_status: Option<SyncStatusType>,   // Success | Error (null if never)
    pub last_error: Option<String>,                 // Error message from last failed sync
    pub accounts_synced: u32,                       // Count of accounts fetched on last sync
    pub transactions_synced: u32,                   // Count of transactions inserted/updated
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum SyncStatusType {
    Success,
    Error,
}
```

```typescript
export type SyncStatusType = "Success" | "Error";

export interface SyncStatus {
  is_syncing: boolean;
  last_sync_timestamp?: number; // Unix timestamp (seconds)
  last_sync_status?: SyncStatusType;
  last_error?: string;
  accounts_synced: number;
  transactions_synced: number;
}
```

#### TransactionFilters

Request DTO for filtering transactions.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFilters {
    pub account_id: Option<String>,         // Filter by account (null = all)
    pub start_date: Option<String>,         // YYYY-MM-DD (inclusive)
    pub end_date: Option<String>,           // YYYY-MM-DD (inclusive)
    pub category: Option<String>,           // Exact category name (null = all)
    pub search_query: Option<String>,       // Case-insensitive merchant substring match
    pub min_amount: Option<f64>,            // cents (absolute value for spending)
    pub max_amount: Option<f64>,            // cents
    pub sort_by: Option<SortKey>,           // date | amount (default: date DESC)
    pub sort_order: Option<SortOrder>,      // asc | desc (default: desc)
    pub limit: Option<i32>,                 // Max rows (default: 500, max: 5000)
    pub offset: Option<i32>,                // Pagination offset (default: 0)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    Date,
    Amount,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}
```

```typescript
export type SortKey = "date" | "amount";
export type SortOrder = "asc" | "desc";

export interface TransactionFilters {
  account_id?: string;
  start_date?: string; // YYYY-MM-DD
  end_date?: string;
  category?: string;
  search_query?: string;
  min_amount?: number; // cents
  max_amount?: number;
  sort_by?: SortKey;
  sort_order?: SortOrder;
  limit?: number; // default 500, max 5000
  offset?: number; // default 0
}
```

---

## 4. Command Definitions

Each command is defined with its signature, description, request/response structure, and error cases.

### 4.1 get_dashboard_metrics

Retrieve aggregated financial metrics for a time period.

**Async**: Yes  
**Authentication**: None required

**Request**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GetDashboardMetricsRequest {
    pub period: MetricsPeriod,  // Week | Month
}
```

```typescript
interface GetDashboardMetricsRequest {
  period: MetricsPeriod;
}
```

**Response**:
```rust
DashboardMetrics
```

```typescript
DashboardMetrics
```

**Error cases**:
- `Database`: Failed to query metrics (transaction, account tables)
- `Internal`: Calculation overflow or state corruption

**Implementation notes**:
- Metrics calculated from `raw_transactions` and `categorized_transactions`
- Period boundaries: Week = last 7 days (today inclusive); Month = calendar month
- Income = deposits to `checking` or `savings` accounts
- Spending = negative transactions excluding Debt Payments, Transfers, Interest, Internal Xfers
- Debt Paydown = positive transactions to debt accounts (principal component)
- Sparkline data = last 28 daily data points; null days filled with zeros

---

### 4.2 get_transactions

Retrieve paginated transaction list with optional filters.

**Async**: Yes  
**Authentication**: None required

**Request**:
```rust
TransactionFilters
```

```typescript
TransactionFilters
```

**Response**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionsResponse {
    pub transactions: Vec<Transaction>,
    pub total_count: i32,  // Total matching records (ignoring pagination)
    pub has_more: bool,    // Whether more results exist
}
```

```typescript
interface GetTransactionsResponse {
  transactions: Transaction[];
  total_count: number;
  has_more: boolean;
}
```

**Error cases**:
- `Validation`: Invalid date range, negative amounts, offset out of range
- `Database`: Query failed

**Implementation notes**:
- Supports full-text search on merchant (case-insensitive substring)
- Limit capped at 5000 to prevent resource exhaustion
- Sorting by date (DESC default) uses `posted_date`; by amount uses absolute value
- Pagination via limit + offset (prefer for frontend cursors)

---

### 4.3 get_transaction_detail

Retrieve full detail view of a single transaction for editing.

**Async**: Yes  
**Authentication**: None required

**Request**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionDetailRequest {
    pub transaction_id: String,
}
```

```typescript
interface GetTransactionDetailRequest {
  transaction_id: string;
}
```

**Response**:
```rust
TransactionDetail
```

```typescript
TransactionDetail
```

**Error cases**:
- `NotFound`: Transaction does not exist
- `Database`: Query failed

**Implementation notes**:
- Includes secondary category and user notes fields (not in list view)
- Used in detail/recategorization modal

---

### 4.4 recategorize_transaction

Update category, secondary category, and notes for a single transaction.

**Async**: No (direct DB write, <1ms)  
**Authentication**: None required

**Request**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct RecategorizeTransactionRequest {
    pub transaction_id: String,
    pub category: Option<String>,           // null clears category
    pub secondary_category: Option<String>, // null clears
    pub user_notes: Option<String>,         // null clears
    pub is_manual: bool,                    // Mark as user-overridden
}
```

```typescript
interface RecategorizeTransactionRequest {
  transaction_id: string;
  category?: string;
  secondary_category?: string;
  user_notes?: string;
  is_manual: boolean;
}
```

**Response**:
```rust
()  // Returns empty success or AppError
```

```typescript
void
```

**Error cases**:
- `NotFound`: Transaction does not exist
- `Validation`: Empty category string (use null instead)
- `Database`: Update failed (locked transaction, FK violation)

**Implementation notes**:
- Atomically updates `categorized_transactions` table
- Sets `source: "user"` when `is_manual: true`
- Does not recalculate metrics; metrics refresh on next dashboard query

---

### 4.5 get_opportunity_scenarios

Generate opportunity-cost scenarios for accelerated debt payoff.

**Async**: Yes  
**Authentication**: None required

**Request**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GetOpportunityScenariosRequest {}  // Empty for now
```

```typescript
interface GetOpportunityScenariosRequest {}
```

**Response**:
```rust
Vec<Scenario>  // Typically: [$200/mo, $500/mo] scenarios
```

```typescript
Scenario[]
```

**Error cases**:
- `Database`: Failed to load debt accounts or transaction history
- `Internal`: Amortization calculation error (negative interest, invalid APR)

**Implementation notes**:
- Queries all debt accounts (loans, credit cards) from `debt_accounts` table
- Per-account APR + minimum payment must be set (from `debt_accounts.interest_rate`, `minimum_payment`)
- Uses amortization formula: `n = -ln(1 - r*P/PMT) / ln(1+r)` where:
  - `P` = current principal balance
  - `r` = monthly APR (annual / 12 / 100)
  - `PMT` = monthly payment
  - `n` = months to payoff
- Generates 2–3 templates: [+$200/mo, +$500/mo] (adjustable in future)
- For each scenario, calculates total interest paid under accelerated schedule
- Aggregates across all debt accounts

---

### 4.6 sync_simplefin

Trigger a full SimpleFIN sync: fetch accounts, transactions, categorize, update metrics.

**Async**: Yes (long-running, should emit progress updates in future)  
**Authentication**: Requires SimpleFIN access URL in keychain

**Request**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncSimplefinRequest {}  // No params; uses keychain
```

```typescript
interface SyncSimplefinRequest {}
```

**Response**:
```rust
SyncStatus
```

```typescript
SyncStatus
```

**Error cases**:
- `Keychain`: Access URL not found or unreadable
- `SimpleFin`: API unreachable, auth failed, rate limited
- `Database`: Transaction insert/upsert failed (corrupted data, FK violation)
- `Config`: Missing required config (data dir, db path)

**Implementation notes**:
- Fetches `/accounts`, `/transactions` from SimpleFIN access URL
- Parses sign convention: positive = credit/deposit, negative = debit/withdrawal
- Skips pending transactions
- Dedupes by SimpleFIN transaction ID before insert
- Upserts into `raw_transactions` + `accounts` tables
- Queues unprocessed transactions for categorization (non-blocking)
- Updates `sync_log` with timestamp + status
- Metric cache invalidated; metrics recalculated on next dashboard query
- Should handle network retry + backoff (implementation detail, not API contract)

---

### 4.7 get_sync_status

Poll current or last sync state without blocking.

**Async**: No (reads sync_log + in-memory flag)  
**Authentication**: None required

**Request**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GetSyncStatusRequest {}
```

```typescript
interface GetSyncStatusRequest {}
```

**Response**:
```rust
SyncStatus
```

```typescript
SyncStatus
```

**Error cases**:
- `Database`: Failed to query sync_log

**Implementation notes**:
- Non-blocking; returns immediately
- `is_syncing` = true only if sync_simplefin currently running
- `last_sync_*` fields null if never synced
- Used for progress indicator + error display in header

---

### 4.8 get_accounts

List all accounts (checking, savings, credit, loans).

**Async**: Yes  
**Authentication**: None required

**Request**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GetAccountsRequest {}
```

```typescript
interface GetAccountsRequest {}
```

**Response**:
```rust
Vec<Account>
```

```typescript
Account[]
```

**Error cases**:
- `Database`: Query failed

**Implementation notes**:
- Ordered by account type (checking, savings, credit, loan) then by name
- Includes all account types; filtering done on frontend
- Balance is current (from last SimpleFIN fetch)

---

### 4.9 set_debt_terms

Update APR and minimum payment for a debt account.

**Async**: No (direct DB write, <1ms)  
**Authentication**: None required

**Request**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SetDebtTermsRequest {
    pub account_id: String,
    pub apr: f64,                      // Annual interest rate, 0.0..100.0
    pub minimum_payment: f64,          // cents
}
```

```typescript
interface SetDebtTermsRequest {
  account_id: string;
  apr: number;
  minimum_payment: number; // cents
}
```

**Response**:
```rust
()  // Returns empty success or AppError
```

```typescript
void
```

**Error cases**:
- `NotFound`: Account does not exist or is not a debt account
- `Validation`: APR < 0 or > 100, minimum_payment < 0
- `Database`: Update failed

**Implementation notes**:
- Updates `debt_accounts` table: `interest_rate`, `minimum_payment` fields
- APR must be annual percentage (e.g., 21.5 for 21.5%)
- Minimum payment is required for amortization math; must be > 0
- After update, opportunity scenarios will reflect new APR/payment on next fetch

---

## 5. Error Handling Contract

### Request Validation

All commands must validate input before processing:

```rust
// Example: validate APR in set_debt_terms
if request.apr < 0.0 || request.apr > 100.0 {
    return Err(AppError::validation("apr", "APR must be between 0 and 100"));
}
```

**Frontend responsibility**:
- Display error toast with `AppError.details.message`
- Log full error for debugging
- Do not automatically retry on Validation errors

### Response Marshaling

All successful responses must be JSON-serializable:

```rust
#[tauri::command]
pub async fn get_dashboard_metrics(
    period: MetricsPeriod,
    app_state: tauri::State<'_, AppState>,
) -> Result<DashboardMetrics, AppError> {
    // Implementation
}
```

---

## 6. Async Command Behavior

Async commands run in Tauri's async runtime pool and should not block the UI.

**Tauri async signature** (Rust):
```rust
#[tauri::command]
pub async fn command_name(params: ..., state: tauri::State<'_, AppState>) -> Result<T, AppError> {
    // May call I/O (database, network)
}
```

**Frontend invocation** (TypeScript):
```typescript
import { invoke } from "@tauri-apps/api/core";

const metrics = await invoke<DashboardMetrics>("get_dashboard_metrics", {
  period: "Week",
});
```

**Commands that are async**:
- `get_dashboard_metrics`
- `get_transactions`
- `get_transaction_detail`
- `get_opportunity_scenarios`
- `sync_simplefin`

**Commands that are NOT async** (synchronous):
- `recategorize_transaction`
- `get_sync_status`
- `get_accounts` (may become async if cached)
- `set_debt_terms`

---

## 7. Type Synchronization Procedure

To maintain DTO consistency:

1. **Source of truth**: This document (specs/02_tauri_commands.md)
2. **Rust implementation**: `src-tauri/src/models.rs` (derived from spec via `serde`)
3. **TypeScript bindings**: `src/lib/tauri-commands.ts` (manually written, validated against spec)
4. **Validation**: On each build, typecheck both sides independently; before release, manual spot-check

**Breaking changes**:
- Bump version in request/response DTOs (e.g., `GetDashboardMetricsRequest` → `GetDashboardMetricsV2Request`)
- Update command name if signature changes (e.g., `get_dashboard_metrics_v2`)
- Document migration path in CHANGELOG.md

---

## 8. Example: Full Request/Response Flow

### Scenario: Recategorize Transaction

**Frontend code**:
```typescript
const response = await invoke<void>("recategorize_transaction", {
  transaction_id: "txn_abc123",
  category: "Food & Dining",
  secondary_category: "Restaurants",
  user_notes: "Lunch with team",
  is_manual: true,
});
```

**Rust handler**:
```rust
#[tauri::command]
pub fn recategorize_transaction(
    request: RecategorizeTransactionRequest,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
    // Validate input
    if request.category.as_ref().map(|c| c.is_empty()).unwrap_or(false) {
        return Err(AppError::validation("category", "Category cannot be empty string"));
    }

    // Query database
    let mut conn = state.db.lock().map_err(|_| AppError::internal("Lock poisoned"))?;
    conn.execute(
        "UPDATE categorized_transactions 
         SET category = ?, secondary_category = ?, user_notes = ?, source = 'user', is_manual = ?
         WHERE id = ?",
        params![
            request.category,
            request.secondary_category,
            request.user_notes,
            request.is_manual,
            request.transaction_id,
        ],
    ).map_err(|e| AppError::database(e.to_string()))?;

    Ok(())
}
```

**Frontend error handling**:
```typescript
try {
  await invoke("recategorize_transaction", { ... });
  toast.success("Transaction updated");
} catch (error: unknown) {
  const err = error as AppError;
  if (err.type === "NotFound") {
    toast.error("Transaction no longer exists");
  } else if (err.type === "Validation") {
    toast.error(`Invalid input: ${err.details.message}`);
  } else {
    toast.error(`Error: ${err.details.message}`);
  }
}
```

---

## 9. Future Enhancements

These are out of scope for MVP but documented for reference:

- **Pagination cursors**: Replace offset-based pagination with cursor tokens (prevents offset skew)
- **Command versioning**: `/v2/` endpoint path for breaking changes
- **Event streaming**: Real-time sync progress updates via Tauri event emitter
- **Batch commands**: Recategorize multiple transactions in one RPC
- **Query optimizations**: Indexed search, metric caching with TTL
- **Audit logging**: Track all mutations (recategorize, debt-terms edits)

---

## 10. Checklist for Implementation

- [ ] Define `AppError` enum in `src-tauri/src/error.rs` with all variants
- [ ] Define all DTOs in `src-tauri/src/models.rs` with `serde` derives
- [ ] Implement all 9 command handlers with proper error propagation
- [ ] Write TypeScript DTO definitions in `src/lib/tauri-commands.ts`
- [ ] Validate Rust compilation + serde serialization
- [ ] Add per-command tests (error cases, boundary values)
- [ ] Generate/verify TypeScript types against Rust models (compare field names/types)
- [ ] Document command invocation patterns in DEVELOPMENT.md
- [ ] Create mock implementations in `src/lib/mocks.ts` for frontend development
- [ ] Before merging CP3: spot-check all error cases and type compatibility

---

## 11. Version History

| Version | Date       | Changes                      |
|---------|------------|------------------------------|
| 1.0     | 2026-07-06 | Initial specification frozen |

**Frozen status**: This document is locked for MVP development. Changes require unanimous team approval.
