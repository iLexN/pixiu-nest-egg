## Context

`backend/src/calc.rs::summarize` produces the per-stock `StockSummary` purely from `TradeFacts` plus the manual price; `backend/src/routes/summary.rs::build` collects trades per stock and calls it. Dividends already live in the `dividends` table with `received_amount` / `estimated_amount`, and `parity.rs::check_dividends` already defines the sheet-equivalent "effective amount" as `COALESCE(received_amount, estimated_amount)` — verified to match the 港股 sheet's K column (e.g. 中國銀行 K6 = 54023.87).

The 港股 sheet formulas (row 5 headers): K 累計派息 `=SUMIF(trade!J:J, code, trade!M:M)`, P 累計派息% `=K/F`, U 淨投入總本金 `=F−K`, V 淨攤薄單價 `=U/D`, W 實質動態總回報% `=(H−U)/U`. The 美股 sheet has none of these columns.

## Goals / Non-Goals

**Goals:**
- The summary API exposes the five per-stock figures and three market totals for both markets, computed on each read from stored trades, dividends and 現價.
- The figures mirror the workbook formulas so the sheet's numbers are reproducible.
- The parity command can verify the dividend-dependent columns against the workbook's cached values.

**Non-Goals:**
- No schema migration — all values are derived.
- No dividend data-model or lifecycle changes.
- Sector rollups stay unchanged.
- 實質動態總回報% is not parity-checked (sheet used live market-data prices).

## Decisions

- **`summarize` gains a `dividend_total` parameter rather than fetching dividends inside it.** Calc stays pure (no DB); `build()` runs one grouped query (`SUM(received_amount)` per stock) and passes the value in. Alternative considered: a separate post-processing function — rejected because the five figures belong on `StockSummary` next to the figures they derive from, and one function keeps the formula chain in one place.
- **累計派息 counts received amounts only** (owner's decision). The sheet sums every J–O row including pending estimates; the app deliberately diverges so 淨投入總本金 only reflects money actually received. A pending estimate changes nothing until it is marked received.
- **Parity computes the sheet-equivalent figures with effective amounts.** The API's received-only 累計派息 would legitimately differ from sheet K for stocks with pending rows (currently 匯豐, 中移動, 長江基建). Parity therefore recomputes net-position figures per stock using `COALESCE(received_amount, estimated_amount)`, reproducing the sheet's own semantics; the four price-independent columns K/P/U/V are compared, W is not.
- **Aggregate 實質動態總回報% uses the priced subset** — `(Σ market value − Σ net invested of priced stocks) ÷ Σ net invested of priced stocks` — mirroring how `net_percent` already uses `buy_cost_priced`. Unpriced stocks still contribute to the 累計派息 and 淨投入總本金 totals.
- **JSON field names**: `dividends_received`, `dividend_return`, `net_invested`, `net_diluted_price`, `real_total_return` on the per-stock summary; `dividends_received`, `net_invested`, `net_invested_priced`, `real_total_return` on totals. English snake_case matches the existing API; the Chinese labels live only in the UI.
- **Column order in 持倉總覽**: appended after 未實現報酬率 in the sheet's logical order — 累計派息, 累計派息%, 淨投入總本金, 淨攤薄單價, 實質動態總回報%.

## Risks / Trade-offs

- Negative 淨投入總本金 when dividends exceed buy cost → displayed as-is (mirrors the sheet); only the 0 denominator is suppressed, matching the sheet's `#DIV/0!` behaviour.
- UI/sheet divergence for stocks with pending estimates → intentional; parity still validates the formula implementation via effective amounts, and the divergence disappears once each dividend is received.
- `summarize` signature change touches existing unit tests → mechanical; new tests cover the dividend-adjusted paths.
