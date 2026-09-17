# Design

## Context

See proposal.md for motivation. Relevant current state:

- `backend/src/calc.rs` derives everything from stored trades/dividends on read — nothing financial is stored denormalized. `market_totals` (calc.rs:329) already computes cumulative buy cost, market value, and dividend totals for a market.
- `stocks.manual_price` is the only price — there is no price history, so past year-end 總市值 cannot be recomputed.
- Migrations are plain SQL files in `backend/migrations/` run by `sqlx::migrate!` (db.rs:40). Latest is `0007`.
- Routes are flat under `backend/src/routes/`, registered in `routes/mod.rs::api_router`. Responses are serialized structs; the Vue views only format and display.
- The workbook (`YearInReview`, `港股` year block `B31:C32`, `美股` `B1:B2`) holds year-end figures the owner previously froze; `import.rs` already skips existing rows on re-import.

## Goals / Non-Goals

**Goals:**
- One yearly row per market computed from trades/dividends, with a stored snapshot only where recomputation is impossible (past 總市值) or the owner wants the sheet's frozen number kept (成本 override).
- Freeze flow replaces the spreadsheet's copy-raw-value-at-year-end step.

**Non-Goals:**
- Sold P/L — column reserved, computed later once SELL semantics are decided.
- Cross-market/currency combining — tables stay per-market.
- Historical price series — snapshots store a single total per year, not per-stock prices.
- Editing/deleting snapshot years beyond what PATCH exposes.

## Decisions

### D1: Store a sparse `year_snapshots` table, compute the rest
New migration `0008_year_snapshots.sql`:

```sql
CREATE TABLE year_snapshots (
    market        TEXT    NOT NULL CHECK (market IN ('HK','US')),
    year          INTEGER NOT NULL,
    invested      REAL,            -- manual override for the year's invested
    cost          REAL,            -- frozen year-end 成本 override
    market_value  REAL,            -- frozen year-end 總市值
    updated_at    TEXT    NOT NULL,
    PRIMARY KEY (market, year)
);
```

Only user-frozen values are stored; `invested`/`cost`/`dividends` recompute from trades/dividends on every read and the snapshot columns act as optional overrides. This preserves the project's "derive on read, never store" convention while giving the sheet's frozen numbers a home.

Alternatives considered: storing all columns per year (rejected — duplicates derivable data, can drift from trades); storing per-stock year-end prices (rejected — the owner only needs the market total, and sheet data only has totals).

### D2: New route module `routes/yearly.rs`
- `GET /api/summary/yearly?market=HK|US` → `Vec<YearRow>`: `{year, invested, cost, market_value, dividends, yield_on_cost, yield_on_value, monthly_dividend, dividend_yoy, invested_yoy, sold_pl: null, snapshot: {invested?, cost?, market_value?, updated_at?} | null}`.
- `PATCH /api/summary/yearly/:market/:year` — upserts the snapshot row; any of `invested`, `cost`, `market_value` may be set or cleared (null). Rejects non-finite/negative values.
- `POST /api/summary/yearly/:market/:year/freeze` — writes the currently computed cumulative `cost` and live `market_value` into the snapshot (overwriting those two fields; leaves `invested` alone).

Computation: fetch the market's trades (join stocks for market filter) and dividends once, group by `trade_date`/`pay_date` year in Rust (matches how `holdings_snapshot`/dividend rollups already work in `calc.rs`), then walk years in order to build cumulative cost and YoY fields. Year range = earliest activity year → current year.

Alternatives considered: extending `GET /api/summary` with a `years` array (rejected — keeps the summary response small; the year table is a separate panel).

### D3: YoY base columns follow the sheet formulas
`invested_yoy` uses the cumulative `cost` column (the sheet's F column: `(F−F_prev)/F_prev`), and `dividend_yoy` uses `dividends` (J column). Both are empty for the first year and when the denominator is ≤ 0.

### D4: Frontend — a `YearTable` section inside `SummaryView.vue` per market tab
Renders the returned rows; money 2dp, yields/YoY as percentages 3dp per AGENTS.md conventions. Frozen cells show a subtle marker (e.g. `*` or icon) when the value comes from a snapshot. Inline edit on `invested`/`cost`/`market_value` cells issues PATCH; a 凍結 button on the current-year row issues the freeze POST and reloads. No new route or navigation entry — it lives inside the existing 總覽 tab.

### D5: Import seeds snapshots conservatively
`xlsx.rs` gains parsing for the `港股`/`美股` year blocks (year label + year's buy total) and `YearInReview` 股票 cost/"now value" cells where a year and single market are attributable; `import.rs` inserts them as snapshots with `INSERT OR IGNORE`-style skipping so re-imports and existing rows are untouched. The 美股 `B1`/`B2` figures are HKD-converted and do NOT seed the US (USD) table — mixing currencies would violate the per-market separation; US past years are entered manually.

Alternatives considered: seeding YearInReview combined figures into a combined table (rejected — the feature is per-market); parsing the sheet's per-year invested literals (rejected — they are combined-market HKD figures with manual adjustments, not attributable to one market).

### D6: Parity check stays unchanged for now
`check_parity` compares against the same workbook we seed from, so seeded snapshots would trivially match; a parity assertion for year tables adds little until the app becomes source of truth. Noted as a possible follow-up, not a task.

## Risks / Trade-offs

- [Frozen 市值 vs. live totals can disagree silently if the user forgets to freeze on Dec 31] → `updated_at` is returned with each snapshot so the UI can show when the number was captured; freezing is explicit, never automatic.
- [Workbook seed attribution is heuristic — `YearInReview` 股票 figures may not cleanly map to one market] → seed only cells that parse unambiguously; anything ambiguous is skipped and left for manual entry. Worst case: fewer seeded rows, never wrong ones.
- [`invested` semantics on future SELL rows] → spec defines it as `Σ BUY − Σ SELL` for the year (net cash deployed); equals Σ BUY while no sells exist. 成本 stays Σ BUY-only per existing convention, so the two diverge after a sell — that matches the sheet's separate invested vs. cumulative-total-buy columns.
- [Current-year row is live, so its 市值 moves with price edits] → that is intended; the row only freezes when the user clicks 凍結.

## Migration Plan

1. Add `0008_year_snapshots.sql`; `sqlx::migrate!` applies it on next backend start. Existing databases upgrade in place; no data migration needed (snapshots start empty).
2. Deploy backend + rebuilt frontend together — the new endpoints are additive; no breaking change.
3. Rollback: drop the table and revert the route/view; no other data depends on it.

## Open Questions

- Exact workbook cells to seed from per market (which `YearInReview` columns are HK-only vs. combined) — resolved during implementation by inspecting the sheets; ambiguous cells are skipped, so this cannot change the spec or approach.
- Whether a future `sold_pl` column needs realized-P/L tracking per year — deferred by spec.
