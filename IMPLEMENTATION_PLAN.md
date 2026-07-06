# Momentum Budgeting App - Implementation Roadmap

**Status**: Greenfield. Reprioritized to unblock backend/frontend parallelization.
**Legend**: `[SPEC]` = requires spec formalization before coding · `[BLOCKED:n]` = blocked by Gap n

---

## Spec Gaps (root blockers)

1. **No `accounts` table** — only `debt_accounts`; can't distinguish checking/savings/credit.
2. **Tauri Command API undefined** — command names exist, zero signatures.
3. **Opportunity-cost formula incomplete** — no amortization / per-account APR model.
4. **SimpleFIN auth wrong** — spec says "user/pass in keychain"; real flow is setup-token → access URL.
5. **APR data source undefined** — SimpleFIN gives balances, not APR; must be user input.

---

## SPRINT 0 — Spec Formalization (do FIRST; unblocks all parallel work)

> These are docs/contracts in `specs/`, not code. Completing them lets frontend +
> backend proceed simultaneously against a frozen contract.

- [ ] `[SPEC]` Define `accounts` schema + `account_type` enum values `[BLOCKED:1]`
  - id, simplefin_account_id, name, type(checking|savings|credit|loan), org, balance, last_updated
- [ ] `[SPEC]` Decide accounts vs debt_accounts: merge or FK relation `[BLOCKED:1]`
- [ ] `[SPEC]` Write Tauri command signatures (params, return, error enum) `[BLOCKED:2]`
  - Freeze JSON DTOs shared by Rust + TS (serde <-> TS types)
- [ ] `[SPEC]` Define AppError enum + Result contract for all commands `[BLOCKED:2]`
- [ ] `[SPEC]` Formalize amortization model: per-account APR payoff math `[BLOCKED:3,5]`
  - n = -ln(1 - r*P/PMT) / ln(1+r); aggregate across debt accounts
  - Define interest-saved = baseline_interest - accelerated_interest
- [ ] `[SPEC]` Rewrite SimpleFIN auth: setup-token claim -> access URL `[BLOCKED:4]`
  - Store single access URL (basic-auth embedded), not user/pass
- [ ] `[SPEC]` Define APR/min-payment as user-input fields + edit source `[BLOCKED:5]`
- [ ] `[SPEC]` Correct README lines 367 & 46 (auth) and line 105 (payoff math)

---

## CRITICAL PATH (strictly sequential; everything downstream waits)

### CP1 — Project skeleton
- [ ] Init Tauri project (src-tauri/, src/, public/) for macOS
- [ ] Configure tauri.conf.json, tsconfig.json, vite.config.ts
- [ ] TailwindCSS with dark/light mode
- [ ] Rust + Node toolchains; logging (Rust `tracing` + React)
- [ ] Create ~/.config/momentum/ data dir bootstrap

### CP2 — Database layer (needs Sprint-0 #1,#2)
- [ ] Create `raw_transactions` table + indexes (posted_date, account_id)
- [ ] Create `accounts` table + index (simplefin_account_id) `[BLOCKED:1]`
- [ ] Create `categorized_transactions` table (FK -> raw.id)
- [ ] Create `debt_accounts` table (interest_rate, minimum_payment user-set) `[BLOCKED:5]`
- [ ] Create `sync_log` table + index (sync_date)
- [ ] Migration/init system (Rust-side, versioned)
- [ ] Connection pool (rusqlite/sqlx) + query helpers (insert/update/read)
- [ ] Rust structs: Transaction, Account, DebtAccount, Categorized, SyncLog

### CP3 — Tauri command layer (needs Sprint-0 #3,#4; unblocks frontend real data)
- [ ] Implement command handlers per frozen signatures `[BLOCKED:2]`
  - get_dashboard_metrics, get_sparkline_data, sync_simplefin
  - get_transactions(filters), get_transaction_detail(id)
  - recategorize_transaction(...), get_opportunity_scenarios
  - get_sync_status, get_accounts, set_debt_terms(apr, min_payment) `[BLOCKED:5]`
- [ ] Result/AppError propagation + input validation
- [ ] Generate/write TS bindings `src/lib/tauri-commands.ts`

---

## PARALLEL TRACK A — Backend data pipeline (after CP2)

### SimpleFIN integration `[BLOCKED:4]`
- [ ] Setup-token claim flow -> obtain + store access URL (keychain)
- [ ] SimpleFIN client (reqwest) using access-URL basic auth
- [ ] Upsert accounts from /accounts response into `accounts` table `[BLOCKED:1]`
- [ ] Transaction parser (raw -> raw_transactions), sign convention
- [ ] Skip pending; dedupe by transaction id
- [ ] 90-day chunked backfill (Jan 1 2026 -> now) with checkpoints
- [ ] Daily delta sync + last-sync persistence + resume/error recovery

### LLM categorization engine
- [ ] Prompt design: merchant/desc -> primary category + confidence
- [ ] Secondary category when confidence >= 0.85; special cases (transfer/fee/interest)
- [ ] Ollama client (localhost:11434) with timeout + health check
- [ ] Claude API fallback with structured output (tool_use/JSON schema)
- [ ] Batch queue, retry, source tracking (ollama|api)
- [ ] Non-destructive "recategorize all" reprocessing

