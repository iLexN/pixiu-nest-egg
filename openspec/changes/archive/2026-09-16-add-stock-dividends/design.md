## Context

See proposal.md — Why. The workbook tracks dividends in columns J–O of `港股Trade`/`美股Trade`: J stock, K pay date, L `= M ÷ <hardcoded 總買入成本>` (yield on cost), M 派息 amount (often a formula encoding the estimate: `=(0.12*4000)-30`, `=0.36*7.7731*400`, `=(2.9003*O42)*0.9-30`), N `= M ÷ (<hardcoded price> × O)` (yield on market price), O 股數 snapshot. The app already owns trades, stocks, and deposits; dividends slot into the same pattern: stored facts + derived fields + a summary endpoint.

## Goals / Non-Goals

**Goals:**
- One row per dividend event; two-phase lifecycle (pending → received) on one record.
- Point-in-time snapshots for 股數 / 總買入成本 / 現價 so later activity never rewrites history.
- Import the existing J–O history idempotently; light parity against it.
- UI mirrors the deposits/trades pattern: market-scoped tab, form + table + rollups.

**Non-Goals:**
- Structured estimate inputs (fx_rate, withholding, collection fee). Deliberately minimal: `per_share` + `estimated_amount` + free-text `note` reproduce the sheet's M formulas without new schema. Chosen by the owner.
- Ex-date vs pay-date modelling — K is the pay date; only `pay_date` is stored (the receipt lands the same day, so no separate received_date).
- Feeding Month Stat / 回報率 sheets — that migration is later; the summary endpoint only exposes the aggregates they will need.
- Auto price fetch; dividend amounts are native currency (HKD / USD) like trade totals.

## Decisions

### Schema: one row, two amount fields
`estimated_amount` and `received_amount` are separate columns rather than one `amount` overwritten on receipt. The sheet overwrites M, but keeping both preserves the estimate for the variance display at no extra workflow cost — receipt is a PATCH, not a rewrite. Status derives from `received_amount IS NULL`, so no status column. `shares_held`/`buy_cost`/`received_price` are nullable REAL columns so imported history and partial edits stay representable.

Alternative considered: a single `amount` + status flag — rejected because it loses the estimate exactly when the interesting comparison (assumed FX/fee vs actual) appears.

### Snapshots: derive at write, store, never recompute
On `POST /api/dividends`, `shares_held` and `buy_cost` are computed from trades with `trade_date <= pay_date` (ΣBUY−ΣSELL shares; ΣBUY total — the same convention as the portfolio summary). Storing them is what makes the sheet's hardcoded L/N denominators safe: editing or adding trades later cannot move a stored rate. `PATCH` never re-derives unless `refresh_snapshots: true` is sent, covering the "pay_date was wrong" case without making snapshots implicit.

### Derived fields on read
Following deposits (`total`/`status` derived in `row_to_*`), dividends derive `status`, effective `amount`, `yield_on_cost`, `yield_on_price`, and `variance` in `row_to_dividend`. Rates return `None` when their denominator is missing/zero — empty cells, not zeros, matching the sheet.

### Receiving = PATCH
`PATCH /api/dividends/:id` with `received_amount` flips status. The frontend's 收訖 form also offers 同時更新現價, which issues a second `PATCH /api/stocks/:id` — kept as an explicit checkbox because the user may update 現價 on a different day.

### Import: recover denominators from cached rates, not formula parsing
`calamine`'s `worksheet_range` yields cached values only. Rather than parsing L/N formula text, the importer recovers `buy_cost = M ÷ L` and `received_price = M ÷ (N × O)` from the cached numbers — exactly the denominators the user typed, even where trades can't reproduce them. Fallback when L/N/O are absent: derive snapshots from imported trades as-of K. M's formula text is read via `worksheet_formula` and stored in `note` for provenance only.

Import status heuristic: N present → received with recovered price; else pay_date ≤ today → received without price (early rows predate the second-rate habit); else → pending estimate. The import report lists each row's status so a misclassified recent row is a one-PATCH fix.

Idempotency matches trades: skip when `(stock_id, pay_date, amount)` already exists, where amount compares `COALESCE(received_amount, estimated_amount)` — first run preserves genuine duplicate rows.

### Parity: count + Σ amount per market
The sheet stores no dividend aggregate (yearly SUMIFs live in 回報率 sheets and use row ranges, not dates). The check therefore compares row count and ΣM per market against Σ effective amount in the DB — enough to catch missed/duplicated rows without pretending the sheet has rollups it doesn't.

### API surface
- `GET /api/dividends?market=&status=&year=&order=`
- `POST /api/dividends` — `{stock_id | market+code, pay_date, per_share?, estimated_amount?, shares_held?, buy_cost?, note?}`
- `PATCH /api/dividends/:id` — merge patch; `refresh_snapshots` flag; receive fields
- `DELETE /api/dividends/:id`
- `GET /api/dividends/summary?market=` — `{today, pending[], years[{year, total, stocks[{code,total}]}]}`

`resolve_stock` from trades is reused for `market`+`code` lookup. `stocks::remove` gains a dividend count alongside the existing trade count.

### Frontend shape
New market-scoped 派息 tab (`'dividends'` joins `STOCK_TABS`). `DividendsView` = 待收派息 table + 已收記錄 table (year filter) + per-stock year rollup, all fed by `GET /api/dividends/summary`. `DividendForm` creates records (stock dropdown, pay date, per_share, estimated_amount override, note); its live `per_share × shares` hint reuses `GET /api/summary?market=` figures as a non-authoritative preview (same rule as the trade form's preview) — the authoritative snapshot is whatever the server derives at create. `DividendTable` rows carry an inline 收訖 editor for pending rows.

## Risks / Trade-offs

- [Import heuristic misclassifies a recent row] → import report prints each row's status; a single PATCH corrects it.
- [M ÷ L recovery inherits stale sheet denominators] → acceptable: it reproduces exactly what the sheet showed; rows where L is absent fall back to trade-derived snapshots and the report notes which source was used.
- [Zero-share / delisted stocks] → rates degrade to empty cells; validation still requires per_share or estimated_amount so a record can't be meaningless.
- [received_date vs pay_date rollup year] → resolved by dropping received_date entirely: receipts land on the pay date, so the summary groups by `pay_date` — the bucket the later Month Stat migration needs.
