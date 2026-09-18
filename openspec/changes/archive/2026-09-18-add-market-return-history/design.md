# Design

## Context

The MPF tracker already implements exactly this feature shape: an `mpf_history` table (`recorded_on`, `contributions`, `balance`, `synthetic`), `record_history` in `backend/src/mpf.rs` that upserts today's row and backfills synthetic month-end rows via `mpf_gap_month_ends`, and `mpf_last_month`/`mpf_max`/`mpf_figures` in `backend/src/calc.rs` that derive the figures on read. See `proposal.md` for motivation and `specs/` for the required behavior.

The stock side has no history at all: `summary::build` recomputes `MarketTotals` live from trades + `manual_price`. The workbook does carry cached last-month/max cells per market — 港股 `A1:B1` `last month`, `A2:B2` `max Balance %`, `A3:B3` `max net`; 美股 `I4:J4`, `I5:J5`, `I6:J6` (the K column repeats each figure HKD-converted) — plus the year-end 成本/總市值 already imported into `year_snapshots` (HK only; YearInReview's US figures are HKD-combined and are not attributed to US).

## Goals / Non-Goals

**Goals:**
- One `market_history` row per market per day carrying `(buy_cost_priced, market_value)`, with the same upsert + synthetic month-end backfill semantics as `mpf_history`.
- 上月/最高 derived on read by reusing the existing MPF math unchanged.
- Year-end seeding from `year_snapshots` plus cached-cell seeding (last-month row + maxima marks) so HK 上月/最高 reproduce the sheet immediately.
- 持倉總覽 totals strip shows the two figure pairs like the MPF cards.

**Non-Goals:**
- Per-stock last-month/max — no per-stock price history exists; the snapshot would need a much larger table. Possible follow-up.
- History viewer/deletion UI — `mpf_history` has `DELETE /api/mpf/history/:id`; a market equivalent is deferred until needed.

## Decisions

### Store raw components, not derived figures
`market_history` stores `buy_cost_priced` and `market_value` — the exact analogue of MPF's `(contributions, balance)`. 未實現報酬率 and 未實現金額 are then `mpf_figures(balance, contributions)` renamed: `percent = (market_value − buy_cost_priced) ÷ buy_cost_priced`, `amount = market_value − buy_cost_priced`. Alternative considered: storing `net_amount`/`net_percent` directly — rejected because keeping components preserves the zero-denominator rule (`buy_cost_priced = 0` → empty percent) and lets the derivation stay in one place.

### Reuse `MpfPoint` and the MPF calc functions
`load` maps history rows to `MpfPoint { recorded_on, contributions: buy_cost_priced, balance: market_value }` and calls `mpf_last_month`, `mpf_max`, and `mpf_gap_month_ends` verbatim. Alternative: a parallel `market_*` family — pure duplication. Renaming `MpfPoint` to something generic was rejected as unrelated churn on a stable module; the field-name mismatch is confined to the small mapping function.

### Record lazily inside `summary::build`, plus one explicit hook
Every UI mutation (price edit, trade edit, stock create/delete, dividend receipt price) is followed by a summary reload, so recording inside `summary::build` captures every user-visible state with a single call site. `prices::apply` is the only path that can mutate totals without an immediate build (CLI `import_prices`, and it can touch both markets at once), so after its transaction commits it triggers a summary build per updated market. Alternative considered: hooks in all ~9 mutation routes — more invasive, and produces identical rows since snapshots upsert per `(market, day)`. Trade-off accepted: a mutation that is never followed by any build is not timestamped — under the standing-value convention its values still land in the next record.

A row is written only when `totals.market_value` is `Some` (at least one priced stock). An unpriced market simply leaves a gap that later backfill carries forward — this keeps every stored row complete (no NULL `market_value` handling in the figure math).

### Seeds are history rows where a `(cost, value)` pair exists, `app_meta` marks where it does not
`import_xlsx` inserts `INSERT OR IGNORE` synthetic rows dated `YYYY-12-31` from `year_snapshots` rows that have both `cost` and `market_value`. `mpf_max` then includes them for free and January's 上月 resolves to the prior Dec-31 row. Idempotent on re-import for free via the `UNIQUE(market, recorded_on)` constraint. The seeded `cost` is the sheet's cumulative 成本, which equals `buy_cost_priced` when every held stock was priced at year-end — close enough for a max floor, and marked `synthetic`.

The market sheets' cached `last month` cell holds only a **rate** — no amount — so the seeded previous-month-end row reconstructs the pair as `(cost, cost × (1 + rate))` with `cost` = the market's imported Σ BUY total, the same concession MPF's import makes when it cannot recover last month's contributions: the rate reproduces the sheet exactly, the amount approximates the true last-month figure. Since the rate is exact at any assumed cost, the cost is capped at `max net ÷ rate` when the implied amount would exceed the sheet's own `max net` — the row stays consistent with both cached figures. The cached `max Balance %` and `max net` are lone maxima that cannot be decomposed into a `(cost, value)` pair at all, so they live as per-market `app_meta` marks (`market.<MKT>.seed_max_percent` / `.seed_max_amount`) passed to `mpf_max` as seeds — floors, exactly like `mpf.seed_max_*`.

### Response and UI shape
`SummaryResponse` gains `last_month: Option<MarketFigures>` and `max: Option<MarketFigures>` (`MarketFigures { percent: Option<f64>, amount: f64 }`). They sit beside `totals`, not inside it — `MarketTotals` stays the pure output of `market_totals()`. `mpf_max` takes `current: Option<MpfPoint>` so 最高 still reports the marks/history maxima while the market is unpriced; `max` is `None` only when marks, history and current values are all absent. The frontend mirrors the MPF cards: `percent / amount` pairs colored via `compareClass` against the current figure; the helper moves to `format.ts` alongside `signClass`.

## Risks / Trade-offs

- `GET /api/summary` writes a row → idempotent upsert, single-user local app; parity and yearly builds also record harmlessly.
- Gap-month backfill fabricates standing values → same convention as MPF; rows are flagged `synthetic`. For a seeded Dec-31 followed by months of silence, gap rows carry the year-end figures — an approximation, but it matches the convention and the alternative (gaps in 上月) is worse.
- First-ever record has no anchor → no backfill; we do not fabricate values for months that predate any observation.
- Seeded year-end `cost` may differ from `buy_cost_priced` if some stocks were unpriced then → affects only the max floor; documented as synthetic.
- `prices::apply` triggering summary builds adds a full rollup per touched market → trivial cost locally, and it is the only reliable capture point for the CLI path.

## Migration Plan

`backend/migrations/0011_market_history.sql` creates the table; `sqlx::migrate!` applies it on backend start. Existing databases gain the table empty and begin accruing rows from the next summary build; re-running `import_xlsx` seeds the year-end rows (`INSERT OR IGNORE`, safe on a populated table). Rollback: drop the table and revert the code — nothing else depends on it.
