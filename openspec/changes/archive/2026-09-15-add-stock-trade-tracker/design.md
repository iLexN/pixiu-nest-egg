## Context

See `proposal.md` — Why. Current state relevant to the approach:

- The repository is greenfield: no commits, no source code, only `財富分析報告.xlsx` and OpenSpec/skill config. So every convention here is being established by this change.
- `港股Trade` columns: `A 股票代碼`(中文名) `B 股數` `C buy unit price =D/B` `D buy total` `E 單價` `F =E*B` `G fee =D-F` `H 日期` `I 類別`; 24 trade rows, all `BUY`. Columns `J–O` hold an unrelated 派息 table (43 rows).
- `美股Trade` columns: `A 股票代碼` `B 股數` `C =D/B` `D buy total =(B*E)+F` `E 單價` `F fee` `G 日期` `H 類別`; 10 rows, including one adjustment row with 0 shares and total −0.01.
- `港股`/`美股` compute per-stock figures with SUMIFS over the trade sheets, `GOOGLEFINANCE` for prices, and hand-written sector rollups (`港股` rows 20–25). Their cached values are present in the file, which is what makes a parity check possible.
- Local toolchain: rustc 1.94.1, node 26.8.2, pnpm, sqlite3. `.devin/config.local.json` already pre-approves `cargo build`, `npm install`, `node`.
- Confirmed with the owner: Rust + SQLite backend, Vue frontend, manual price entry, "same as current sheet" averaging, import everything, migrate section by section, spreadsheet stays authoritative until all sections are done.

## Goals / Non-Goals

**Goals:**
- One data model that serves both markets while keeping their different input conventions at the edge.
- All derived money figures computed on read from stored trades, so edits can never leave stale totals.
- Numeric parity with the spreadsheet, demonstrable by an automated check.
- A layout that later sections (派息, 定期, Month Stat) can be added to without restructuring.
- Single command to run locally; no daemon, no container, no network access.

**Non-Goals:**
- Multi-user access, authentication, or remote deployment; it binds to loopback only.
- Currency conversion, tax lots, realized P/L reporting, or FIFO accounting.
- Any market-data integration, even optional.
- Writing back to the workbook or to Google Sheets.

## Decisions

### Store the four money fields, derive nothing twice
Each trade stores `shares`, `unit_price`, `fee`, `total` plus an `input_mode` recording which pair the user typed (`HK_TOTAL` or `US_FEE`). The server fills the missing field per the market rule; edits re-derive from `input_mode`. Everything else — per-trade 平均單價, holdings, cost, weighted average, market value — is computed on read.

Alternatives: store only the typed inputs and derive both fee and total every time (loses the ability to represent an odd row where both were entered, and makes SQL aggregation over `total` awkward); store aggregates on the stock row (fast, but a deleted or edited trade silently corrupts them — the exact failure mode this change is meant to remove).

### f64 money, rounding only at display
The spreadsheet does f64 arithmetic, and parity with its cached values is the acceptance criterion, so the app uses `REAL`/`f64` too. Display rounds: money 2dp, unit price 4dp (US fees run to 6dp, so raw values are kept), percentages 2dp. Parity comparison uses a relative tolerance (1e-6).

Alternative: integer cents — correct in principle, but US fees like `1.000009` and fractional shares mean the natural unit is not cents, and it would introduce rounding differences against the sheet. Noted as a possible later change.

### Reproduce the sheet's 加權平均買入單價 quirk, and label it
`加權平均買入單價 = Σ BUY total ÷ Σ BUY shares`, not cost per held share. Changing it would break parity and silently alter numbers the owner has been tracking for years, so it is reproduced verbatim and the UI column carries the definition as a tooltip/caption. A follow-up change can introduce a true average-cost column beside it once parity is signed off.

### axum + sqlx(SQLite) + a pure `calc` module
Backend crate layout: `main.rs` (bootstrap, static files, router), `db.rs` (pool + `sqlx::migrate!`), `models.rs`, `calc.rs` (pure functions, no DB or HTTP types), `routes/{stocks,trades,summary}.rs`, and two binaries `import_xlsx` and `check_parity`. Keeping `calc.rs` pure is what makes the spreadsheet numbers testable as plain unit tests with no fixtures.

