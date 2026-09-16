# Wealth tracker data-flow guide

This guide explains where each action saves data, which database table changes, where calculations happen, and what the frontend reloads afterward.

## Mental model

```text
Vue UI
  ↓ user action / form submission
Rust HTTP API
  ↓ validation + derivation
SQLite database (`data/wealth.db`)
  ↓ read back rows
Rust calculation layer
  ↓ API response
Vue UI displays formatted values
```

The **backend is authoritative**. The frontend may show a temporary preview while typing, but the backend validates and recalculates every value before saving or returning summary figures.

There is **no summary table** in SQLite. Summary figures are recomputed from `stocks` and `trades` every time the summary API is called, and deposit rollups are recomputed from `deposits`.

## Database tables

### `stocks`

One row per stock per market.

| Column | Meaning |
|---|---|
| `id` | Internal stock ID |
| `market` | `HK` or `US` |
| `code` | Displayed 股票代碼 / stock name |
| `ticker` | Optional ticker, e.g. `0003`, `VOO` |
| `exchange` | Optional exchange, e.g. `HKG`, `NYSE` |
| `sector` | Optional sector used for the sector rollup |
| `manual_price` | Manually entered 現價 |
| `price_updated_at` | Timestamp set when `manual_price` is updated |
| `pe`, `eps`, `high52`, `low52` | Optional metadata |
| `note` | Optional free-text note |
| `is_active` | Whether the stock is active |
| `sort_order` | Manual display order within that market |

`market + code` must be unique. The same code can exist in HK and US because the market is part of the uniqueness check.

### `trades`

One row per trade.

| Column | Meaning |
|---|---|
| `id` | Internal trade ID |
| `stock_id` | Links to `stocks.id` |
| `trade_type` | `BUY` or `SELL` |
| `trade_date` | `YYYY-MM-DD` text date |
| `shares` | 股數 |
| `unit_price` | Entered 單價, excluding fee |
| `fee` | Stored fee |
| `total` | Stored total including fee |
| `input_mode` | `HK_TOTAL` or `US_FEE` |
| `note` | Optional note, required for zero-share or negative-fee adjustments |
| `created_at`, `updated_at` | Audit timestamps |

### `deposits`

