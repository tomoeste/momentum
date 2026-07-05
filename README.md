# Momentum

## Vision
A local-first budgeting app that shows real cash flow momentum and the actual cost of debt through interest visibility. Think Hume's metabolic score approach applied to personal finances: deltas, ratios, and signals that tell you whether you're winning right now, not prescriptive budget goals.

---

## Core Features

### 1. Dashboard: The Momentum View
**Primary view—what you see when you open the app.**

#### Key Metrics (Week + Month View)
- **Income**: Total deposits to checking/savings
- **Spending**: CC charges + cash/check transactions (broken down by category)
- **Debt Paydown**: Principal paid to credit cards
- **Interest Paid**: Interest charges on all debt (separate line item for visibility)
- **Debt Ratio**: Total debt / monthly average income

#### Visual Elements
- **Sparklines**: 4-week trend for each metric (income, spending, debt paydown, interest paid)
- **Interest Bleed Card**: 
  - "Interest this month: $X (Y% of your monthly income)"
  - This is the gut-punch metric. It's real opportunity cost.
- **Category Breakdown**: Top 5-7 spending categories this month, sortable by amount
- **Opportunity Cost Insight**:
  - "If you cut discretionary by $X/month and threw it at debt, you'd save $Y in future interest and pay off debt Z months faster"
  - Show 2-3 scenarios: -$200/mo, -$500/mo, etc.

### 2. Transaction Drill-Down
**Click into any metric or category to see the actual transactions.**

- **Filterable List**:
  - Date, merchant/description, amount, account, category (with confidence score)
  - Filter by: date range, category, account, transaction type (income/spend/payment)
- **Manual Recategorization**: Click category to change it (updates the transaction record but preserves confidence score for auditing)
- **Search**: Quick merchant/description search

### 3. SimpleFIN Integration
**Automated transaction import with intelligent polling.**

#### Sync Strategy
- **Frequency**: Daily, either background (if app is running) or on-open (if last sync was >24h ago)
- **Backfill**: On first launch, pull 6 months of data (Jan 1, 2026 → present) via 90-day chunks
- **Account Coverage**: All checking, savings, and credit card accounts from Chase + BoA
- **Error Handling**: Graceful degradation if sync fails; show last-sync timestamp on dashboard

#### SimpleFIN Flow
1. Fetch transactions (90-day chunks for backfill, then daily delta)
2. Parse into raw transaction format (see Data Model)
3. Detect pending transactions vs. posted (skip pending)
4. Queue for LLM categorization if uncategorized
5. Update dashboard metrics

### 4. LLM Categorization
**Intelligent, iterative transaction labeling.**

#### Local-First with API Fallback
- **Primary**: Ollama local inference (your 16GB Mac setup)
- **Fallback**: API-based (Anthropic/OpenAI/etc.) if local is slow or unavailable
- **Processing**: Runs in background after sync; doesn't block dashboard update

#### Category Taxonomy

**Primary Categories** (what the LLM targets—broad, high-confidence buckets):
- **Income**: Salary, bonus, transfer-in, other deposits
- **Groceries**: Supermarket, specialty food, bulk/wholesale
- **Dining Out**: Restaurants, coffee shops, food delivery
- **Transportation**: Gas, maintenance, repairs, parking, tolls, public transit
- **Utilities**: Electric, water, internet, phone, natural gas
- **Home & Property**: Rent/mortgage, repairs, maintenance, improvements, equipment (tools, mower, chipper, tractor), homesteading supplies (bee gear, chicken feed, etc.)
- **Subscriptions**: Software, entertainment, fitness, recurring services
- **Shopping**: Clothing, household goods, online retail, misc
- **Healthcare**: Medical, doctor visits, pharmacy, insurance
- **Personal Care**: Haircut, salon, supplements, wellness, personal hygiene
- **Entertainment**: Hobbies, events, books, media
- **Transfers**: Internal account transfers (ignore for momentum)
- **Interest**: Interest charges on debt
- **Debt Payments**: Credit card/loan payments (principal paydown)
- **Uncategorized**: Confidence too low or ambiguous