Alternative: `rusqlite` (simpler, synchronous, no compile-time query checking) — `sqlx` is chosen for built-in migrations and async fit with axum. Alternative to a separate importer binary: an HTTP upload endpoint — rejected as one-off work that should not live in the running app's API surface.

### Vue 3 + Vite, three views, market as a tab
`TradesView` (market tab → filters, trade table, trade form with live derived preview of fee/total/平均單價 before submit), `SummaryView` (per-stock table with inline 現價 editing, sector rollup, market totals), `StocksView` (stock registry + metadata). Plain Vue 3 + TypeScript with hand-rolled table/sort; no component library, to keep the dependency surface small for a single-user local tool. Dev runs Vite on 5173 proxying `/api` to the backend on 127.0.0.1:8787; a built `frontend/dist` is served by the backend for normal use.

### Schema
`stocks(id, market, code, ticker, exchange, sector, manual_price, price_updated_at, pe, eps, high52, low52, note, is_active, sort_order)` with `UNIQUE(market, code)`. `sort_order` is scoped to each market and preserves the user's manual summary order.
`trades(id, stock_id → stocks, trade_type, trade_date TEXT 'YYYY-MM-DD', shares, unit_price, fee, total, input_mode, note, created_at, updated_at)`, indexed on `(stock_id, trade_date)`. Dates as ISO text sort lexicographically in SQLite, which is all the ordering and range filtering needs. The DB lives at `data/wealth.db`, gitignored — it will hold personal financial data and must never be committed.

Manual reordering uses `POST /api/stocks/order` with `{ market, stock_ids }`. The endpoint requires every stock id in that market exactly once and updates the order in one transaction; this prevents a partial drag/drop request from dropping rows or mixing markets. Imported stocks receive `sort_order` from their summary-sheet row order, while stocks that appear only in a trade sheet are appended after them.

### Idempotent import keyed on natural fields
The importer matches an existing trade on `(stock_id, trade_date, trade_type, shares, total)` and skips it. Genuine duplicate rows in the source are preserved on the first run because the whole first pass inserts before any dedupe lookup sees its own inserts — implemented by loading the existing set once at start and counting inserts per key against the source's own multiplicity. No synthetic "source row" column is stored, so a manually re-sorted spreadsheet does not break re-import.

## Risks / Trade-offs

- Parity depends on the workbook's cached values → the parity binary reads cached values via `calamine` (verified present, e.g. `港股!E6 = 3.069803529`); it fails loudly if a cell holds no cached value instead of silently comparing against 0.
- Sector rollups in the sheet are hand-grouped (`Banks = 中國銀行 + 匯豐`), while the app derives them from each stock's `sector` field → group membership is asserted in the parity check for the current stocks; naming differences (trailing tabs in sheet values like `"Utilities\t"`) are trimmed on import.
- f64 accumulation order can differ from the sheet's → tolerance-based comparison rather than exact equality; ordering aggregation by `trade_date, id` keeps runs reproducible.
- The 加權平均買入單價 definition is misleading after a SELL → documented in the spec and surfaced in the UI, not silently "fixed".
- Double entry during migration (app + spreadsheet still needed for Overview/Month Stat) → accepted for this phase; the import is idempotent and re-runnable, so the app can be rebuilt from the sheet at any point until it takes over.
- `sqlx` compile-time query macros require a database at build time → use runtime `query`/`query_as` with explicit types, or set `SQLX_OFFLINE`, so a fresh clone builds without a prepared DB.
- SQLite file holds personal financial data → loopback-only bind, `data/` gitignored, no auth deliberately (single local user).

## Migration Plan

1. Build backend + frontend against an empty database.
2. Run `import_xlsx` against `財富分析報告.xlsx`; confirm 24 HK + 10 US trades and the stock list.
3. Run `check_parity`; resolve differences until it reports none.
4. Enter 現價 manually for each stock and eyeball the summary against the `港股`/`美股` sheets.
5. From then on enter new trades in the app; keep updating the spreadsheet's non-trade sheets by hand until later changes migrate them.
6. Rollback is trivial: delete `data/wealth.db`, keep using the spreadsheet, re-run the import later.

## Open Questions

- Whether the summary should eventually show a true cost-per-held-share column alongside the sheet-compatible one — deferred; it does not affect this change's specs or tasks.
- Whether trade notes need to be searchable — deferred until there are enough notes to matter.