One row per 定期 deposit (from 定期Info's 表_定期List). Nullable columns reflect the sheet's interest-only and label-only rows.

| Column | Meaning |
|---|---|
| `id` | Internal deposit ID |
| `label` | The sheet's `id` column: bank reference like `SC-9632` |
| `bank` | Bank code (SC = 渣打, HS = 恒生); derived from the label prefix on import |
| `principal` | The sheet's `input` column |
| `rate` | Annual rate as a fraction (0.03 = 3%) |
| `interest` | 利息 |
| `end_date` | `YYYY-MM-DD` text date, required |
| `note1`, `note2` | Optional notes |
| `sort_order` | Workbook row order; new entries append |
| `created_at`, `updated_at` | Audit timestamps |

### `dividends`

One row per 派息 event per stock (from the trade sheets' J–O block). The snapshot columns freeze the position at the pay date, so later trades never rewrite a recorded rate.

| Column | Meaning |
|---|---|
| `id` | Internal dividend ID |
| `stock_id` | Links to `stocks.id` |
| `pay_date` | `YYYY-MM-DD` text date (the sheet's K column) |
| `per_share` | Announced 每股派息, optional |
| `shares_held` | 股數 snapshot on the pay date |
| `buy_cost` | 總買入成本 snapshot on the pay date |
| `estimated_amount` | 預期派息, optional |
| `received_amount` | 實收派息; empty while pending |
| `received_price` | 現價 snapshot at receipt, optional |
| `note` | Optional free-text note (imported rows keep the sheet's M formula) |
| `created_at`, `updated_at` | Audit timestamps |

### Values not stored

These are calculated by the backend when needed:

- Per-trade 平均單價
- Shares held
- 總買入成本
- 加權平均買入單價
- 當前總市值
- 未實現金額
- 未實現報酬率
- Sector rollups
- Market totals
- Deposit `total` (principal + interest)
- Deposit status (`End` once end_date is today or past)
- Deposit end month/year
- Active/history/month/year/bank rollups
- Dividend status (`received` once `received_amount` is set)
- Dividend effective amount (received else estimated)
- Dividend `rate` = amount ÷ `buy_cost` snapshot
- Dividend second `rate` = amount ÷ (`received_price` × `shares_held`)
- Dividend variance = `received_amount − estimated_amount`
- Per-year received rollups

This avoids stale copied totals.

## Feature flows

## Add a stock in 股票管理

The 新增港股股票 / 新增美股股票 button reveals the StocksView form, which collapses after a successful save.

```text
StocksView form
  → POST /api/stocks
  → validate market + code
  → INSERT one row into stocks
  → assign the next sort_order in that market
  → GET /api/stocks?market=... reloads the stock list
```

What is saved:

- One row in `stocks`
- No row in `trades`

What is calculated:

- No financial calculation yet
- The new stock appears in 持倉總覽 with zero holdings and zero buy cost

## Remove a stock

```text
刪除 in a StockTable row's ⋯ menu (in StocksView)
  → browser confirmation
  → DELETE /api/stocks/:id
  → backend counts trades where stock_id = :id
  → if trades exist: reject
  → otherwise DELETE one row from stocks
  → GET /api/stocks?market=... reloads the stock list
```

A stock with trades or dividend records cannot be deleted. Delete those rows first, or keep the stock for history.

## Update stock metadata in 股票管理

```text
StockForm edit mode (via 編輯 in a StockTable row's ⋯ menu)
  → PATCH /api/stocks/:id
  → backend updates only the fields included in the patch
  → UPDATE one stocks row
  → GET /api/stocks?market=... reloads all stocks in that market
```

Fields such as `ticker`, `exchange`, `sector`, `pe`, `eps`, `high52`, `low52`, and `note` only affect the stock row.

- `sector` affects the sector rollup the next time the summary is loaded.
- Metadata edits do not modify any trades.
- Sending `null` for an optional field clears that field.

## Update 現價 in 持倉總覽

```text
Edit price in SummaryView
  → PATCH /api/stocks/:id { manual_price: 699.3 }
  → backend UPDATEs one row in stocks
  → backend also updates price_updated_at
  → frontend calls GET /api/summary?market=...
  → backend reloads all stocks and trade facts for that market
  → backend recomputes the complete summary
  → frontend displays the returned figures
```

This updates **one stock row**, then fetches the **whole market summary**, not only that stock's row.

Recalculated on the backend:

- 當前總市值 = `manual_price × shares held`
- 未實現金額 = `market value − total buy cost`
- 未實現報酬率 = `unrealized amount ÷ total buy cost`
- Sector rollups
- Market totals

Not changed:

- Any `trades` row
- Historical buy cost
- Weighted average buy price

No external market-data API is called. The price is manual data saved locally.

## Bulk-update 現價 from a price file

```text
匯入現價 JSON button in SummaryView (or: import_prices CLI)
  → file text is sent to POST /api/stocks/prices
  → backend parses {"stocks": [{"symbol", "price"}]}
  → each symbol resolves to a stock (NNNN.HK → HK ticker; else US code/ticker, - ≡ .)
  → one transaction UPDATEs stocks.manual_price + price_updated_at per match
  → response reports updated, unmatched, invalid, and not_updated
  → frontend calls GET /api/summary?market=...
```

Unmatched or invalid entries do not block the rest; matched prices are applied and the report lists every skipped symbol. Stocks absent from the file keep their previous 現價 and are listed as `not_updated`.

The same logic runs offline via:

```sh
cargo run -p wealth-backend --bin import_prices -- current-price.json
```

No external market-data API is called; the JSON file is user-supplied input equivalent to manual entry.

## Add a trade in 交易記錄

The 新增交易 button reveals TradeForm, which collapses after a successful save.

```text
TradeForm
  → POST /api/trades
  → backend resolves stock_id from market + code
  → backend validates date/type/shares/price
  → backend derives fee or total
  → INSERT one row into trades
  → response returns the stored trade plus per-trade 平均單價
  → frontend reloads stocks and the filtered trade list
```

HK input:

```text
fee = buy total − 股數 × 單價
```

US input:

```text
buy total = 股數 × 單價 + fee
```

Both `fee` and `total` are stored, so either market can show complete trade history.

Per-trade 平均單價 is not stored. The backend returns it as:

```text
total ÷ 股數
```

It is empty for zero-share adjustment rows.

## Edit a trade

```text
TradeForm edit mode (via 編輯 in a TradeTable row's ⋯ menu)
  → PATCH /api/trades/:id
  → backend loads the existing trade
  → backend merges changed fields with unchanged fields
  → backend validates and derives fee/total again
  → UPDATE one trades row
  → frontend reloads stocks and the filtered trade list
```

Editing a trade changes only that trade row. Summary values are not stored; they update automatically the next time 持倉總覽 is loaded.

## Delete a trade

```text
刪除 in a TradeTable row's ⋯ menu
  → browser confirmation in TradesView
  → DELETE /api/trades/:id
  → backend deletes one trades row
  → frontend reloads stocks and the filtered trade list
```

Because summary figures are calculated from `trades`, deleting a trade changes holdings and buy cost on the next summary load.

## Load 持倉總覽

```text
SummaryView
  → GET /api/summary?market=HK or US
  → backend loads all stocks in that market, ordered by sort_order
  → backend loads all trade facts for that market
  → backend groups trades by stock_id
  → backend calculates every stock row
  → backend calculates sector rollups and market totals
  → frontend formats and displays the response
```

The summary request fetches the selected market's complete summary. It does not fetch only changed rows.

Per stock, the backend calculates:

```text
shares held = Σ BUY 股數 − Σ SELL 股數
總買入成本 = Σ BUY total
加權平均買入單價 = 總買入成本 ÷ Σ BUY 股數
當前總市值 = 現價 × shares held
未實現金額 = 當前總市值 − 總買入成本
未實現報酬率 = 未實現金額 ÷ 總買入成本
```

Important spreadsheet-compatible behavior:

- A SELL reduces shares held.
- A SELL does **not** reduce 總買入成本.
- 加權平均買入單價 divides by **shares bought**, not shares held.
- If there are no holdings, weighted average is shown as `0`.
- If 現價 is missing, market value and unrealized figures are empty, not zero.

## Reorder stocks in 持倉總覽

```text
Drag the ↕ handle in SummaryView
  → frontend moves the row immediately for responsive UI
  → POST /api/stocks/order { market, stock_ids: [...] }
  → backend checks that every stock id in that market appears exactly once
  → backend updates sort_order for each row in one transaction
```

What is saved:

- `stocks.sort_order` only

What is not changed:

- No trade data
- No price data
- No calculated values

If the reorder request fails, the frontend reloads the previous summary order.

## Filter trades

```text
Filter controls in TradesView
  → GET /api/trades?market=...&code=...&from=...&to=...&order=...
  → backend filters and sorts matching rows
```

Filtering does not change saved data. It only changes which `trades` rows are returned.

## Add a deposit in 定期記錄

The 新增定期 button reveals DepositForm, which collapses after a successful save.

```text
DepositForm
  → POST /api/deposits
  → backend validates end_date and non-negative amounts
  → INSERT one row into deposits, sort_order appended
  → GET /api/deposits?year=... reloads the history list
```

The form accepts rate as a percent; the API stores the fraction. `bank` is free text with 渣打 (SC) / 恒生 (HS) presets.

## Edit or delete a deposit

```text
DepositForm edit mode (via 編輯 in a DepositTable row's ⋯ menu)
  → PATCH /api/deposits/:id
  → backend merges the patch, validates, UPDATEs one row

刪除 in the row's ⋯ menu → DELETE /api/deposits/:id
```

Derived fields (`total`, `status`, end month/year) change automatically on the next read.

## Load the 定期 view

```text
DepositsView (定期 tab)
  → GET /api/deposits/summary
      → upcoming list (end_date > today, earliest first)
      → active totals, month buckets, bank rollups
      → year tables for every end year present, plus the current year

DepositHistoryView (定期記錄 tab)
  → GET /api/deposits/summary
      → history_years for the year selector
  → GET /api/deposits?year=YYYY&order=desc
      → history rows for the selected year
```

Status and rollups are point-in-time: they derive from today, so a deposit moves from 未到期定期 to history on its end date without any stored change.

The 手動步驟提醒 checklists (定期 start step / 定期 end step) are static hints for the still-unmigrated `Month Stat`, `回報率`, `Overview`, and money-master bookkeeping in the workbook.

## Record a dividend in 派息

```text
DividendForm (記錄派息)
  → POST /api/dividends
  → backend resolves stock_id from stock_id or market + code
  → backend derives the snapshots from trades with trade_date <= pay_date:
      shares_held = Σ BUY 股數 − Σ SELL 股數
      buy_cost    = Σ BUY total
  → caller-supplied shares_held/buy_cost override the derivation
  → backend validates and stores one row in dividends
  → estimated_amount defaults to 每股派息 × shares_held when only 每股派息 is given,
    and 每股派息 is implied as estimated_amount ÷ shares_held when only 預期派息 is given
  → the 派息 view reloads GET /api/dividends/summary?market=...
```

The snapshots are stored, not recomputed: buying more of the same stock later does not change a recorded dividend's 股數, 總買入成本, or rate. If the pay date was wrong, editing it with 重新計算快照 checked re-derives both snapshots.

## Mark a dividend received

```text
收訖 on a pending row → DividendReceiveForm
  → PATCH /api/dividends/:id { received_amount, received_price? }
  → status becomes received; both rates now use the received amount
  → 同時更新現價 checked → a separate PATCH /api/stocks/:id sets manual_price
```

The 現價 entered at receipt is stored on the dividend record only. It does not update the stock's 現價 unless 同時更新現價 is checked, because the receipt price may be recorded on a different day than the price update.

## Load the 派息 view

```text
DividendsView (派息 tab, market-scoped like 交易記錄)
  → GET /api/dividends/summary?market=...
      → pending list, per-year received totals, per-stock breakdown,
        history_years for the year selector
  → GET /api/summary?market=...
      → stock list for the form; the per-share × shares preview hint
  → GET /api/dividends?market=...&status=received&year=YYYY&order=desc
      → received history rows
```

Derived on every read, never stored: `status`, effective `amount`, `yield_on_cost` (amount ÷ buy_cost), `yield_on_price` (amount ÷ received_price × shares_held), `variance` (received − estimated). Yields are empty when their denominator is missing.

## Import workbook data

```text
cargo run -p wealth-backend --bin import_xlsx -- "財富分析報告.xlsx"
  → xlsx.rs opens the workbook read-only
  → cached formula values are read
  → import.rs inserts/updates stocks and inserts missing trades
```

The importer reads:

- `港股Trade` — trade columns plus the J–O 派息 block
- `美股Trade` — trade columns plus the J–O 派息 block
- `港股`
- `美股`
- `定期Info` (表_定期List) and `定期` (cached aggregates for parity)

For each dividend row the importer stores J (stock), K (pay date, Excel serial dates accepted), M (派息 amount, cached value), and O (股數 snapshot). The remaining snapshots are recovered from the cached rates the same way the sheet computed them — `buy_cost = M ÷ L`, `received_price = M ÷ (N × O)` — falling back to trade-derived snapshots when a rate is absent. The M formula text, when present, is kept in `note`. Rows with an N rate, or a pay date already past, import as received; future rows without it import as pending estimates.

The import is idempotent. A second run skips trades already stored with the same stock, date, type, shares, and total, deposits already stored with the same label, end date, principal, and interest, and dividends already stored with the same stock, pay date, and amount.

The workbook is never modified.

## Run parity check

```text
cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"
  → backend recomputes stock summaries from SQLite trades
  → compares them to cached workbook summary values
  → recomputes deposit aggregates from SQLite deposits
  → compares them to 定期's cached month/bank rows, 定期!B1,
    and the 定期Info year tables
  → counts stored dividends and totals the effective amount per market,
    compared against the J–O block's row count and Σ 派息
```

This verifies that the database reproduces the spreadsheet's trade-derived figures, deposit rollups, and dividend totals. Deposit parity is point-in-time: the cached values reflect the workbook's last recalculation, so a deposit that matures after that point shows as a difference.

## Frontend vs backend responsibilities

| Responsibility | Frontend | Backend |
|---|---:|---:|
| Collect form input | Yes | — |
| Temporary fee/total preview | Yes | — |
| Validate input before saving | Basic required fields | Authoritative validation |
| Derive HK fee | Preview only | Yes |
| Derive US total | Preview only | Yes |
| Store stocks/trades | — | Yes |
| Per-trade 平均單價 | Displays it | Calculates it |
| Holdings/cost/average | Displays them | Calculates them |
| Market value/unrealized | Displays them | Calculates them |
| Sector rollup/totals | Displays them | Calculates them |
| Formatting/rounding | Yes | Keeps full `f64` precision |
| Manual stock order | Drag/drop UI | Persists `sort_order` |
| Deposit CRUD | Form (add + edit) | Yes, validates |
| Deposit totals/status/rollups | Displays them | Calculates them |
| Dividend CRUD + receipt | Forms | Yes, validates and snapshots |
| Dividend rates/variance/rollups | Displays them | Calculates them |
| Workbook import | — | Yes |
| Parity check | — | Yes |

## Common questions

### Is 平均單價 saved in the database?

No. It is calculated as `total ÷ 股數` when a trade is returned.

### Is the summary saved in the database?

No. Every `GET /api/summary` recalculates it from `stocks` and `trades`.

### Does updating 現價 touch trades?

No. It updates only `stocks.manual_price` and `stocks.price_updated_at`.

### Does updating 現價 fetch one row or the whole summary?

The update itself affects one stock row. Afterward, the frontend fetches the complete summary for the selected market so all totals and rollups are consistent.

### Does deleting a stock delete its trades?

No. The backend refuses to delete a stock that still has trades or dividend records.

### Does buying more shares change a recorded dividend?

No. The 股數 and 總買入成本 on a dividend row are snapshots stored when the dividend was recorded; the rate columns use those frozen values.

### Does recording the receipt price update the stock's 現價?

No. `received_price` is stored on the dividend only. The 同時更新現價 checkbox issues a separate stock update, so recording a receipt on a different day than the price change is safe.

### Does reordering stocks change calculations?

No. It changes only `sort_order`.

### Does the app modify the Excel workbook?

No. Import and parity open it read-only.
