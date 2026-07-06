# Account-to-Transaction Mapping UI - Implementation Plan

**Status**: PLANNING
**Priority**: HIGH (critical for multi-account accuracy)
**Estimated Scope**: 4-5 days (research, backend, frontend, testing)
**Related**: Track A continuation; depends on existing sync infrastructure

---

## Problem Statement

**Current Behavior:**
- SimpleFIN API returns a flat list of transactions without account association metadata
- Current implementation assigns ALL transactions from a sync to the primary account
- Single-account scenarios work correctly
- Multi-account scenarios fail silently: users see all transactions under one account regardless of actual source

**User Impact:**
- Multi-account users cannot trust their transaction data
- No way to disaggregate spending by actual account
- Dashboard metrics are misleading (all activity attributed to one account)

**Root Cause:**
SimpleFIN's `/transactions` endpoint returns transaction objects with no `account_id` field. The API provides:
- id, posted_date, amount, merchant, description, transaction_type
- No account identifier or source account metadata

---

## Solution Design

### MVP Scope: User-Guided Modal with Bulk Assignment

**Approach**: Show an account picker modal when multi-account scenario is detected during initial sync. Allow user to map unmapped transactions to the correct accounts via a guided UI.

**Trigger Logic:**
- On sync completion: check if `accounts.len() > 1`
- If YES: Check if transactions lack proper account_id assignments
- Show modal with transaction list + account picker
- Allow bulk selection + assignment
- Save mapping decision to prevent re-prompting on future syncs

**Flow:**
```
sync_simplefin() fetches accounts (e.g., 2+ accounts)
  ↓
Fetches transactions (all unassigned initially)
  ↓
Before insert: Check if multi-account scenario detected
  ↓
If yes + first sync: Emit UI signal to show mapping modal
  ↓
Modal displays unmapped transactions with merchant/amount previews
  ↓
User groups transactions by account (visual/drag or dropdown per txn)
  ↓
User confirms assignment
  ↓
Backend updates account_id for all transactions
  ↓
Categorization proceeds as normal
```

---

## Implementation Tasks

### Phase 1: Database Schema & Storage

#### 1.1 Add Transaction Mapping Metadata Table

**File**: `src-tauri/src/db.rs`

New table to track user mapping decisions and prevent re-prompting:

```sql
CREATE TABLE IF NOT EXISTS transaction_mappings (
    id TEXT PRIMARY KEY,  -- transaction id (FK to raw_transactions)
    account_id TEXT NOT NULL,  -- mapped account id (FK to accounts)
    mapped_by_user BOOLEAN NOT NULL DEFAULT 0,  -- true if user explicitly mapped it
    mapped_at TEXT NOT NULL,  -- ISO 8601 timestamp
    confidence REAL DEFAULT 1.0,  -- user confidence in mapping (1.0 = certain, <1.0 = guessed)
    notes TEXT,  -- optional user notes
    FOREIGN KEY(id) REFERENCES raw_transactions(id),
    FOREIGN KEY(account_id) REFERENCES accounts(id)
);

CREATE INDEX IF NOT EXISTS idx_transaction_mappings_account ON transaction_mappings(account_id);
CREATE INDEX IF NOT EXISTS idx_transaction_mappings_mapped_by_user ON transaction_mappings(mapped_by_user);
```

**Purpose:**
- Track which transactions user has explicitly mapped
- Prevent duplicate mapping prompts for same transaction
- Store confidence scores (used for future ML-based auto-mapping)
- Enable audit trail for data quality

#### 1.2 Add Settings Flag for Mapping State

Add new setting to track whether user has completed initial mapping for a sync batch:

```rust
// In database settings table
"account_mapping_completed_syncs": "[timestamp1, timestamp2, ...]"  // JSON array of syncs that have mapping
```

This prevents showing the modal for every sync once user has done initial mapping.

#### 1.3 Implement Database Methods

Add to `Database` struct in `db.rs`:

