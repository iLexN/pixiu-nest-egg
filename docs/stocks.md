# 股票 flows — stocks, trades, prices, dividends, yearly

Part of the [data-flow guide](DATA_FLOW.md). Table definitions: [`stocks`, `trades`, `dividends`, `year_snapshots`, `market_history`](database.md).

## Add a stock in 股票 → 管理

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

## Update stock metadata in 股票 → 管理

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

## Update 現價 in 股票 → 總覽

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

The live formula preview in TradeForm mirrors those rules only so the user can check the numbers before submitting.

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

## Load 股票 → 總覽

```text
SummaryView
  → GET /api/summary?market=HK or US
  → backend loads all stocks in that market, ordered by sort_order
  → backend loads all trade facts for that market
  → backend sums received 派息 per stock
  → backend groups trades by stock_id
  → backend calculates every stock row
  → backend calculates sector rollups and market totals
  → backend upserts today's market_history row when at least one stock is
    priced, backfilling synthetic month-end rows for fully elapsed months
  → backend derives 上月 (latest history row in the previous calendar month)
    and 最高 (maxima over all history rows plus the current values) for
    未實現報酬率 and 未實現金額 — reusing the MPF math (`mpf_last_month`,
    `mpf_max`) over history rows mapped to (contributions, balance); the
    two maxima are independent
  → frontend formats and displays the response
```

The summary request fetches the selected market's complete summary. It does not fetch only changed rows. A bulk price update (`prices::apply`, UI or `import_prices` CLI) triggers the same `market_history` build per touched market so bulk/CLI imports are captured too.

Per stock, the backend calculates:

```text
shares held = Σ BUY 股數 − Σ SELL 股數
總買入成本 = Σ BUY total
加權平均買入單價 = 總買入成本 ÷ Σ BUY 股數
當前總市值 = 現價 × shares held
未實現金額 = 當前總市值 − 總買入成本
未實現報酬率 = 未實現金額 ÷ 總買入成本
累計派息 = Σ received_amount of the stock's 派息 records
累計派息% = 累計派息 ÷ 總買入成本
淨投入總本金 = 總買入成本 − 累計派息
淨攤薄單價 = 淨投入總本金 ÷ shares held
實質動態總回報% = (當前總市值 − 淨投入總本金) ÷ 淨投入總本金
```

Important spreadsheet-compatible behavior:

- A SELL reduces shares held.
- A SELL does **not** reduce 總買入成本.
- 加權平均買入單價 divides by **shares bought**, not shares held.
- If there are no holdings, weighted average is shown as `0`.
- If 現價 is missing, market value and unrealized figures are empty, not zero.
- 累計派息 counts **received amounts only**; a pending estimate changes none of the dividend-adjusted figures until it is marked received. This deliberately differs from the workbook's K column, which sums every J–O row including future ones.
- 累計派息% is empty when 總買入成本 is 0; 淨攤薄單價 is empty when holdings are 0; 實質動態總回報% is empty when the stock has no 現價 or 淨投入總本金 is 0.

The totals row also shows 累計派息 and 淨投入總本金 summed across all stocks in the market, plus an aggregate 實質動態總回報% = (total market value − 淨投入總本金 of priced stocks) ÷ 淨投入總本金 of priced stocks. Stocks without 現價 are excluded from that denominator, matching how the unpriced subset is excluded from 未實現報酬率.

Stocks with no current price, zero holdings, or no derived market value are not included in market-value totals; the summary response lists them in `totals.excluded_codes`.

## Reorder stocks in 股票 → 總覽

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

## Hide a stock from 股票 → 總覽

```text
⋯ menu on a row in 股票 → 管理 → 隱藏 (or 顯示 to undo)
  → PATCH /api/stocks/:id { is_active: false }
  → backend updates stocks.is_active
  → 持倉總覽 omits inactive rows on the next load
```

Hiding is display-only:

- Hidden stocks remain in the summary response and still count toward totals and sector rollups.
- Their trades, dividends and prices are untouched, and they stay selectable in the trade and dividend forms.
- Hidden rows stay in the registry, shown muted, so they can be unhidden; a stock with records still cannot be deleted.

## Filter trades

```text
Filter controls in TradesView
  → GET /api/trades?market=...&code=...&from=...&to=...&order=...
  → backend filters and sorts matching rows
```

Filtering does not change saved data. It only changes which `trades` rows are returned.

## Record a dividend in 股票 → 派息

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
  → PATCH /api/dividends/:id { received_amount, received_price?, bank_in? }
  → status becomes received; both rates now use the received amount
  → banks the amount — HK → HS cash row (also records the div:<id>
    adjustment month_item for the pay month); US → the ibkr.usd_cash
    meta value (no month item — the money never touches 活期)
  → 同時更新現價 checked → a separate PATCH /api/stocks/:id sets manual_price
```

The 現價 entered at receipt is stored on the dividend record only. It does not update the stock's 現價 unless 同時更新現價 is checked, because the receipt price may be recorded on a different day than the price update. Clearing `received_amount` reverses the bank-in credit and deletes the `div:<id>` item.

## Load the 股票 → 派息 view

```text
DividendsView (股票 → 派息 tab, market-scoped like 交易記錄)
  → GET /api/dividends/summary?market=...
      → pending list, per-year received totals, per-stock breakdown,
        history_years for the year selector
  → GET /api/summary?market=...
      → stock list for the form; the per-share × shares preview hint
  → GET /api/dividends?market=...&status=received&year=YYYY&order=desc
      → received history rows
```

Derived on every read, never stored: `status`, effective `amount`, `yield_on_cost` (amount ÷ buy_cost), `yield_on_price` (amount ÷ received_price × shares_held), `variance` (received − estimated). Yields are empty when their denominator is missing.

## Load the yearly table in 股票 → 總覽

```text
SummaryView loads GET /api/summary/yearly?market=HK|US
  → routes/yearly.rs reads the market's trades, received dividends and
    year_snapshots, plus the live market totals
  → calc.rs::yearly_rows builds one row per year from the earliest activity
    year through the current year
  → the frontend formats and displays the rows
```

Each row carries:

- `invested` — Σ BUY − Σ SELL of trades dated in the year; a stored snapshot value wins
- `sold_pl` — 賣出損益, the hand-entered realized P/L stored on `year_snapshots` (the workbook never recorded SELL trades); seeded from YearInReview's 投資P/L, editable per market-year
- `cost` — 年末總成本, cumulative Σ BUY total through Dec 31; a stored snapshot value wins
- `market_value` — 年末總市值, the stored snapshot; the live total for the current year when it has no snapshot; empty for a past year without one
- `dividends` — Σ `received_amount` with pay_date in the year (estimates excluded)
- `yield_on_cost` — 派息 ÷ 成本; `yield_on_value` — 派息 ÷ 總市值; `monthly_dividend` — 派息 ÷ 12; `dividend_yoy` and `invested_yoy` — the sheet's (J−J′)/J′ and (F−F′)/F′ changes

## Freeze or edit a year's figures

```text
Click a year's invested/成本/總市值 cell, type the sheet's number, save
  → PATCH /api/summary/yearly/:market/:year upserts the snapshot
  → null clears a field; clearing the last stored field deletes the row

Click 凍結 on the current year's row
  → POST /api/summary/yearly/:market/:year/freeze stores the computed
    cumulative 成本 and the live 總市值 (invested is left alone)
```

Frozen cells are marked `*` with the snapshot's `updated_at` in the tooltip. Once frozen, later price edits or new trades no longer move that year's row — the spreadsheet's copy-raw-value step becomes explicit.