**Secondary Categories** (optional—LLM only adds if confident, enables richer reporting later):
```
Groceries
├─ Supermarket
├─ Specialty Food
└─ Bulk/Wholesale

Dining Out
├─ Restaurants
├─ Coffee
└─ Food Delivery

Transportation
├─ Gas/Fuel
├─ Maintenance/Repairs
├─ Parking/Tolls
└─ Public Transit

Home & Property
├─ Rent/Mortgage
├─ Repairs/Maintenance
├─ Improvements
├─ Equipment (tools, mower, chipper, tractor, etc.)
├─ Homesteading (bee supplies, chicken feed, etc.)
└─ Utilities

Subscriptions
├─ Software
├─ Entertainment
├─ Fitness
└─ Services

Shopping
├─ Clothing
├─ Household
├─ Online Retail
└─ Misc

Healthcare
├─ Medical/Doctor
├─ Pharmacy
├─ Insurance
└─ Medical Supplies

Entertainment
├─ Hobbies
├─ Events
└─ Books/Media
```

#### Categorization Output
```json
{
  "category": "Groceries",
  "secondary_category": "Specialty Food",
  "confidence": 0.95,
  "note": "Whole Foods transaction"
}
```

**Note**: `secondary_category` is optional. Only include if confidence is high (0.85+) and it adds meaningful detail.

**Confidence Scoring**:
- **0.9+**: High confidence, likely correct
- **0.7-0.89**: Medium, reasonable guess
- **<0.7**: Low, might want to review or recategorize