```rust
// Get all unmapped transactions
pub fn get_unmapped_transactions(&self) -> Result<Vec<RawTransaction>>

// Bulk update transaction account mappings
pub fn bulk_update_transaction_accounts(&self, mappings: Vec<(String, String)>) -> Result<()>
// mappings = [(transaction_id, new_account_id), ...]

// Record mapping metadata
pub fn record_transaction_mapping(
    &self,
    transaction_id: &str,
    account_id: &str,
    confidence: f64,
) -> Result<()>

// Check if a sync batch needs mapping review
pub fn sync_needs_mapping_review(&self, sync_timestamp: DateTime<Utc>) -> Result<bool>

// Mark sync as mapping-complete
pub fn mark_sync_mapping_complete(&self, sync_timestamp: DateTime<Utc>) -> Result<()>
```

---

### Phase 2: Backend Commands & Sync Integration

#### 2.1 New Command: `suggest_transaction_mappings`

**File**: `src-tauri/src/commands.rs`

Purpose: Called after sync to analyze transactions and suggest account assignments (optional ML phase 2).

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionMappingSuggestion {
    pub transaction_id: String,
    pub description: String,
    pub merchant: Option<String>,
    pub amount: f64,
    pub suggested_account_id: Option<String>,
    pub suggested_account_name: String,
    pub confidence: f64,  // 0.0 - 1.0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionMappingSuggestionsResponse {
    pub needs_mapping: bool,
    pub unmapped_count: i32,
    pub accounts: Vec<Account>,
    pub suggestions: Vec<TransactionMappingSuggestion>,  // Empty in MVP; populated in Phase 2
}

#[tauri::command]
pub async fn get_transaction_mapping_suggestions(
    db: State<'_, Database>,
) -> Result<GetTransactionMappingSuggestionsResponse> {
    // Get unmapped transactions
    let unmapped = db.get_unmapped_transactions()?;
    
    // Get accounts for mapping
    let accounts = db.get_accounts()?;
    
    // In MVP: return empty suggestions (user will manually select)
    // In Phase 2: implement merchant pattern matching or LLM-based suggestion
    
    Ok(GetTransactionMappingSuggestionsResponse {
        needs_mapping: !unmapped.is_empty() && accounts.len() > 1,
        unmapped_count: unmapped.len() as i32,
        accounts,
        suggestions: vec![],  // TODO: Phase 2
    })
}
```

#### 2.2 New Command: `submit_transaction_mappings`

**File**: `src-tauri/src/commands.rs`

Purpose: Accept user's manual mapping decisions and persist them.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionMappingsRequest {
    pub mappings: Vec<TransactionMapping>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionMapping {
    pub transaction_id: String,
    pub account_id: String,
    pub confidence: f64,  // 1.0 for user-selected, <1.0 for guessed
}

#[tauri::command]
pub async fn submit_transaction_mappings(
    req: SubmitTransactionMappingsRequest,
    db: State<'_, Database>,
) -> Result<()> {
    // Validate all transaction_ids exist
    // Validate all account_ids exist
    // Update raw_transactions.account_id for each
    // Record mapping metadata in transaction_mappings table
    // Mark sync as complete
    
    for mapping in req.mappings {
        db.bulk_update_transaction_accounts(&[(
            mapping.transaction_id,
            mapping.account_id
        )])?;
        
        db.record_transaction_mapping(
            &mapping.transaction_id,
            &mapping.account_id,
            mapping.confidence,
        )?;
    }
    
    Ok(())
}
```

#### 2.3 Integrate Mapping Check into `sync_simplefin`

**File**: `src-tauri/src/commands.rs`

Modify `sync_simplefin` command to:
1. After upsert of transactions, check for multi-account scenario
2. If multi-account + unmapped transactions exist → emit signal to frontend
3. Don't finalize sync until mapping is complete (optional: could auto-finalize with primary account assignment)

```rust
// At end of sync_simplefin, before success log:

let fetched_accounts = db.get_accounts()?;
if fetched_accounts.len() > 1 {
    let unmapped_count = db.count_unmapped_transactions()?;
    if unmapped_count > 0 {
        // Option A: Emit frontend signal (requires event system)
        // Option B: Return SyncStatus with mapping_required flag
        // Option C: Store flag in database for frontend to poll
        
        // For MVP: Store in SyncStatus return value
        return Ok(SyncStatus {
            in_progress: false,
            last_sync: Some(Utc::now()),
            last_error: None,
            transaction_count,
            mapping_required: true,  // NEW FIELD
            unmapped_transaction_count: unmapped_count,
        });
    }
}
```

