# Stock tracker data-flow guide

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

There is **no summary table** in SQLite. Summary figures are recomputed from `stocks` and `trades` every time the summary API is called.

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

This avoids stale copied totals.

## Feature flows

## Add a stock in 股票管理

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
Delete button in StocksView
  → browser confirmation
  → DELETE /api/stocks/:id
  → backend counts trades where stock_id = :id
  → if trades exist: reject
  → otherwise DELETE one row from stocks
  → GET /api/stocks?market=... reloads the stock list
```

A stock with trades cannot be deleted. Delete its trades first, or keep the stock for history.

## Update stock metadata in 股票管理

```text
StocksView edit form
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

## Edit a trade inline

```text
TradeTable inline editor
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
Delete button in TradeTable
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

## Import workbook data

```text
cargo run -p wealth-backend --bin import_xlsx -- "財富分析報告.xlsx"
  → xlsx.rs opens the workbook read-only
  → cached formula values are read
  → import.rs inserts/updates stocks and inserts missing trades
```

The importer reads:

- `港股Trade`
- `美股Trade`
- `港股`
- `美股`

The import is idempotent. A second run skips trades already stored with the same stock, date, type, shares, and total.

The workbook is never modified.

## Run parity check

```text
cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"
  → backend recomputes stock summaries from SQLite trades
  → compares them to cached workbook summary values
```

This verifies that the database reproduces the spreadsheet's trade-derived figures.

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

No. The backend refuses to delete a stock that still has trades.

### Does reordering stocks change calculations?

No. It changes only `sort_order`.

### Does the app modify the Excel workbook?

No. Import and parity open it read-only.