**Special Cases**:
- Transfers between your own accounts → skip (don't count as spending)
- SimpleFIN fees → "Uncategorized" or flag for manual review
- Subscriptions → LLM should recognize patterns and mark as "Subscriptions"

#### Reprocessing
- Allow bulk "recategorize all" if you refine the LLM prompt
- Raw transactions stored separately, so categorization is non-destructive

---

## Data Model

### Raw Transactions Table
```sql
CREATE TABLE raw_transactions (
  id TEXT PRIMARY KEY,  -- SimpleFIN transaction ID
  account_id TEXT,
  account_name TEXT,
  posted_date TEXT,     -- ISO 8601
  amount REAL,          -- Positive for income, negative for spend
  merchant TEXT,
  description TEXT,
  transaction_type TEXT, -- "debit", "credit", "check", "electronic", etc.
  imported_at TEXT,     -- When we fetched it from SimpleFIN
  source TEXT           -- "simplefin"
);
```

### Categorized Transactions Table
```sql
CREATE TABLE categorized_transactions (
  id TEXT PRIMARY KEY,  -- Foreign key to raw_transactions.id
  category TEXT,        -- Primary category (required)
  secondary_category TEXT, -- Optional secondary category for richer reporting
  confidence REAL,      -- 0-1 scale
  note TEXT,            -- Optional LLM reasoning
  categorized_at TEXT,  -- ISO 8601
  is_manual BOOLEAN     -- True if user recategorized
);
```

### Debt Accounts Table
```sql
CREATE TABLE debt_accounts (
  id TEXT PRIMARY KEY,
  simplefin_account_id TEXT,
  account_name TEXT,
  account_type TEXT,    -- "credit_card", "loan", etc.
  current_balance REAL,
  interest_rate REAL,   -- APR as decimal (0.2199 = 21.99%)
  minimum_payment REAL, -- If known
  last_updated TEXT
);
```

### Syncs Log Table
```sql
CREATE TABLE sync_log (
  id INTEGER PRIMARY KEY,
  sync_date TEXT,
  status TEXT,          -- "success", "partial", "failed"
  transaction_count INTEGER,
  error_message TEXT,
  duration_ms INTEGER
);
```

---

## Key Calculations

### Income (Monthly & Weekly)
```
SUM(amount) WHERE amount > 0 AND category IN ("Income", ...)
```

### Spending (Monthly & Weekly)
```
SUM(amount) WHERE amount < 0 AND category NOT IN ("Debt Payments", "Transfers", "Interest")
Note: Interest and debt payments tracked separately
```

### Interest Paid (Monthly & Weekly)
```
ABS(SUM(amount)) WHERE category = "Interest"
```

### Debt Paydown (Monthly & Weekly)
```
ABS(SUM(amount)) WHERE category = "Debt Payments"
Exclude interest, track principal only
```

### Debt Ratio
```
Total Debt Balance / (Monthly Average Income over last 3 months)
```

### Interest as % of Income
```
(Interest Paid This Month / Monthly Average Income) * 100
```

### Opportunity Cost Scenarios
```
For each scenario reduction ($X/month to debt):
  months_to_payoff = total_debt / (current_monthly_payment + X)
  total_interest_saved = (months_saved * monthly_interest) + future_interest_avoided
```

---

## UI/UX Architecture

### Main Dashboard
```
Header: Last Sync: [timestamp] | Settings

[This Week / This Month] toggle

MOMENTUM CARDS (in a grid or column):
├─ Income: $X [sparkline trend]
├─ Spending: $X [sparkline trend] [breakdown: categories]
├─ Debt Paydown: $X [sparkline trend]
├─ Interest Paid: $X/month [sparkline trend]
└─ Debt Ratio: X.XX [sparkline trend]

INTEREST BLEED ALERT:
┌─────────────────────────────────────────┐
│ Interest this month: $X (Y% of income)  │
│ That's $Z per day in overhead.          │
└─────────────────────────────────────────┘

OPPORTUNITY COST SCENARIOS:
┌──────────────────────────────────────────────┐
│ If you cut discretionary by:                 │
│ • $200/mo → pay off debt 2 months faster     │
│ • $500/mo → pay off debt 5 months faster     │
│           save $XX in interest               │
└──────────────────────────────────────────────┘

CATEGORY BREAKDOWN (This Month):
[Category] [Amount] [% of spending]
```

### Transaction Drill-Down
```
Header: [Back] [Date Range: This Month ▼] [Category: All ▼] [Account: All ▼]

[Search box: "amazon"]

Transaction List:
Date        | Merchant           | Amount  | Account    | Category      | Conf.
─────────────────────────────────────────────────────────────────────────────
2024-06-15  | Whole Foods        | -$87.42 | Chase CC   | Groceries    | 0.98
2024-06-14  | Amazon             | -$34.99 | Chase CC   | Shopping     | 0.92
2024-06-10  | Interest Charge    | -$42.16 | Chase CC   | Interest     | 1.00
            [click row to see details / recategorize]
```

---

## Technical Stack

### Frontend
- **Tauri**: Desktop app shell (macOS first, Windows later)
- **React/TypeScript**: UI framework
- **TailwindCSS**: Styling
- **Recharts**: Sparklines and basic charts

### Backend
- **Tauri Commands**: Rust backend for file I/O, SimpleFIN API calls, LLM inference
- **SQLite**: Local transaction database (via rusqlite or sqlx)
- **SimpleFIN SDK/HTTP**: Transaction fetching (REST API via reqwest or similar)
- **Ollama HTTP API**: Local LLM categorization (fallback to API key for remote)

### LLM Integration
- **Local**: Ollama running on localhost:11434 (mistral or similar fast model)
- **API Fallback**: Claude API with structured output (via tool_use or prompt engineering)
- **Structured Output**: JSON schema for category + confidence + note

### Storage
- SQLite database stored in `~/.config/simplefin-budgeting/` or similar
- Raw SimpleFIN responses cached briefly (for debugging, optional)

---

## Phase 1 (MVP) Scope
- Dashboard with core metrics (income, spending, debt paydown, interest)
- SimpleFIN sync (daily, backfill to Jan 1)
- LLM categorization (local Ollama first, fallback to API)
- Transaction drill-down with filters
- Manual recategorization
- Sparklines for 4-week trends
- Interest bleed visibility
- Basic opportunity cost scenarios

### Phase 2 (Nice-to-Have)
- Export/reporting (CSV, PDF)
- Custom category definitions
- Budget goals (if you change your mind)
- Recurring transaction detection
- Multi-device sync (encrypted cloud backup)
- Mobile app version

---

## Success Criteria
- **Speed**: Dashboard loads in <1s, sync completes in <5s
- **Accuracy**: 90%+ of transactions correctly categorized on first pass
- **Trust**: You feel comfortable using this as your single source of truth for cash flow
- **Behavior Change**: Interest bleed metric actually makes you reconsider spending decisions

---

## Notes for Implementation
- Start with UI mockups of the dashboard—this is the thing you'll stare at daily
- SimpleFIN auth (username/password) stored securely in OS keychain
- Error boundaries everywhere; sync failures shouldn't crash the app
- Consider a "last 24h" transaction cache to minimize API calls
- Categorization confidence threshold: only show recategorization UI if <0.8
- Sparkline data: pull last 28 days of each metric for consistency