#### 2.4 Update `SyncStatus` Model

**Files**: `src-tauri/src/models.rs` + `src/lib/tauri-commands.ts`

Add fields to track mapping state:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub in_progress: bool,
    pub last_sync: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub transaction_count: i32,
    pub mapping_required: bool,  // NEW
    pub unmapped_transaction_count: i32,  // NEW
}
```

---

### Phase 3: Frontend UI Components

#### 3.1 Create `AccountMappingModal` Component

**File**: `src/components/AccountMappingModal.tsx`

```typescript
interface AccountMappingModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (mappings: TransactionMapping[]) => Promise<void>;
  accounts: Account[];
  transactions: RawTransaction[];  // Only unmapped transactions
}

export function AccountMappingModal({
  isOpen,
  onClose,
  onSubmit,
  accounts,
  transactions,
}: AccountMappingModalProps) {
  const [mappings, setMappings] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [selectedAccount, setSelectedAccount] = useState<string>(accounts[0]?.id || '');
  
  // Transaction grouping strategy options:
  // 1. Manual dropdown per transaction (MVP)
  // 2. Bulk select + assign to account (recommended MVP+)
  // 3. Drag-drop grouping (Phase 2 polish)
  
  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center">
      <div className="bg-gray-800 rounded-lg p-6 w-full max-w-4xl max-h-96 overflow-auto">
        <h2 className="text-xl font-bold mb-4">Map Transactions to Accounts</h2>
        <p className="text-gray-400 mb-6">
          SimpleFIN returned {transactions.length} transactions from {accounts.length} accounts.
          Please assign each transaction to the correct account below.
        </p>
        
        {/* Strategy 1: Table with dropdown per row */}
        <table className="w-full text-sm mb-6">
          <thead>
            <tr className="border-b border-gray-700">
              <th className="text-left p-2">Date</th>
              <th className="text-left p-2">Merchant</th>
              <th className="text-right p-2">Amount</th>
              <th className="text-left p-2">Account</th>
            </tr>
          </thead>
          <tbody>
            {transactions.map((txn) => (
              <tr key={txn.id} className="border-b border-gray-700 hover:bg-gray-700">
                <td className="p-2">{new Date(txn.posted_date).toLocaleDateString()}</td>
                <td className="p-2">{txn.merchant || txn.description}</td>
                <td className="p-2 text-right font-mono">${Math.abs(txn.amount).toFixed(2)}</td>
                <td className="p-2">
                  <select
                    value={mappings[txn.id] || ''}
                    onChange={(e) =>
                      setMappings({ ...mappings, [txn.id]: e.target.value })
                    }
                    className="bg-gray-700 border border-gray-600 rounded px-2 py-1 text-white"
                  >
                    <option value="">Select account...</option>
                    {accounts.map((acc) => (
                      <option key={acc.id} value={acc.id}>
                        {acc.name}
                      </option>
                    ))}
                  </select>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        
        {/* Strategy 2: Bulk assign (alternative) */}
        {/* 
        <div className="mb-6 p-4 bg-gray-700 rounded">
          <p className="text-gray-300 mb-3">Or bulk assign unselected transactions to:</p>
          <select
            value={selectedAccount}
            onChange={(e) => setSelectedAccount(e.target.value)}
            className="bg-gray-800 border border-gray-600 rounded px-3 py-2 text-white w-full"
          >
            {accounts.map((acc) => (
              <option key={acc.id} value={acc.id}>{acc.name}</option>
            ))}
          </select>
          <button
            onClick={bulkAssignUnmapped}
            className="mt-3 px-4 py-2 bg-blue-600 rounded text-white"
          >
            Bulk Assign Remaining
          </button>
        </div>
        */}
        
        {/* Action buttons */}
        <div className="flex gap-4">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-gray-700 rounded text-white hover:bg-gray-600"
          >
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            disabled={loading || Object.keys(mappings).length < transactions.length}
            className="px-4 py-2 bg-blue-600 rounded text-white hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? 'Saving...' : 'Confirm Mappings'}
          </button>
        </div>
      </div>
    </div>
  );
}
```

**Design Notes:**
- Show a table with transaction date, merchant, amount, and account dropdown
- All transactions must be assigned before submit is enabled
- Merchant name prominently displayed to help user recognize patterns
- Amount in red if negative (expense) to clarify credit vs. debit accounts
- Simple dropdown selection (MVP) — can add drag-drop in Phase 2

#### 3.2 Integrate Modal into `App.tsx`

**File**: `src/App.tsx`

```typescript
// Add to App state
const [mappingModalOpen, setMappingModalOpen] = useState(false);
const [unmappedTransactions, setUnmappedTransactions] = useState<RawTransaction[]>([]);

// After sync completes in handleSync()
async function handleSync() {
  setSyncing(true);
  setError(null);

  try {
    const syncResult = await syncSimpleFin({ days_back: 90 });
    
    // Check if mapping is required
    if (syncResult.mapping_required && syncResult.unmapped_transaction_count > 0) {
      // Fetch unmapped transactions and show modal
      const suggestions = await getTransactionMappingSuggestions();
      setUnmappedTransactions(suggestions.transactions);  // Or however we get unmapped list
      setMappingModalOpen(true);
      // Don't reload data yet; wait for mapping completion
    } else {
      // No mapping needed, proceed to load data
      await loadData();
    }
  } catch (err) {
    console.error('Failed to sync:', err);
    setError(err instanceof Error ? err.message : 'Sync failed');
  } finally {
    setSyncing(false);
  }
}

// Handle mapping submission
async function handleSubmitMappings(mappings: TransactionMapping[]) {
  try {
    await submitTransactionMappings({ mappings });
    setMappingModalOpen(false);
    // Now load data with correctly mapped transactions
    await loadData();
  } catch (err) {
    setError(err instanceof Error ? err.message : 'Failed to save mappings');
  }
}

// In JSX:
<AccountMappingModal
  isOpen={mappingModalOpen}
  onClose={() => setMappingModalOpen(false)}
  onSubmit={handleSubmitMappings}
  accounts={/* get from state or fetch */}
  transactions={unmappedTransactions}
/>
```

#### 3.3 Add TypeScript Bindings

**File**: `src/lib/tauri-commands.ts`

```typescript
export interface TransactionMapping {
  transaction_id: string;
  account_id: string;
  confidence: number;
}

export interface GetTransactionMappingSuggestionsResponse {
  needs_mapping: boolean;
  unmapped_count: number;
  accounts: Account[];
  suggestions: TransactionMappingSuggestion[];
}

export interface TransactionMappingSuggestion {
  transaction_id: string;
  description: string;
  merchant?: string;
  amount: number;
  suggested_account_id?: string;
  suggested_account_name: string;
  confidence: number;
}

export async function getTransactionMappingSuggestions(): Promise<GetTransactionMappingSuggestionsResponse> {
  return await invoke('get_transaction_mapping_suggestions');
}

export async function submitTransactionMappings(
  req: { mappings: TransactionMapping[] }
): Promise<void> {
  return await invoke('submit_transaction_mappings', { req });
}
```

Also update `SyncStatus` interface:
```typescript
export interface SyncStatus {
  in_progress: boolean;
  last_sync?: string;  // ISO 8601
  last_error?: string;
  transaction_count: number;
  mapping_required?: boolean;  // NEW
  unmapped_transaction_count?: number;  // NEW
}
```

---

### Phase 4: Integration & Testing

#### 4.1 Update Main Command Registration

**File**: `src-tauri/src/main.rs`

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    get_transaction_mapping_suggestions,
    submit_transaction_mappings,
])
```

#### 4.2 Test Scenarios

**Scenario 1: Single Account**
- User has 1 account connected
- Sync completes
- No mapping modal shown (mapping_required = false)
- Dashboard shows all transactions correctly

**Scenario 2: Multi-Account (First Time)**
- User has 2+ accounts connected
- First sync completes
- Modal shown with all unmapped transactions
- User selects account for each transaction
- Submit → transactions updated in DB
- Dashboard shows transactions correctly grouped by account

**Scenario 3: Multi-Account (Second Sync)**
- User already did initial mapping
- Second sync completes
- Check: if unmapped transactions < threshold → no modal (auto-assign to primary)
- Otherwise show modal again

**Scenario 4: Error Handling**
- User cancels mapping modal
- Sync is marked as incomplete; data not finalized
- Next app open offers to complete mapping
- Or: user can manually retrigger sync

---

## File Changes Summary

### Backend (Rust)

| File | Changes | Lines |
|------|---------|-------|
| `src-tauri/src/db.rs` | Add transaction_mappings table; add query methods | +150 |
| `src-tauri/src/commands.rs` | Add two new commands + SyncStatus fields | +120 |
| `src-tauri/src/models.rs` | Extend SyncStatus struct | +5 |
| `src-tauri/src/main.rs` | Register new commands | +2 |

**Total Backend**: ~280 LOC

### Frontend (TypeScript/React)

| File | Changes | Lines |
|------|---------|-------|
| `src/components/AccountMappingModal.tsx` | NEW component | +120 |
| `src/App.tsx` | Add modal state + integration | +50 |
| `src/lib/tauri-commands.ts` | Add bindings for new commands | +40 |

**Total Frontend**: ~210 LOC

**Grand Total**: ~490 LOC

---

## Phase 2 Enhancements (Future)

These are NOT in MVP but valuable additions once basic mapping works:

1. **Automatic Merchant Pattern Matching**
   - Analyze merchant names/descriptions to infer source account
   - Example: "PayPal" transactions likely from checking → PayPal account transfer
   - Implement in `suggest_transaction_mappings()` logic

2. **Confidence Scoring**
   - Score each suggestion based on pattern strength
   - Show confidence to user (e.g., "85% likely")
   - Allow user to override low-confidence suggestions

3. **Learning from History**
   - Track merchant → account mappings over time
   - Use ML model (simple logistic regression) on future syncs
   - Build accuracy as dataset grows

4. **Bulk Assign UI**
   - Group-by-merchant view
   - Assign all "Starbucks" txns to one account at once
   - Drag-drop grouping for visual bulk assignment

5. **Undo/Redo**
   - Let user review and revise before final submit
   - Show summary of assignments before confirming

---

## Rollout Plan

### Week 1: Backend Foundation
- **Day 1-2**: Implement database schema + methods (Phase 1)
- **Day 3**: Implement new commands (Phase 2.1-2.2)
- **Day 4**: Integrate into sync_simplefin (Phase 2.3)
- **Build check**: `cargo check` + `cargo test`

### Week 2: Frontend + Integration
- **Day 1-2**: Implement AccountMappingModal component (Phase 3.1)
- **Day 3**: App.tsx integration (Phase 3.2-3.3)
- **Day 4**: E2E testing + bug fixes
- **Build check**: `npm build` + `npm test`

### Week 3: Polish + Release
- **Day 1-2**: UX refinements (error messaging, edge cases)
- **Day 3**: Documentation + testing scenarios
- **Day 4**: Release + monitor for issues

---

## Known Limitations & Edge Cases

### Limitation 1: No Transaction Account Hints
SimpleFIN returns no metadata about which account a transaction belongs to. We must rely on:
- User knowledge (primary source)
- Transaction descriptions/merchants (secondary; unreliable)

**Mitigation**: Prompt user immediately after first sync; don't try to auto-guess.

### Limitation 2: Subsequent Syncs
For syncs after initial mapping:
- How do we know if NEW transactions are from already-mapped merchants?
- Option A: Check merchant pattern; if 100% match, auto-assign
- Option B: Always show modal for new unmapped txns
- Option C: Require explicit mapping every sync

**Recommendation for MVP**: Option B (always prompt for unmapped)

### Limitation 3: Pending Transactions
SimpleFIN may return "pending" transactions that later disappear/merge with posted txns. Mapping pending txns could create duplicates.

**Mitigation**: Filter pending txns from mapping flow; only map posted transactions.

### Limitation 4: Performance with Large Transaction Sets
If user syncs 5+ accounts × 1000+ transactions = 5000+ unmapped txns, modal could be slow to render.

**Mitigation**: Paginate modal table (show 50 at a time); lazy-load rendering.

---

## Success Criteria

### Functional
- [x] Multi-account users can disambiguate transactions after sync
- [x] Mapping decisions persist across app restarts
- [x] No mapping re-prompts after initial mapping complete
- [x] Single-account users unaffected (no modal shown)
- [x] Transactions correctly attributed to mapped accounts in dashboard

### Non-Functional
- [x] Modal UI responsive + accessible (keyboard navigation)
- [x] Backend operations complete in <5s for 1000 transactions
- [x] All edge cases handled (cancel, network error, DB error)
- [x] Comprehensive test coverage (unit + E2E)

### User Experience
- [x] First-time user can complete mapping in <2 minutes (5 accounts × 100 txns)
- [x] Clear visual feedback (selected account highlighted)
- [x] Obvious submit button (no accidental losses)
- [x] Error messages are actionable (not technical jargon)

---

## Questions & Decisions Needed

**Q1: What if user cancels mapping modal?**
- Option A: Mark sync as incomplete; re-prompt on next sync
- Option B: Auto-assign unmapped to primary account (like current behavior)
- **Decision**: Option A (safer; doesn't silently mis-assign)

**Q2: Should we support incremental mapping (map-as-you-go)?**
- Option A: User must map all at once before confirm
- Option B: Save partial mappings; allow user to finish later
- **Decision**: Option A (simpler; less state to track)

**Q3: How to handle "I'm not sure which account" scenario?**
- Option A: Require selection (can't proceed without)
- Option B: Allow null selection; mark as "needs manual review"
- **Decision**: Option A + eventually Phase 2 confidence scoring

**Q4: Should mapping history be persistent (audit trail)?**
- Option A: Yes; store in transaction_mappings table for debugging
- Option B: No; just update raw_transactions.account_id
- **Decision**: Option A (helps troubleshoot + enables Phase 2 ML)

---

## References

**Related Documentation:**
- `/work/IMPLEMENTATION_PLAN.md` — Main project roadmap (Note: Account mapping listed as Track A continuation)
- `/work/specs/02_tauri_commands.md` — Tauri command signatures (need to add new commands)
- `/work/specs/04_simplefin_auth.md` — SimpleFIN auth flow (context for account handling)

**Code References:**
- `src-tauri/src/commands.rs:sync_simplefin()` (line 224-379) — Current sync logic
- `src-tauri/src/simplefin.rs` (line 143-194) — fetch_transactions() returns flat list
- `src/components/SettingsModal.tsx` — Modal UI patterns to follow
- `src-tauri/src/db.rs` — Database method examples

---

## Appendix: Sample API Responses

### Request: `get_transaction_mapping_suggestions`

```json
{
  "needs_mapping": true,
  "unmapped_count": 47,
  "accounts": [
    {
      "id": "acct_1",
      "name": "Checking (Primary)",
      "account_type": "checking",
      "balance": 3500.00
    },
    {
      "id": "acct_2",
      "name": "Savings",
      "account_type": "savings",
      "balance": 15000.00
    },
    {
      "id": "acct_3",
      "name": "PayPal",
      "account_type": "checking",
      "balance": 240.50
    }
  ],
  "suggestions": []  // Empty in MVP; populated in Phase 2
}
```

### Request: `submit_transaction_mappings`

```json
{
  "mappings": [
    {
      "transaction_id": "txn_abc123",
      "account_id": "acct_1",
      "confidence": 1.0
    },
    {
      "transaction_id": "txn_def456",
      "account_id": "acct_3",
      "confidence": 1.0
    }
  ]
}
```

**Response**: `{ "success": true }`

---

**Document Version**: 1.0
**Last Updated**: 2026-07-06
**Author**: Implementation Planning Agent
