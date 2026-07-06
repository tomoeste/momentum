# Momentum Budgeting App - Implementation Plan

**Status**: Greenfield project, initialization phase

---

## Phase 1: Foundation

### Infrastructure & Setup
- [ ] Initialize Tauri project structure (src-tauri/, src/, public/)
- [ ] Configure Tauri build settings (tauri.conf.json) for macOS
- [ ] Set up TypeScript/React project configuration (tsconfig.json, vite.config.ts)
- [ ] Configure TailwindCSS with dark/light mode support
- [ ] Set up development environment (Rust toolchain, Node dependencies)
- [ ] Create project directory structure for configs/database (~/.config/momentum/)
- [ ] Configure environment variables handling for SimpleFIN and LLM endpoints
- [ ] Set up logging and error tracking (both Rust and React sides)

### Database Schema & Initialization
- [ ] Create SQLite schema (raw_transactions table)
  - Columns: id, account_id, account_name, posted_date, amount, merchant, description, transaction_type, imported_at, source
  - Indexes on: id (PK), posted_date, account_id
- [ ] Create categorized_transactions table
  - Columns: id, category, secondary_category, confidence, note, categorized_at, is_manual
  - Foreign key to raw_transactions.id
- [ ] Create debt_accounts table
  - Columns: id, simplefin_account_id, account_name, account_type, current_balance, interest_rate, minimum_payment, last_updated
- [ ] Create sync_log table
  - Columns: id, sync_date, status, transaction_count, error_message, duration_ms
  - Index on: sync_date (for recent sync queries)
- [ ] Implement database initialization/migration system (Rust-side)
- [ ] Set up secure storage for SimpleFIN credentials (OS keychain integration)

### Backend Utilities & Models
- [ ] Implement Rust data structures for Transaction, CategorizedTransaction, DebtAccount, SyncLog
- [ ] Create database connection pool (sqlx or rusqlite)
- [ ] Implement database query helpers (insert, update, read operations)
- [ ] Set up error handling and logging infrastructure (Rust)

---

## Phase 2: Backend Core

### SimpleFIN Integration (Foundational)
- [ ] Implement SimpleFIN API client (HTTP requests via reqwest)
  - Authentication handling (username/password from keychain)
  - Chunked 90-day backfill logic
  - Daily delta sync logic
- [ ] Create transaction parser (SimpleFIN raw → raw_transactions table)
  - Parse posted_date (ISO 8601)
  - Calculate amount sign convention (positive=income, negative=spend)
  - Extract merchant/description
  - Detect pending vs posted (skip pending)
- [ ] Implement sync state tracking
  - Last sync timestamp persistence
  - Resume logic for interrupted syncs
  - Error recovery (graceful degradation)
- [ ] Implement first-run backfill (Jan 1, 2026 → present)
  - Chunk API calls in 90-day windows
  - Store checkpoint to avoid re-fetching
  - Display progress to user

### LLM Categorization Engine (Foundational)
- [ ] Create LLM prompt engineering for categorization
  - Map merchant/description → primary category
  - Confidence scoring (0-1 scale)
  - Secondary category inference (if confidence ≥0.85)
  - Handle special cases (transfers, fees, interest, debt payments)
- [ ] Implement Ollama integration (HTTP API to localhost:11434)
  - Model selection (mistral or similar)
  - Timeout and fallback logic
  - Streaming vs batch processing decision
- [ ] Implement Claude API fallback
  - Use structured output (tool_use or JSON schema)
  - API key from environment/config
  - Same categorization output format as Ollama
