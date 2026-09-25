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
| `interest` | 利息 (expected; the 收訖 form can correct it to the amount actually received) |
| `end_date` | `YYYY-MM-DD` text date, required |
| `received_at` | 收訖日 (`YYYY-MM-DD`); NULL while the deposit is still on the books — it stays in 未到期定期 until 收訖 and its interest counts in Month Stat 利息 only once received |
| `credited_asset_id`, `credited_amount` | The cash `manual_assets` row and amount the 收訖 action credited (optional); stored so 取消收訖 reverses exactly |
| `note1`, `note2` | Optional notes |
| `sort_order` | Workbook row order; new entries append |
| `created_at`, `updated_at` | Audit timestamps |

### `family_deposits`

One row per 定期 held on behalf of a family member (the workbook's `Mum`/`Dad` sheets — entered by hand, never imported). Record-only: the money is outside the user's own totals, so no figure derived from `deposits`, `manual_assets`, or `month_items` ever reads this table, and 收訖 has no side effects.

| Column | Meaning |
|---|---|
| `id` | Internal deposit ID |
| `holder` | The family member, e.g. `媽媽`, `爸爸`, `Irene` — free text, required; grouping is by exact string |
| `label` | Bank reference like `SC-9179`, optional |
| `bank` | Bank code, optional |
| `principal` | The deposit's principal, optional |
| `interest` | 利息 (the 收訖 form can correct it to the amount actually received) |
| `start_date` | `YYYY-MM-DD` text date, optional |
| `end_date` | `YYYY-MM-DD` text date, required |
| `received_at` | 收訖日 (`YYYY-MM-DD`); NULL while unreceived — unlike `deposits` it is never auto-set from a past end_date |
| `note` | Optional free text, e.g. the bank's stepped-rate schedule (there is no rate column) |
| `sort_order` | Entry order; new entries append |
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
| `credited_amount` | What the 收訖 bank-in credited (HS cash for HK, `ibkr.usd_cash` for US); NULL = nothing credited, so clearing the receipt reverses exactly |
| `received_price` | 現價 snapshot at receipt, optional |
| `note` | Optional free-text note (imported rows keep the sheet's M formula) |
| `created_at`, `updated_at` | Audit timestamps |

### `year_snapshots`

One row per (market, year) holding the frozen figures the yearly table cannot recompute — the values the spreadsheet workflow used to copy as raw numbers at year end. A NULL column means no override: the yearly row falls back to the figure computed from trades/dividends (or stays empty for a past year's 總市值).

| Column | Meaning |
|---|---|
| `market` | `HK` or `US` |
| `year` | Calendar year |
| `invested` | Override for the year's net invested |
| `cost` | Frozen 年末總成本 (cumulative buy cost) |
| `market_value` | Frozen 年末總市值 |
| `sold_pl` | Hand-entered realized 賣出損益 for the year (may be negative); feeds the yearly table and Month Stat's 投資純利 |
| `updated_at` | When the snapshot was last written |

### `mpf_accounts`

One row per MPF account (from the `MPF` sheet's account table).

| Column | Meaning |
|---|---|
| `id` | Internal account ID |
| `label` | The sheet's account label, e.g. `new type`, `強積金個人帳戶` |
| `trustee` | 受託人, e.g. 宏利 |
| `contributions` | 總供款額, the monthly-edited input |
| `balance` | 帳戶結存, the monthly-edited input |
| `plan_name`, `member_no` | Optional account metadata |
| `sort_order` | Workbook row order |
| `seed_max_rate`, `seed_max_gain` | The workbook's all-time marks, written once at import and never raised; the reported max is `MAX(seed, history, current)` |
| `created_at`, `updated_at` | Audit timestamps |

### `mpf_history`

One row per account per recorded day — `UNIQUE(account_id, recorded_on)`, so a same-day correction replaces the row rather than appending.

| Column | Meaning |
|---|---|
| `id` | Internal history ID |
| `account_id` | Links to `mpf_accounts.id` |
| `recorded_on` | `YYYY-MM-DD` text date |
| `contributions`, `balance` | The account's values on that date |
| `synthetic` | `1` for month-end rows the backend backfilled automatically or seeded at import; `0` for real updates |

When a contributions/balance update arrives in a later month and whole calendar months passed with no rows, the update backfills a synthetic month-end row per empty month carrying the pre-update values — the values that actually stood during those months.

### `app_meta`

A generic key-value table for section-level state that no account row can hold. Currently: `mpf.note` (the MPF page's free-text note), the portfolio-level seeded maxima `mpf.seed_max_rate` / `mpf.seed_max_gain`, the Overview settings `overview.salary` / `overview.pool_rate.<year>`, the manual USD→HKD rate `aia.usd_hkd_rate`, and the 美股 sheet's IBKR account cells `ibkr.now_value` / `ibkr.hkd_cash` / `ibkr.usd_cash`.

| Column | Meaning |
|---|---|
| `key` | The entry's name, unique |
| `value` | Free-text value |

The market sheets' cached `max Balance %` / `max net` cells seed `market.HK.seed_max_percent`, `market.HK.seed_max_amount`, `market.US.seed_max_percent`, `market.US.seed_max_amount` — per-market maxima marks that floor the derived 最高, exactly like the MPF seeds.

Family holder notes live here too: `family.note.<holder>` holds one free-text note per 家人 定期 holder (e.g. Mum's AIA policy lines). The key survives deleting the holder's last deposit, and clearing the note deletes the key.

### `market_history`

One row per market per day a summary is computed — `UNIQUE(market, recorded_on)`, so rebuilding the same market's summary again on the same day replaces the row rather than appending.

| Column | Meaning |
|---|---|
| `id` | Internal history ID |
| `market` | `HK` or `US` |
| `recorded_on` | `YYYY-MM-DD` text date |
| `buy_cost_priced`, `market_value` | The market totals' priced cost basis and market value on that date |
| `synthetic` | `1` for month-end rows backfilled automatically or seeded at import; `0` for real records |

When a summary is computed and whole calendar months passed with no rows, the build backfills a synthetic month-end row per empty month carrying the last-recorded values — the values that actually stood during those months. `import_xlsx` also seeds a synthetic `YYYY-12-31` row per market per `year_snapshots` year that has both 成本 and 總市值, so 最高 has real history from day one, plus one synthetic previous-month-end row per market reconstructed from the sheet's cached `last month` rate and the market's Σ BUY total (the rate matches the sheet exactly; the amount approximates). A market with no priced stock records nothing.

### `bonds`

One row per 債券 (from the `債券` sheet's registry table). The sheet only ever lists active bonds — it deletes them on maturity — so matured rows survive here as history the workbook no longer keeps.

| Column | Meaning |
|---|---|
| `id` | Internal bond ID |
| `label` | The sheet's label, e.g. `silver bond` |
| `issue_no` | 發行編號, e.g. `03GB2710R` — the dedupe key on import |
| `principal` | Face value, e.g. 50000 |
| `maturity_date` | `YYYY-MM-DD` text date (the sheet's `end` column) |
| `received_at` | 本金收訖日 (`YYYY-MM-DD`); NULL while the matured bond's principal return is unconfirmed |
| `credited_asset_id`, `credited_amount` | The cash `manual_assets` row and amount the 收訖 action credited (optional); stored so 取消收訖 reverses exactly |
| `note` | Optional free-text note |
| `sort_order` | Workbook row order; new entries append |
| `created_at`, `updated_at` | Audit timestamps |

Deleting a bond cascades to its `bond_coupons` rows.

### `bond_coupons`

One row per coupon date per bond (from the bond's 付息日 block in the `債券` sheet). `annual_rate`/`per_10k` stay NULL while the sheet shows 待定; `received_amount` records the actual amount paid.

| Column | Meaning |
|---|---|
| `id` | Internal coupon ID |
| `bond_id` | Links to `bonds.id` |
| `pay_date` | 付息日, `YYYY-MM-DD` text date |
| `fixing_date` | 利息釐定日, optional |
| `annual_rate` | 年息率 as a fraction (0.04 = 4%); NULL = 待定 |
| `per_10k` | 每1萬港元債券利息; NULL = 待定 |
| `received_amount` | 實收利息; empty while unreceived |
| `credited_amount` | What the 收訖 bank-in credited to the HS cash row; NULL = nothing credited |
| `note` | Optional free-text note |
| `created_at`, `updated_at` | Audit timestamps |

### `month_stats`

One row per month, keyed by `month` (`YYYY-MM-01`), migrated from the `Month Stat` sheet. `total_assets`/`liquid_assets` are frozen snapshots; NULL means "derive live", which the API only applies to months at/after the current month — historical rows with no sheet B/D value stay NULL.

| Column | Meaning |
|---|---|
| `month` | First of month, `YYYY-MM-DD` text date; unique |
| `start_cash` | F 月初(出糧後): 活期 balance right after salary |
| `salary` | Salary in effect that month, snapshotted at creation |
| `total_assets`, `liquid_assets` | B 總數 / D 流動資產; NULL = live-derived |
| `pool_input` | P Irene+開心Pool |
| `end_cash_override` | Hand-frozen H 月尾 for rows the chain can't reproduce (e.g. 2023-12); NULL = derive |
| `note` | Optional free-text note |
| `created_at`, `updated_at` | Audit timestamps |

### `year_review`

One row per year, keyed by `year`, holding the YearInReview figures that cannot be derived from records — because the workbook entered them by hand (收入, the invested adjustment, sold P/L) or because it deleted the underlying data (matured bonds and deposits drop off the 債券/定期 sheets, so the app can no longer see past years' principal/interest). NULL means "derive live".

| Column | Meaning |
|---|---|
| `year` | Calendar year; unique |
| `income` | The sheet's 收入 cell — hand-entered salary total |
| `invested_adjustment` | `invested = HK net invested + 當年 IBKR 轉入 + invested_adjustment`; seeded as `sheet invested − app HK net invested − year transfers` so the total reproduces the workbook while the transfer part derives live (it folded in US principal, bond purchases, etc.) |
| `salary_raise` | 月薪增幅 override for the 投資目標 target; NULL derives `max(0, last month salary of the year − last of the prior year)`; negative allowed |
| `bond_principal`, `bond_interest` | Year-end 債券 principal held and coupons received; seeded for past years only |
| `deposit_principal`, `deposit_interest` | 定期 principal/interest of deposits ending in the year; seeded for past years only |
| `updated_at` | When the record was last written |

### `month_items` and `month_item_dismissals`

`month_items` holds the month's line items — `adjustment` (G 調整: bank flows that are not spending), `extra_spend` (I−J extras like tax or premiums), `income` (L tail beyond `salary − spend`), `interest` (the manual part of N 利息: bank 活期 interest, promo rebates — the rest derives from deposit/coupon/dividend events), `entertainment` (O 娛樂支出). An `entertainment` item may carry `exclude_from_living`, which also subtracts it from 生活支出 — one entry then serves both the O total and the J exclusion (the sheet typed such amounts in both formulas). Imported items keep the sheet's formula text in `note` (e.g. `=47850-I23+24675`) and have no `auto_key`. Suggested items carry a stable `auto_key` (e.g. `dep-start:<id>`, `div:<id>`); a partial unique index prevents double-accepting, and `month_item_dismissals` (month + auto_key) records ignored suggestions so they stay gone.

### `manual_assets`

The manual Overview cells as rows: `label`, `kind` (`cash` = 活期 like HS/渣打, `asset` = 資產 like Irene/HS人壽), `amount`, `sort_order`. They feed the live 總數/流動資產 derivation and are editable inline on the 總覽 tab.

### `ibkr_transfers`

One row per bank→IBKR transfer (negative amounts record a withdrawal/correction): `transfer_date`, `amount_hkd`, `created_at`. 累計轉入 (美股!B1) is the log's sum, and each year's sum joins the year review's `invested`. The import seeds it with the workbook's cumulative B1 value as one row dated to the first US trade; saving the IBKR form's 轉入 delta appends a dated row.

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
- Market 上月/最高 figures (derived from `market_history` plus current values)
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
- Yearly `invested`, `cost`, `dividends` and the derived yield/YoY columns
- MPF per-account `rate` = (`balance` − `contributions`) ÷ `contributions`, and `gain` = `balance` − `contributions`
- MPF `last month` figures (the latest `mpf_history` row in the previous calendar month)
- MPF `max` rate and `max` gain, each independently the largest of the seed, every history row, and the current values
- MPF portfolio totals/last-month/max, via the as-of merge described below
- Bond status (`matured` once maturity_date is today or past)
- Bond `next_pay_date` (earliest unreceived coupon pay date)
- Coupon expected amount = `per_10k × principal ÷ 10000`
- Coupon status (`PENDING_FIX` while rate/per_10k are unset, `PENDING` once fixed, `RECEIVED` once `received_amount` is set)
- Coupon variance = `received_amount − expected`
- Active principal total and the upcoming unpaid coupon list
- Month Stat 利息/月尾/月支出/生活支出/存/娛樂支出/Changed columns and the item Σ columns
- Month Stat yearly aggregates, running averages, and 投資純利
- Year in Review ledger/investment/asset figures, YoY columns, and blended 回報率 rates
- Live 總數/流動資產 for NULL months (market + deposit principal + bonds + AIA + MPF + manual cells + IBKR account + pool)
- The 總覽 asset table, its Sum and per-row shares, the A13 半流動 ratio, the 半流動資金 block (已定期/活期/total/`total − 25%×流動資產`), and the B1/H1/J1 headline
- IBKR derived figures: 累計轉入 = Σ `ibkr_transfers`, 美股!B7 `(stock value USD + USD cash) × rate + HKD cash`, net and net% against transferred, and the computed-vs-App difference
- 開心Pool balances (chained per year from interest × pool rate − 娛樂 + inputs)

This avoids stale copied totals.

## Feature flows

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
    未實現報酬率 and 未實現金額
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

## Add a deposit in 定期 → 記錄

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

## Load the 定期 views

```text
DepositsView (定期 → 總覽)
  → GET /api/deposits/summary
      → upcoming list (unreceived deposits, earliest end first — an
        end_date in the past without 收訖 shows 已到期未收)
      → active totals, month buckets, bank rollups (all by end_date,
        matching the sheet's own 定期 formulas)
      → year tables for every end year present, plus the current year
  → 收訖 on an upcoming row → POST /api/deposits/:id/receive
      → UPDATE deposits SET received_at (+ interest correction and
        credited_asset_id/credited_amount when 存入活期 is picked)
      → UPDATE manual_assets amount += credited (cash rows only)
      → INSERT the dep-end:<id> adjustment month_item for the end month
        (skipped when already stored or the month has no row)
  → 取消收訖 (history rows) → POST /api/deposits/:id/unreceive reverses
      the credit, deletes the dep-end item, and clears received_at

DepositHistoryView (定期 → 記錄)
  → GET /api/deposits/summary
      → history_years for the year selector
  → GET /api/deposits?year=YYYY&order=desc
      → history rows for the selected year
```

The list split is receipt-based: a deposit leaves 未到期定期 on 收訖 (a stored `received_at`), while the totals and rollups stay point-in-time — they derive from `end_date` like the sheet, so an overdue-but-unreceived deposit shows 已到期未收 in the list yet no longer counts in active principal.

The 手動步驟提醒 checklists (定期 start step / 定期 end step) are static hints for the still-unmigrated `Month Stat`, `回報率`, `Overview`, and money-master bookkeeping in the workbook.

## Load the 家人 → 定期 view

```text
FamilyDepositsView (家人 → 定期)
  → GET /api/family/deposits/summary
      → one section per holder (anyone with deposits or a stored note):
        their note, the unreceived 未到期 list ordered by end date
        (flagging 已到期未收 where past due), 活躍本金 = Σ principal over
        deposits ending in the future
      → history_years for the year selector
  → GET /api/family/deposits?year=YYYY&order=desc
      → 記錄 rows for the selected year
  → holder chips (全部 + one per holder) filter the sections
```

```text
新增定期 / 編輯 in a row's ⋯ menu
  → POST /api/family/deposits or PATCH /api/family/deposits/:id
  → validates holder (required, trimmed), end_date, non-negative amounts —
    the same rules as 定期 minus the rate (there is no rate field; the
    bank's stepped-rate schedule goes in note)

收訖 on an upcoming row
  → POST /api/family/deposits/:id/receive { received_at?, interest? }
  → marks the deposit received (409 if already received) and optionally
    corrects the interest actually paid — nothing else happens: no cash
    manual_assets credit, no month item, no suggestion, and a past
    end_date is never auto-received on create/edit

取消收訖 (history rows) → POST /api/family/deposits/:id/unreceive clears
    received_at (409 if not received)

刪除 → DELETE /api/family/deposits/:id
```

```text
編輯 on a holder's note block
  → PUT /api/family/holders/:holder/note { note }
  → writes the family.note.<holder> app_meta key; empty/null deletes it
  → the note survives deleting the holder's last deposit
```

Isolation: `family_deposits` rows are records only — they never enter 定期!B1, Overview 已定期/半流動資金/總數/流動資產, Month Stat 利息/`interest_auto`/suggestions, or the 回顧 figures, and neither the importer nor the parity check touches the `Mum`/`Dad` sheets.

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

The 現價 entered at receipt is stored on the dividend record only. It does not update the stock's 現價 unless 同時更新現價 is checked, because the receipt price may be recorded on a different day than the price update.

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

## Update MPF figures in MPF → 總覽

```text
MpfView edit form (via 編輯 in the account row's ⋯ menu)
  → PATCH /api/mpf/accounts/:id
  → backend merges the patch and validates (label required, amounts ≥ 0)
  → UPDATE one mpf_accounts row
  → if contributions or balance changed, inside the same transaction:
      → backfill a synthetic month-end mpf_history row per empty elapsed
        month, carrying the pre-update values
      → upsert today's mpf_history row
        (ON CONFLICT account_id+recorded_on → replace)
  → GET /api/mpf reloads accounts, totals, note, and history
```

Editing only metadata (label, trustee, plan/contract/member numbers) writes **no** history row. Deleting a history row via 記錄 → 刪除 removes just that row; last-month and max figures recompute on the next read, since they are derived rather than stored.

Derived on every read:

```text
per-account rate      = (balance − contributions) ÷ contributions   (empty when 0)
per-account last month = latest history row in the previous month
per-account max        = MAX(seed_max_*, all history rows, current),
                          rate and gain tracked independently
portfolio buy/now/%/gain = Σ contributions, Σ balance, derived
portfolio last month     = the as-of portfolio at last month-end
portfolio max            = the largest rate / largest gain independently over
                          the as-of timeline and the app_meta seeds
```

The as-of merge: for every recorded history date (plus last month-end and today), each account contributes its latest row at or before that date — an account with no eligible row falls back to its current values once it existed (`created_at` ≤ date), or is excluded before that. This carries an untouched account forward through months it was never edited and yields a true portfolio max rather than summing per-account peaks that may never have co-occurred.

The 備註 block saves free text via `PATCH /api/mpf/note` into `app_meta`; clearing it removes the entry.

## Load the 債券 view

```text
BondsView (債券 → 總覽)
  → GET /api/bonds/summary
      → Σ active principal, active bonds with their coupon schedules,
        matured bonds for history, and the unpaid coupons ordered by pay date
  → 收訖 on an unreceived matured bond → POST /api/bonds/:id/receive
      → UPDATE bonds SET received_at (+ credited_asset_id/credited_amount
        when 存入活期 is picked)
      → UPDATE manual_assets amount += credited (cash rows only)
      → INSERT the bond-end:<id> adjustment month_item for the maturity
        month (skipped when already stored or the month has no row)
  → 取消收訖 → POST /api/bonds/:id/unreceive reverses the credit, deletes
      the bond-end item, and clears received_at
```

Coupon receipt is per-coupon (each 付息日's `received_amount`); bond 收訖 covers only the principal return. The Active/已到期 split stays maturity-date based — an unreceived matured bond sits in 已到期 flagged 本金未收 until confirmed, and 債券!B1 parity is untouched since the sheet drops matured rows outright.

## Add or edit a bond or coupon

```text
新增債券 / 編輯 in a bond header's ⋯ menu
  → POST /api/bonds or PATCH /api/bonds/:id
  → validate label, principal > 0, valid maturity date

新增付息 / 編輯 in a coupon row's ⋯ menu
  → POST /api/coupons or PATCH /api/coupons/:id
  → validate pay_date and non-negative amounts

刪除 → DELETE /api/bonds/:id (cascades its coupons) or /api/coupons/:id
```

## Fix a coupon's rate or mark it received

```text
釐定 on a 待定 row
  → PATCH /api/coupons/:id { fixing_date, annual_rate, per_10k }
  → status becomes PENDING; the expected amount now shows

收訖 on a pending row
  → PATCH /api/coupons/:id { received_amount, bank_in? }
  → status becomes RECEIVED; variance = received − expected shows
  → banks the amount into the HS cash manual asset (uncheck 存入活期 to skip)
    and records the coupon:<id> adjustment month_item for the pay month
```

Rate fields are entered as a percent (4); the API stores the fraction. Clearing `received_amount` flips the coupon back to pending, reverses the stored bank credit, and deletes the coupon item. A matured bond's coupons stay editable — the receipt history remains completable after maturity.

## Load the AIA view

```text
AiaView (AIA → 總覽)
  → GET /api/aia/summary
      → every policy row with its derived balance_pct and event history,
        totals over non-excluded rows, the in-account display value, HKD
        conversions via the stored rate, and the next premium-due date
```

Two flags reproduce the sheet's two sums: `excluded` rows sit in the AIA account but are not the user's money (the `irene 20%` share), so they drop out of the totals; `in_account` rows feed `display_value`, the figure that should match the AIA portal — rows held in another account (`irene 年金`) count in the totals but not in `display_value`.

## Add or edit a policy, or update the rate

```text
新增保單 / 編輯 in a policy row's ⋯ menu
  → POST /api/aia/policies or PATCH /api/aia/policies/:id
  → validate label and non-negative USD figures
  → changing value_usd refreshes value_updated_at

編輯 on the USD → HKD card
  → PATCH /api/aia/rate
  → writes app_meta key aia.usd_hkd_rate — a manual copy of the
    workbook's Overview!N3 GOOGLEFINANCE cell; all HKD figures derive
    from it and are absent while unset
```

## Record a premium payment or withdrawal

```text
繳費 on a policy row
  → POST /api/aia/events { kind: "payment", event_date, amount_usd, next_pay_date }
  → one transaction: premium_usd += amount, remaining_years −= 1 (when
    set, never below zero), next_pay_date = the submitted date (the form
    proposes current +1 year); the event row snapshots the previous
    next_pay_date/remaining_years

提取 on a policy row
  → POST /api/aia/events { kind: "withdrawal", ... }
  → withdrew_usd += amount

刪除 on an event row
  → DELETE /api/aia/events/:id reverses it: the amount leaves its
    cumulative field and the snapshotted fields restore — delete is the
    undo for a misrecorded event
```

Recording a payment replaces the workbook's three manual edits (buy usd, remaining years, next pay) with one action. Amounts are entered in USD; the `irene 年金` premium paid in HKD is converted before entry.

## Month Stat (總覽 → 月結)

```text
MonthStatView loads GET /api/months/summary + /api/months?year=YYYY
  + /api/months/settings + /api/manual-assets
  → routes/months.rs derives every stored row first, then filters by year,
    so December still gets its end_cash from January
  → NULL total_assets/liquid_assets fill from live totals — only for months
    at/after the current month — with total_assets_live/liquid_assets_live set
```

**Payday entry** (`PATCH /api/months/:ym`, upsert): stores `start_cash` — the 新增月份 form takes no 月初 input, so a created row has none until 重新擷取 or a manual edit; on create the row snapshots the live 總數/流動資產 and defaults `salary` to the `overview.salary` setting. Derived columns: `end_cash = end_cash_override ?? next row's start_cash − this row's salary` (the sheet's `=F(n+1) − <own salary>` — March rows prove it uses the pre-raise literal; `end_cash_override` is the editor's 月尾 field for hand-frozen months the chain can't reproduce, e.g. 2023-12's typed balance — blank/`null` derives), `month_spend = start_cash + Σadjustment − end_cash`, `living_spend = month_spend − Σextra_spend − Σ exclude_from_living entertainment`, `saved = salary − month_spend + Σincome`, `interest = Σ auto events (received deposit interest ending in the month + received coupons + received HK dividends) + Σ interest items`, `entertainment = Σ entertainment items`, `Changed = next − this` for both asset columns — the last stored row, while it is the current month, diffs the live totals instead (the sheet's last-row C/E cells read live `B1`/`H1`), so the in-progress month's change counts in the yearly Σ總數+.

**Suggestions** (`GET /api/months/:ym` → `suggestions`, only for months ≥ current month): `dep-start:<id>` −principal (only with `start_date`), `dep-end:<id>` +principal+interest, `trade:<id>` −BUY/+SELL total (HK only), `div:<id>` (HK only — US dividends stay inside IBKR) / `coupon:<id>` received amounts (收訖 auto-creates these items — suggestions now only catch months that had no row at receipt time), `aia-pay:<id>` premium × USD→HKD (skipped without a rate), `pool-input` −pool_input. Accepting stores an item with the same `auto_key` (second accept → 409); dismissing writes a tombstone so the suggestion never returns. IBKR transfers and deposits without `start_date` stay manual items. `interest_auto` lists the auto 利息 components (per deposit/coupon/HK dividend, with label and amount) that the derived `interest` adds on top of `interest` items — marking a dividend or coupon 收訖, or a deposit's 收訖, updates the figure with no month write; unreceived components carry `received: false` and preview muted in the breakdown without counting (with the expected/estimated amount, or `—` while 待定).

**Frozen vs live**: `改為即時` patches both totals to `null` (live); `重新擷取` (`recapture: true`) re-snapshots the totals and `start_cash` (月初 = the live 活期 sum, Σ `cash` manual assets — an explicit `start_cash` in the same patch still wins). The pool balance chains per year: `balance(y) = balance(y−1) + Σinterest×rate(y) − Σentertainment + Σpool_input`, where `rate(y)` is the exact year's `overview.pool_rate.<y>` else the latest earlier year; years with no rate contribute zero pool income.

**Settings**: `PATCH /api/months/settings` writes `overview.salary` and `overview.pool_rate.<pool_rate_year>` to `app_meta`; the manual Overview cells live in `manual_assets`. Deposits carry `start_date` (optional, `YYYY-MM-DD`) which enables `dep-start` suggestions.

## Load the 總覽 view

```text
OverviewView loads GET /api/overview
  → routes/overview.rs reuses the live-totals components, reads the manual
    asset rows, and builds the IBKR block from the ibkr.* app_meta keys and
    the ibkr_transfers log
  → the response carries the B1/H1/J1 headline, the asset table with each
    row's share of the Sum, the 半流動資金 block, and the IBKR block
```

The asset table mirrors `Overview!A3:C10` — 港股 / 債券 / 基金 / MPF / manual `asset` rows / IBKR — with the C column as each row's share of the Sum. The 半流動資金 block mirrors `A14:C18`: 已定期 (Σ active deposit **principal**, matching `定期!B1`), the manual `cash` rows, 活期, the total, `total ÷ (港股 + 債券 + total + IBKR)` (A13), and `total − 25% × 流動資產` (C14). Manual rows expose their `manual_assets` id so the amount edits inline via `PATCH /api/manual-assets/:id`.

`總數` = Sum + 半流動資金 (equivalently 港股 + IBKR + 定期 + 債券 + 基金 + MPF + manual assets + 活期). `流動資產` = 港股 + 半流動資金 + 債券 + IBKR − 開心Pool. J1 = `流動資產 ÷ (薪金 × 100)`. The headline strip shows a fourth tile with the live 開心Pool balance (the sheet's `G10` figure — a live balance, not an average) beside them, since 流動資產 subtracts it. USD-denominated figures (US stocks, AIA, IBKR USD cash) convert at the stored `aia.usd_hkd_rate`; rows are absent while no rate is set.

The 過去 12 個月平均 card mirrors `Overview!F3:G10` + `H6`: averages of 總數增加 / 支出 / 生活支出 / 存 / 利息 over the 12 most recent completed month rows (`month <` the current month; months with no value are skipped per column), and 生活預算 = `ROUNDUP(生活支出_avg × 1.05, −2)`, shown green while it stays below `流動資產 × 0.0001 × 30 + 9000` and red while it exceeds it (the sheet's 預測 threshold).

The 投資目標 card mirrors `Overview!J22:N27`: the header shows the mean of `invested` over the last three completed years (J22), and each year row shows `invested`, `target`, `remain` (current year only: `target − invested`), and `增長` — invested YoY for completed years, `(target − prior invested) ÷ prior invested` for the current year. Unlike the sheet's per-year formulas, one formula retro-computes every year:

```text
target(Y) = invested(Y-1) − interest(Y-1) − pool_spend(Y-1)×0.7
          + 月薪增幅(Y)×0.5×12 + interest(Y) + pool_spend(Y)×0.7
```

— the two entertainment terms net to 70% of the *year-over-year increase* in fun spending, so a steady level adds nothing new. `月薪增幅` comes from the stored month salaries (`last salary of the year − last of the prior year`, floored at 0) unless a `year_review.salary_raise` override is set; every other input already derives on the 回顧 rows.

## Edit the IBKR figures in 美股 → 總覽

```text
Click 編輯 on the IBKR card → PATCH /api/ibkr with the transfer delta/date and
    the three account fields
  → routes/overview.rs validates non-negative finite numbers, appends
    `transfer_hkd` (any finite delta; negative undoes an entry) to
    `ibkr_transfers` dated `transfer_date` (default today), writes each
    provided account field to its ibkr.* app_meta key (null clears), and
    returns the block with the derived figures recomputed
```

`ibkr.now_value` is the account total as the IBKR app displays it — its implied FX rate differs from `aia.usd_hkd_rate`, so it stays a manual input. `computed_total_hkd` is the sheet's `美股!B7` `(美股市值 USD + USD cash) × rate + HKD cash`; `vs_now_value` is the cross-check gap between the two.

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
- `美股` — plus the IBKR account cells `B1` (累計轉入 HKD → one `ibkr_transfers` row dated to the first US trade, seeded only while the log is empty), `B2` (IBKR App 現值), `B4`/`B5` (HKD/USD cash) → `ibkr.*` `app_meta` keys, seeded only while unset
- `定期Info` (表_定期List) and `定期` (cached aggregates for parity)
- `MPF` — the account table under the 總供款額/帳戶結存 headers; the fund-details table below it and the remark row are ignored
- `債券` — the registry table under the `end` header (label / 發行編號 / principal / maturity), then each bond's coupon block under its 發行編號 label line: 付息日 / 利息釐定日 / 年息率 / 每1萬利息 / cached 利息; `待定` cells import as NULL
- `AIA` — the D–O policy block: rows with numeric `buy usd`/`now usd` cells and a label or policy number; a blank label inherits the plan name above it, and the remark cells (L onward) join into `note`. A row resuming after a blank gap imports `in_account` false, and the row whose removal reconciles Σ premium/Σ value to the cached `buy usd`/`now usd` cells imports `excluded` — `irene 20%` and `irene 年金` respectively today; unresolvable cases flag nothing and report a warning
- `Overview` — `N3`, the cached USD→HKD rate, which seeds `aia.usd_hkd_rate` once; plus `E1` (salary → `overview.salary`), `N8` (current-year pool rate → `overview.pool_rate.<year>`), the manual cells `B7`/`B8`/`B16`/`B17` → `manual_assets` (all seed only when unset), and the 投資目標 block's `J:K` year/invested cells → `year_review.invested_adjustment` for years the YearInReview sheet has no block for (e.g. 2023's +110000), never overwriting a stored value
- `Month Stat` — monthly rows since 2023-12: F/O/P store as-is (blank → 0), B/D store their cached literal unless the cell links to `Overview!` (then NULL = live), G/I−J/L-tail materialize as `month_items` keeping the formula text in `note`, and N 利息 imports as an `interest` item holding `sheet N − all in-month auto events` (the unexplained remainder, labeled 其他利息 — skipped for blank cells and exact matches; unreceived deposits are still subtracted so their later 收訖 doesn't double-count a pre-typed cell); each row's salary is recovered from its L formula's leading literal (fallbacks: previous row's H trailing literal, latest known, `Overview!E1`); the yearly block feeds only the historical pool-rate solve — past years' `overview.pool_rate.<y>` are derived from the sheet's M/G/H/N chain (e.g. 2024 → 0.53, 2025 → 0.425); date-only rows are skipped
- The market sheets' year blocks (B year, C net invested, F 成本, H 總市值) and `YearInReview`'s 股票 rows — seeded into `year_snapshots` for years before the current one; the current year stays live

For each dividend row the importer stores J (stock), K (pay date, Excel serial dates accepted), M (派息 amount, cached value), and O (股數 snapshot). The remaining snapshots are recovered from the cached rates the same way the sheet computed them — `buy_cost = M ÷ L`, `received_price = M ÷ (N × O)` — falling back to trade-derived snapshots when a rate is absent. The M formula text, when present, is kept in `note`. Rows with an N rate, or a pay date already past, import as received; future rows without it import as pending estimates.

The import is idempotent. A second run skips trades already stored with the same stock, date, type, shares, and total, deposits already stored with the same label, end date, principal, and interest, dividends already stored with the same stock, pay date, and amount, bonds already stored with the same 發行編號 (or the same label/principal/maturity when the sheet has none), and coupons already stored with the same bond and pay date. Year snapshots merge at field level: a stored value — seeded or edited — is never overwritten, while empty fields on an existing row are filled from the workbook. MPF accounts are keyed by label: a second run skips them entirely. AIA policies are keyed by `policy_no` (falling back to label + premium + value when the sheet has none), so a second run skips them too — and `aia.usd_hkd_rate` seeds only when unset, never clobbering an edit.

A deposit whose `end_date` is already past imports as received (`received_at = end_date`); future deposits stay unreceived — the same heuristic the dividend and coupon imports use.

A bond coupon whose pay date is already past imports as received with the sheet's cached interest value as `received_amount`; future coupons stay unreceived — the same heuristic the dividend import uses. The sheet deletes matured bonds outright, so nothing is ever un-imported: history the app keeps simply stops appearing in later imports.

Each new MPF account also seeds one synthetic `mpf_history` row at last month-end. With exactly two accounts, the per-account last-month rates plus the portfolio's cached last-month rate+gain pin down the actual month-end contributions exactly, and the seeded balance is `contributions × (1 + rate)`; with any other account count the seed uses the current contributions, so the rate stays exact while the net gain approximates. The sheet's per-account max rate seeds `seed_max_rate`, and a reconstructed `rate × contributions` seeds `seed_max_gain`. The portfolio-level `max`/`last month` cells go to `app_meta` as floors, because per-account history cannot rebuild them.

The workbook is never modified.

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

## Load the 總覽 → 年結 view

```text
YearReviewView loads GET /api/year-review
  → routes/year_review.rs loads month_stats + items + pool rates,
    year_snapshots, the year_review records, HK trades + dividends,
    deposits, bonds + coupons, and manual_assets
  → calc.rs::year_review_rows builds one row per year from the union of
    years present in those inputs
  → the frontend renders one block per year mirroring the sheet's
    A–D ledger, E–G investment and H–M asset groups
```

Each row reproduces the workbook block:

- Ledger group — 總數+/平均總數+ (Σ `total_change` and ÷12), 支出/平均支出 (Σ `month_spend`), 生活平均支出 (the sheet's `AVERAGE` over recorded living spends), 開心 Pool 收入/支出/結餘 (chained `pool_balances`), plus the D-column YoY ratios.
- Investment group — 利息回報 and 平均回報 (`interest ÷ 12` flat, even mid-year), 投資P/L (HK `sold_pl`), 投資純利 (`interest + sold_pl`), IBKR 轉入 (Σ the year's `ibkr_transfers`), invested (`HK net invested + 當年轉入 + invested_adjustment`), invested % (`invested ÷ (收入 + 利息回報)`), Irene + 開心 Pool (`pool_input_sum + pool_balance`), plus the G-column YoY ratios.
- Asset group — 債券 (principal held in the year + coupons received), 股票 (yearly HK row: cost/dividends/rates/year-end value; 派息 counts received only), 定期 (deposits ending in the year, `End ≤ today`), the three blended 回報率 rates (income-returns ÷ bond+cost, all-returns ÷ bond+市值, income-returns ÷ bond+市值), 收入, 平均收入 (÷12), and 存/平均存/存% (`income − spend`, ÷income), plus the 收入 YoY.

`pool_income`/`pool_spend`/`pool_balance`, `interest`, `interest_avg` and `irene_pool` always read 0 without month data — the sheet's own formulas do the same.

## Edit Year in Review manual figures

```text
Click 收入 / invested 調整 / 月薪增幅 / 投資P/L / a 債券 or 定期 cell, type
the value, save
  → PATCH /api/year-review/:year
  → manual fields (income, invested_adjustment, raise, bond/deposit
    overrides) upsert the year_review row; clearing the last field deletes it
  → sold_pl instead upserts year_snapshots.sold_pl for HK — the same
    cell the yearly table edits — so both views agree
  → null clears a field; the next read re-derives it
```

Cells holding a stored value are marked `*`; empty input clears the field back to live derivation. 月薪增幅 normally shows the salary-derived figure (last stored month salary of the year minus the prior year's, floored at 0); an override marks `*` — negative allowed — and clearing resumes derivation. Clearing a seeded past-year bond/deposit override exposes the true records — a matured bond deleted from the workbook will then show 0/absent, which is honest history rather than a bug.

## Year-end actions

Once a year, around Dec 31 (or early January):

1. **Freeze the ending year's stock figures** — 股票 → 總覽 → 每年總覽, click 凍結 on the year row, once per market (HK and US). This stores the cumulative 成本 and the live 總市值 into `year_snapshots`. A past year without a snapshot cannot recompute its year-end 市值 — there is no historical price series — so the column would go blank. `invested` needs no freeze (trades always derive it) and 凍結 never touches `sold_pl`.
2. **Enter the year's realized P/L** — the 賣出損益 cell on each market's yearly row, or the 投資P/L cell on 總覽 → 年結 (both write `year_snapshots.sold_pl` for that market-year). Enter 0 when nothing was sold: 投資純利 (`interest + sold_pl`) stays absent until a value exists.
3. **Enter 收入** on 總覽 → 年結 — the year's hand-entered salary total. Also set **invested 調整** if `invested` should include money outside HK trades and the year's IBKR 轉入 (which now derives from the transfer log — the sheet folded US principal and bond purchases into it); the app keeps it as a separate add-on so the two stay reconcilable.

At year start nothing is required:

- The new year's rows appear automatically once the first month row or trade exists.
- The 開心Pool chain resets per year; its rate falls back to the latest earlier year — set the new year's rate in Month Stat settings only if it changed.
- Update 薪金 (`overview.salary`) only if salary changed — new month rows snapshot it.
- No 債券/定期 overrides are needed for future years: the app keeps matured history, so those columns keep deriving. The seeded overrides exist only for years the workbook deleted.

The month ledger, deposits, dividends, coupons, and MPF all roll across the boundary on their own.

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
  → recomputes MPF per-account 總供款額/帳戶結存/回報率 and portfolio
    buy/now/回報率/淨收益 strictly against the MPF sheet's cached cells,
    and the seeded last-month/max figures with a loose tolerance, since
    they reconstruct values the sheet stored only as rates
  → sums active bond principal and compares it to 債券!B1, then each
    coupon's effective amount (received else per_10k-derived expected)
    against the sheet's cached interest cell; 待定 rows carry no cached
    figure and are skipped
  → recomputes AIA per-policy premium/value/balance_pct and the totals
    (buy/now USD, overall return, display value, HKD cells via the stored
    rate) against the sheet's cached G/H/I cells and B1–B9 block; cells
    the sheet leaves blank skip quietly
  → compares Month Stat stored fields (F/O/P/B/D) and derived columns
    (H/I/J/L/C/E plus N — derived interest vs the sheet's cell)
    (H/I/J/L/C/E) per month, the yearly block per year, the row-8 running
    averages, and the seeded settings/manual cells against the sheet's
    cached values; `interest_avg` averages only months carrying a value
    (non-NULL start_cash or nonzero interest), matching AVERAGE's
    non-empty-cell semantics
  → recomputes the Overview A3:C18 block (asset rows by label, Sum, the C
    shares, A13, the 半流動資金 cells, C14) and the B1/H1/J1 headline, plus
    the 美股 IBKR header cells; module-derived figures (債券/基金/MPF/已定期)
    count as real differences while cells downstream of live prices or
    user-edited manual inputs report informational. The J22:N27 投資目標
    block compares J22, the K invested cells, and completed-year N growth
    normally; the L/M cells and the current-year N report informational
    since the unified formula supersedes the sheet's per-year ones
  → rebuilds each YearInReview block and compares every cell it parses:
    ledger sums, interest, seeded manual figures (收入/invested/投資P/L),
    past-year bond/deposit overrides, and the derived blends; the
    current-year block and the sheet's own stale stock cells report
    informational
```

Month Stat parity reports informational rows rather than failures where the sheet legitimately diverges: the live `Overview!`-linked current row (stale cached B/D/C/E), any row edited after import (`updated_at > created_at`, propagated to the previous row whose derived cells depend on it), and rows whose H cell is a hand-frozen literal rather than the `=F(next) − salary` chain (2023-12, 2024-01). YearInReview parity reports informational where the sheet diverges by construction: the current year's live-moving block, its stale frozen 股票 cells (which disagree with the market sheet's own year block), its pending-inclusive 派息 total, and the current year's 債券 figure (the matured bond the registry no longer holds).

This verifies that the database reproduces the spreadsheet's trade-derived figures, deposit rollups, dividend totals, MPF figures, bond figures, and AIA figures. Deposit parity is point-in-time: the cached values reflect the workbook's last recalculation, so a deposit that matures after that point shows as a difference. MPF parity is the same: once the app is edited after import, its current figures legitimately diverge from the frozen sheet. Bond parity too: the sheet's `Total` cell only covers bonds it still lists, so a bond that matured since the workbook last recalculated — or an already-received coupon recorded with a different amount — shows as an informational difference.

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
| Yearly rollup columns | Displays them; inline edit + freeze button | Calculates them; stores snapshots |
| MPF account editing | Form | Yes, validates + records history |
| MPF last-month/max figures | Displays them | Derives them from history + seeds |
| AIA policies + events | Forms | Yes, validates; events update policies atomically |
| AIA totals/HKD figures | Displays them | Calculates them from the stored rate |
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
