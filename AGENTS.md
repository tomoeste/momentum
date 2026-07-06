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
- **Tests**: `npm test` (Vitest - not yet configured)

## Database Testing

**Populate test data**:
- Database initializes automatically at `~/.config/momentum/momentum.db`
- Use sqlite3 CLI to insert test transactions for dashboard validation
- See `specs/01_accounts_schema.md` for schema details

**Sample SQLite commands**:
```sql
-- Insert test account
INSERT INTO accounts VALUES ('test_checking', 'simplefin_123', 'My Checking', 'checking', 'Bank', 5000.0, datetime('now'));

-- Insert test transaction
INSERT INTO raw_transactions VALUES 
  ('txn_1', 'test_checking', datetime('now'), 100.0, 'Employer', 'Paycheck', 'deposit', datetime('now'), 'simplefin');

-- Insert categorization
INSERT INTO categorized_transactions VALUES
  ('txn_1', 'Income', NULL, 1.0, NULL, datetime('now'), 0);
```

## Project Structure

- **Frontend**: `/src/` (React + TypeScript, compiled by Vite)
- **Backend**: `/src-tauri/src/` (Rust, compiled by rustc)
- **Database**: `~/.config/momentum/momentum.db` (SQLite, auto-initialized)
- **Specs**: `/specs/01-05.md` (frozen API contracts - source of truth)
- **Config**: `tauri.conf.json`, `vite.config.ts`, `tsconfig.json`, `tailwind.config.js`

## Codebase Patterns

- **Error handling**: AppError enum (Database, SimpleFin, Llm, Validation, Config, Internal, Keychain, NotFound)
- **Commands**: Async for I/O (SimpleFIN, metrics); sync for mutations; all return Result<T>
- **Database**: rusqlite with named params; COALESCE for null safety; indexes on frequently-filtered columns
- **Serialization**: serde (Rust) ↔ JSON ↔ TypeScript type mapping in tauri-commands.ts
- **Metrics**: SQL aggregations in db.rs; use ABS() for negative amounts (spending, interest); sparkline via recursive CTE