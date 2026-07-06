## Build & Run

**Setup**:
```bash
# Install dependencies
npm install
rustup install stable 2>/dev/null || true  # Install Rust if not present

# Ensure C compiler is available (required for rusqlite, libc, etc.)
# macOS: xcode-select --install
# Ubuntu/Debian: apt-get install -y gcc g++ make pkg-config build-essential
# Fedora: dnf install gcc g++ make pkgconfig
```

**Build**:
- Frontend: `npm run build` (transpiles TypeScript + Tailwind)
- Type check: `npx tsc --noEmit` (no compilation, type validation only)
- Rust check: `cargo check -p momentum` (type check, no linking) in src-tauri/
- Full build: `npm run tauri build` (requires C compiler + Rust toolchain)

**Development**:
```bash
npm run dev             # Start Vite dev server (port 5173)
# In another terminal:
cargo tauri dev         # Launch dev app with hot reload (requires Rust toolchain)
```

## Validation

- **Typecheck**: `npx tsc --noEmit` (runs in any environment; validates TypeScript)
- **Rust check**: `cargo check` (needs Rust toolchain; validates without linking)
- **Frontend tests**: `npm test` (Vitest configured; runs src/test/*.test.ts)
- **Backend tests**: `cargo test` (runs Rust unit tests in each module)

## Database Testing

**Populate test data**:
- Database initializes automatically at `~/.config/momentum/momentum.db`
- Run `sync` command to fetch accounts and auto-categorize transactions using LLM
- View categorized results in TransactionList UI component with filtering and recategorization
- See `specs/01_accounts_schema.md` for schema details

**Transaction categorization**:
- Transactions are auto-categorized during sync using Ollama LLM with confidence scores
- Confidence < 1.0 indicates LLM-assigned category; recategorize_transaction sets confidence to 1.0 (user override)
- Query `categorized_transactions` table to inspect category labels and confidence values

**Sample SQLite commands**:
```sql
-- Insert test account
INSERT INTO accounts VALUES ('test_checking', 'simplefin_123', 'My Checking', 'checking', 'Bank', 5000.0, datetime('now'));

-- Insert test transaction
INSERT INTO raw_transactions VALUES 
  ('txn_1', 'test_checking', datetime('now'), 100.0, 'Employer', 'Paycheck', 'deposit', datetime('now'), 'simplefin');

-- Check LLM-categorized transaction
SELECT id, category, confidence FROM categorized_transactions WHERE id = 'txn_1';
```

## Project Structure

- **Frontend**: `/src/` (React + TypeScript, compiled by Vite)
- **Backend**: `/src-tauri/src/` (Rust, compiled by rustc)
- **Database**: `~/.config/momentum/momentum.db` (SQLite, auto-initialized)
- **Specs**: `/specs/01-05.md` (frozen API contracts - source of truth)
- **Config**: `tauri.conf.json`, `vite.config.ts`, `tsconfig.json`, `tailwind.config.js`

## Codebase Patterns

- **Error handling**: AppError enum (Database, SimpleFin, Llm, Validation, Config, Internal, Keychain, NotFound)
- **SimpleFIN Integration**: Setup token must be base64-encoded; decoded to claim URL → POST to URL → receive access_url; stored securely in OS keychain
- **Commands**: Async for I/O (SimpleFIN, metrics); sync for mutations; all return Result<T>
- **Database**: rusqlite with named params; COALESCE for null safety; indexes on frequently-filtered columns
- **Serialization**: serde (Rust) ↔ JSON ↔ TypeScript type mapping in tauri-commands.ts
- **Metrics**: SQL aggregations in db.rs; use ABS() for negative amounts (spending, interest); sparkline via recursive CTE
- **LLM Categorization**: Ollama (localhost:11434) with JSON-based prompts for auto-categorizing transactions during sync; confidence scores track user overrides vs. LLM assignments
- **Settings Persistence**: localStorage for UI preferences (view toggle, filters); keychain for sensitive credentials (SimpleFIN access_url, LLM API keys)
- **Transaction Management**: getTransactions command supports filtering by account/category/date; recategorize_transaction for user overrides (sets confidence=1.0)
- **View System**: App.tsx toggles between Dashboard and Transaction List views; TransactionList component provides filtering and recategorization UI
- **Error Recovery**: Sync errors displayed prominently with retry button; users can immediately retry failed operations without re-entering credentials

## Test Infrastructure

**HTTP Mocking with httpmock:**
- SimpleFIN API tests use MockServer (localhost mock) for HTTP simulation
- LLM tests mock both Ollama and Claude API endpoints
- Location: src-tauri/src/simplefin.rs::integration_tests, src-tauri/src/llm.rs::integration_tests
- Pattern: MockServer::start(), mock(|when, then| {...}), then execute test

**Database Testing:**
- In-memory SQLite (":memory:") for test isolation
- Location: src-tauri/src/db_integration_tests.rs
- Setup helpers: create_test_account, create_test_transaction, create_test_categorization
- 22 tests cover CRUD, constraints, aggregation, pagination

**React Component Testing:**
- Vitest + React Testing Library configuration in vite.config.ts
- Mocked Tauri API in src/test/setup.ts
- 49 component tests for Header, TransactionList, SettingsModal
- Pattern: render component, mock API responses, assert on DOM

**Test Organization:**
- Unit tests: calculation logic, token validation
- Integration tests: HTTP mocking for external APIs
- Database tests: constraint validation, CRUD operations
- Component tests: React behavior and user interactions
- Total: 158 tests (61 lib + 39 main + 58 TS), 0 warnings

## Testing Checklist

- [ ] LLM categorization works (check Tauri logs for ollama requests; verify categorized_transactions has reasonable categories)
- [ ] Settings UI persists choices to localStorage (change view, refresh, verify state restored)
- [ ] Transaction List filters work correctly (filter by account/category, verify results)
- [ ] Recategorization updates confidence to 1.0 (user override via TransactionList)
- [ ] All 6 settings tabs render without errors (General, Account, LLM, SimpleFIN, Security, Advanced)