### Metrics engine
- [ ] Income = deposits to checking/savings accounts `[BLOCKED:1]`
- [ ] Spending = negative, excl Debt Payments/Transfers/Interest
- [ ] Interest paid, debt paydown (principal), debt ratio `[BLOCKED:1]`
- [ ] Interest as % of income
- [ ] Weekly/monthly aggregation + TTL cache (invalidate on sync)
- [ ] Sparkline series (last 28 days per metric)

### Opportunity-cost engine `[BLOCKED:3,5]`
- [ ] Amortization payoff per debt account (per-account APR)
- [ ] Scenario templates (-$200, -$500) months-saved + interest-saved
- [ ] Human-readable scenario output

---

## PARALLEL TRACK B — Frontend (starts once CP1 done + Sprint-0 #3 frozen)

> Build against mocked tauri-commands returning frozen DTOs; swap to real at CP3.

### Shell & dashboard
- [ ] App.tsx layout + Header (last-sync, settings, sync status)
- [ ] This Week / This Month toggle + period state
- [ ] MomentumCards grid container (responsive)
- [ ] Metric cards: Income, Spending, Debt Paydown, Interest, Debt Ratio
- [ ] Recharts sparkline (no axes; income=green/spend=red/debt=blue/interest=orange)
- [ ] Category breakdown (top 5-7, % of spend, click-to-filter)
- [ ] Interest Bleed alert card ($/day overhead, % income, severity color)
- [ ] Opportunity Cost card (scenario list, dynamic) `[BLOCKED:3]`
- [ ] Loading skeletons + error boundary + failed-sync message

### Transaction drill-down
- [ ] TransactionList view + filter header + search box
- [ ] Virtualized TransactionTable (sortable date/amount)
- [ ] Detail/recategorization modal (category, secondary, note, manual flag)
- [ ] Filters: date-range, category, account, type + reset `[BLOCKED:1]`
- [ ] Debounced case-insensitive search + match highlight
- [ ] Save handler -> recategorize command + toast
- [ ] "Recategorize all" bulk action (confirm + progress)

---

## PARALLEL TRACK C — Settings (after SimpleFIN + LLM land)

- [ ] SettingsModal shell + About section
- [ ] SimpleFIN section: setup-token paste + claim + Test Connection `[BLOCKED:4]`
- [ ] Debt terms editor: per-account APR + min-payment inputs `[BLOCKED:5]`
- [ ] LLM config: Ollama URL, API key, model, local-first toggle
- [ ] Sync settings: frequency, backfill range, manual sync button
- [ ] UI prefs: theme toggle, currency (USD default)
- [ ] Keychain store/retrieve access URL + API keys

---

## PARALLEL TRACK D — Sync orchestration & background (after Track A)

- [ ] Sync scheduler: on-open if >24h; optional background
- [ ] Sync workflow: creds -> fetch -> parse -> skip pending -> insert -> queue -> log
- [ ] Sync status: in-progress indicator + dashboard error display
- [ ] Retry with backoff; preserve partial data
- [ ] Background categorization queue (non-blocking) + result storage

---

## PARALLEL TRACK E — Testing (alongside implementation)

- [ ] Vitest setup for React components
- [ ] Unit tests: metrics (income/spend/debt/interest) + amortization math `[BLOCKED:3]`
- [ ] Unit tests: categorization output parsing
- [ ] DB integration tests (migrations, upserts)
- [ ] SimpleFIN sync test w/ mocked API `[BLOCKED:4]`
- [ ] Error-path tests (failed sync, API timeout)

---

## NICE-TO-HAVE (post-MVP; do not block release)

### Polish
- [ ] Query pagination / lazy detail loading
- [ ] Metric cache tuning; useMemo/useCallback on hot renders
- [ ] Keyboard shortcuts (Cmd+S sync) + a11y navigation
- [ ] WCAG AA contrast pass; metric-update transitions
- [ ] Tooltips for complex metrics
- [ ] DEVELOPMENT.md, command API docs, schema docs, troubleshooting

### Deployment
- [ ] Tauri macOS build: signing + notarization + DMG
- [ ] Test macOS 11-14; GitHub Actions CI
- [ ] Security audit (keychain, API keys, logs)
- [ ] Load test (thousands of transactions); release notes
- [ ] Auto-update (optional)

### Phase 2 features
- [ ] CSV/PDF export, custom categories, budget goals
- [ ] Recurring-txn detection, tagging, net-worth, multi-device sync, mobile

---

## Dependency Graph

```
SPRINT 0 (specs) ──┬──> CP2 (db, needs #1) ──> Track A ──> Track D
                   ├──> CP3 (commands, needs #2,#4) ──> real data
                   ├──> Opp-cost + Track E math (needs #3,#5)
                   └──> Track C settings (needs #4,#5)

CP1 (skeleton) ──> Track B frontend (mock DTOs) ──[CP3]──> live wiring
```

**Hard blockers**: Sprint 0 -> CP2 -> CP3 -> live frontend data.
**Parallelizable after CP1 + frozen contract**: Track B (mocks) ∥ Tracks A/C/D/E.

---

## Critical Files

- `/work/specs/` — Sprint-0 contracts (schema, command API, amortization, auth)
- `/work/src-tauri/src/{main,db,simplefin,llm}.rs`
- `/work/src/App.tsx`, `/work/src/components/{Dashboard,TransactionList}.tsx`
- `/work/src/lib/{calculations,tauri-commands}.ts`