- [ ] Create categorization queue system
  - Background processing (doesn't block dashboard)
  - Batch processing (efficient API calls)
  - Retry logic for failed categorizations
- [ ] Implement reprocessing system
  - Allow "recategorize all" after prompt refinement
  - Preserve raw transactions (non-destructive)
  - Track which transactions were auto vs manual

### Calculations & Metrics Engine
- [ ] Implement metric calculation module
  - Income calculation (sum positive, category = Income)
  - Spending calculation (sum negative, exclude Debt Payments/Transfers/Interest)
  - Interest paid (sum of Interest category transactions)
  - Debt paydown (sum of Debt Payments category, principal only)
  - Debt ratio (total debt / 3-month avg income)
  - Interest as % of income
- [ ] Implement weekly and monthly aggregation
  - Query transactions for past 7 days and past 30 days
  - Cache metric results (TTL-based invalidation)
- [ ] Implement sparkline data generation
  - Last 28 days of each metric (daily aggregation)
  - Trend data for dashboard visualization

### Opportunity Cost Scenarios Engine
- [ ] Implement scenario calculation logic
  - For each reduction amount ($200, $500, etc.)
  - Calculate months to payoff: total_debt / (current_monthly_payment + reduction)
  - Calculate interest saved
  - Generate human-readable output
- [ ] Create scenario templates (default: -$200, -$500)
- [ ] Implement future interest calculation (APR-based)

### Tauri Command Layer
- [ ] Implement Tauri commands:
  - `get_dashboard_metrics()` → returns income, spending, debt paydown, interest, debt ratio
  - `get_sparkline_data(metric: string)` → returns 28-day trend data
  - `sync_simplefin()` → triggers sync, returns status + count
  - `get_transactions(filters)` → returns filtered transaction list with categories
  - `get_transaction_detail(id)` → returns full transaction + categorization
  - `recategorize_transaction(id, category, secondary, is_manual)` → updates categorized_transactions
  - `get_opportunity_scenarios()` → returns scenario projections
  - `get_sync_status()` → returns last sync timestamp + status
- [ ] Implement error handling in commands (return Result types)
- [ ] Implement request validation and sanitization

---

## Phase 3: Frontend Core

### Component Architecture
- [ ] Create App.tsx root component with layout structure
- [ ] Implement Header component
  - Last sync timestamp display
  - Settings/config button
  - Sync status indicator
- [ ] Implement time period toggle (This Week / This Month)
  - State management for selected period
  - Trigger metric recalculation on toggle
- [ ] Create MomentumCards component (grid container)
  - Card styling/layout
  - Responsive design

### Dashboard Metric Cards
- [ ] Implement Income Card
  - Display formatted amount ($X)
  - Embed Sparkline component (28-day trend)
  - Click to drill-down to transactions
- [ ] Implement Spending Card
  - Display formatted amount ($X)
  - Embed Sparkline component
  - Add category breakdown (top 5-7 by amount)
  - Click breakdown to filter/drill-down
- [ ] Implement Debt Paydown Card
  - Display formatted amount ($X)
  - Embed Sparkline component
  - Click to show debt payment transactions
- [ ] Implement Interest Paid Card
  - Display formatted amount ($X)
  - Embed Sparkline component
  - Click to show interest transactions
- [ ] Implement Debt Ratio Card
  - Display ratio value (X.XX)
  - Embed Sparkline component
  - Show current debt balance

### Interest Bleed Alert Card
- [ ] Create AlertCard component
  - Display: "Interest this month: $X (Y% of your monthly income)"
  - Display: "That's $Z per day in overhead."
  - Conditional styling (red/yellow based on severity)
  - Click to drill-down to interest transactions

### Opportunity Cost Scenarios Card
- [ ] Create OpportunityCostCard component
  - Display scenario list (e.g., $200/mo, $500/mo)
  - For each scenario show:
    - Months to payoff
    - Interest saved
    - Visual comparison
  - Update dynamically when debt/income changes

### Category Breakdown Component
- [ ] Create CategoryBreakdown component
  - Display top 5-7 categories (sortable by amount)
  - Show amount and % of total spending
  - Clickable categories to filter transactions
  - Responsive layout (stack on mobile)

### Sparkline Integration
- [ ] Set up Recharts LineChart for sparklines
  - Minimal styling (no axes, legend, etc.)
  - Responsive sizing
  - Data point tooltips on hover
  - Color scheme (income=green, spending=red, debt=blue, interest=orange)

### Loading & Error States
- [ ] Create loading skeleton components for each card
- [ ] Implement error boundary component
- [ ] Add error message display for failed syncs
- [ ] Show "Loading..." state while metrics are fetching

---

## Phase 4: Transaction Features

### Transaction Drill-Down View
- [ ] Create TransactionList component (main view)
  - Header with filters (Date Range, Category, Account)
  - Search box (merchant/description)
  - Transaction table/list
- [ ] Implement TransactionTable component
  - Columns: Date, Merchant, Amount, Account, Category, Confidence
  - Sortable columns (date, amount)
  - Row click → detail view/recategorization modal
  - Virtualized list (for performance with large datasets)
- [ ] Create transaction detail modal
  - Full transaction info (id, merchant, description, amount, posted_date, account)
  - Current category + secondary category + confidence
  - Manual recategorization UI (dropdown selectors)
  - Note field display
  - Manual flag indicator
  - Save/cancel buttons

### Filter System
- [ ] Implement date range filter (picker or preset: "This Week", "This Month", "Last 90 days")
- [ ] Implement category filter (dropdown, multi-select option)
- [ ] Implement account filter (dropdown, multi-select option)
- [ ] Implement transaction type filter (income/spend/payment)
- [ ] Create filter state management (URL params or local state)
- [ ] Implement filter reset functionality

### Search Functionality
- [ ] Implement real-time search (merchant/description)
  - Debounced input handling
  - Case-insensitive matching
  - Highlight matching text in results
- [ ] Filter results as user types

### Recategorization Modal
- [ ] Create category selector (dropdown with all primary categories)
- [ ] Create secondary category selector (conditional, only if applicable)
- [ ] Add note field (optional, for user reasoning)
- [ ] Show current confidence score
- [ ] Show "Manual" flag checkbox (auto-check on change)
- [ ] Implement save handler (calls Tauri command)
- [ ] Show success/error toast after save

### Bulk Recategorization
- [ ] Implement "Recategorize all" button (appears in Transaction List)
  - Shows confirmation dialog
  - Triggers reprocessing of all uncategorized transactions
  - Shows progress indicator
  - Displays results summary after completion

---

## Phase 5: Settings & Configuration

### Settings Panel
- [ ] Create SettingsModal component
- [ ] Implement SimpleFIN credentials section
  - Username input (masked)
  - Password input (masked)
  - Test connection button
  - Save credentials securely (via Tauri command to keychain)
- [ ] Implement LLM configuration section
  - Ollama endpoint URL (default: localhost:11434)
  - API key input for Claude/OpenAI fallback
  - Model selection (for Ollama)
  - Preference toggle (local first vs API first)
- [ ] Implement sync settings
  - Sync frequency (daily, manual, etc.)
  - Backfill date range selector
  - Manual sync button
- [ ] Implement UI preferences
  - Theme toggle (light/dark)
  - Currency selection (default: USD)
- [ ] About section (version, links)

### Secure Credential Storage
- [ ] Implement keychain integration (Tauri built-in)
  - Store SimpleFIN username/password
  - Retrieve on app startup
  - Use in SimpleFIN API calls

---

## Phase 6: Sync & Background Processing

### Sync Orchestration
- [ ] Implement sync scheduler
  - Daily check on app open (if last sync >24h ago)
  - Optional background sync (if app running in background)
- [ ] Create sync workflow
  - Check credentials (fail gracefully if missing)
  - Fetch transactions from SimpleFIN
  - Parse raw transactions
  - Skip pending transactions
  - Insert into raw_transactions table
  - Queue for categorization
  - Log sync result
  - Update dashboard UI on completion
- [ ] Implement sync status tracking
  - Last sync timestamp display on dashboard
  - Sync in progress indicator
  - Error message display on dashboard
- [ ] Implement error recovery
  - Retry failed syncs (with backoff)
  - Show user-friendly error messages
  - Preserve partial data (don't lose transactions on partial failure)

### Background Categorization
- [ ] Implement background task queue
  - Queue uncategorized transactions
  - Process in batches (efficient API calls)
  - Don't block dashboard render
  - Show "Categorizing..." indicator
- [ ] Implement Ollama categorization worker
  - Check local availability (HTTP health check)
  - Batch requests (multiple transactions per call)
  - Handle timeout (fallback to API)
  - Retry logic
- [ ] Implement API categorization fallback
  - Queue transactions if Ollama fails
  - Process via Claude/OpenAI API
  - Use structured output
  - Rate limit to avoid quota issues
- [ ] Implement categorization result storage
  - Update categorized_transactions table
  - Store confidence, category, secondary_category, note
  - Track source (ollama vs api)

---

## Phase 7: Polish & Testing

### Performance Optimization
- [ ] Implement transaction query pagination (virtualized list rendering)
- [ ] Optimize database queries (indexes on frequently queried columns)
- [ ] Implement metric calculation caching (invalidate on sync)
- [ ] Optimize React re-renders (useMemo, useCallback)
- [ ] Implement lazy loading for transaction details
- [ ] Optimize Tauri command performance (batch operations)

### Error Handling & Resilience
- [ ] Add error boundaries around major components
- [ ] Implement graceful degradation (show last known data if sync fails)
- [ ] Add timeout handling (SimpleFIN, Ollama, API calls)
- [ ] Implement offline mode (show cached metrics)
- [ ] Add user-friendly error messages (non-technical language)
- [ ] Log errors for debugging (Rust + React logs)

### Testing
- [ ] Set up Jest/Vitest for React component tests
- [ ] Create unit tests for calculations (income, spending, debt, interest)
- [ ] Create unit tests for categorization logic
- [ ] Create integration tests for database operations
- [ ] Test SimpleFIN sync flow (mock API)
- [ ] Test categorization queue and background processing
- [ ] Test error scenarios (failed sync, API timeout, etc.)

### UI/UX Refinements
- [ ] Ensure responsive design (works on different screen sizes)
- [ ] Add keyboard shortcuts (e.g., Cmd+S for sync)
- [ ] Implement keyboard navigation (accessibility)
- [ ] Test dark/light mode consistency
- [ ] Add loading states for all async operations
- [ ] Add tooltips for complex metrics
- [ ] Ensure color contrast (WCAG AA minimum)
- [ ] Add animations for metric updates (subtle transitions)

### Documentation & Onboarding
- [ ] Create DEVELOPMENT.md (setup, build, run)
- [ ] Document API/command interfaces (Tauri)
- [ ] Document database schema (with examples)
- [ ] Add inline code comments for complex logic
- [ ] Create troubleshooting guide (common issues)

---

## Phase 8: Deployment & Release

### Build & Packaging
- [ ] Configure Tauri build for macOS (signing, notarization)
- [ ] Create installer/DMG for macOS
- [ ] Test on multiple macOS versions (10.15+, 11, 12, 13, 14)
- [ ] Set up GitHub Actions for CI/CD builds
- [ ] Create auto-update mechanism (optional for MVP)

### Launch Preparation
- [ ] Final security audit (keychain, API keys, error logs)
- [ ] Performance profiling (dashboard load time, sync duration)
- [ ] Load testing (sync with thousands of transactions)
- [ ] User acceptance testing
- [ ] Create release notes

---

## Phase 2 (Nice-to-Have) - Not in MVP

### Advanced Features
- [ ] Export to CSV/PDF functionality
- [ ] Custom category definitions (user-defined categories)
- [ ] Budget goal setting and tracking
- [ ] Recurring transaction detection (pattern matching)
- [ ] Multi-device cloud sync (encrypted backup)
- [ ] Mobile app version (iOS/Android)
- [ ] Transaction tagging system (beyond categories)
- [ ] Savings goal tracking
- [ ] Net worth tracking (assets + liabilities)
- [ ] Reporting dashboard (custom date ranges, custom reports)

---

## Critical Dependencies & Sequencing

### Hard Blockers
- **Database schema** → all data operations
- **SimpleFIN API client** → sync operations
- **LLM categorization** → transaction drill-down (need categories)
- **Tauri command layer** → all backend-frontend communication
- **Dashboard layout** → all metric components

### Soft Blockers
- **Error boundaries** → testing & polish
- **Settings panel** → requires working sync + categorization
- **Background processing** → requires categorization infrastructure

### Parallel Tracks
- **Frontend**: UI components can be built while backend is in progress (with mock data)
- **Testing**: Unit tests can be written alongside implementation
- **Database**: Schema creation can happen immediately

---

## Critical Files for Implementation

- `/work/src-tauri/src/main.rs` - Tauri app entry point, command handlers
- `/work/src-tauri/src/db.rs` - Database initialization, schema, connection pool
- `/work/src-tauri/src/simplefin.rs` - SimpleFIN API client, sync logic
- `/work/src-tauri/src/llm.rs` - LLM categorization engine (Ollama + API fallback)
- `/work/src/App.tsx` - React app root, layout structure
- `/work/src/components/Dashboard.tsx` - Main dashboard view with metric cards
- `/work/src/components/TransactionList.tsx` - Transaction drill-down with filters
- `/work/src/lib/calculations.ts` - Metric calculation helpers (frontend)
- `/work/src/lib/tauri-commands.ts` - Tauri command TypeScript bindings

---

## Next Steps

**Immediate**: Initialize Tauri project structure and set up basic configurations (Phase 1)

**Then**: Build database layer and SimpleFIN integration (Phase 2) to get data flowing

**Then**: Implement LLM categorization engine with local/remote fallback

**Then**: Build frontend dashboard components with real data

**Finally**: Add transaction drill-down, settings, and polish for release
