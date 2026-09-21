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

### `year_snapshots`

One row per (market, year) holding the frozen figures the yearly table cannot recompute — the values the spreadsheet workflow used to copy as raw numbers at year end. A NULL column means no override: the yearly row falls back to the figure computed from trades/dividends (or stays empty for a past year's 總市值).

| Column | Meaning |
|---|---|
| `market` | `HK` or `US` |
| `year` | Calendar year |
| `invested` | Override for the year's net invested |
| `cost` | Frozen 年末總成本 (cumulative buy cost) |
| `market_value` | Frozen 年末總市值 |
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

A generic key-value table for section-level state that no account row can hold. Currently: `mpf.note` (the MPF page's free-text note) and the portfolio-level seeded maxima `mpf.seed_max_rate` / `mpf.seed_max_gain`.

| Column | Meaning |
|---|---|
| `key` | The entry's name, unique |
| `value` | Free-text value |

The market sheets' cached `max Balance %` / `max net` cells seed `market.HK.seed_max_percent`, `market.HK.seed_max_amount`, `market.US.seed_max_percent`, `market.US.seed_max_amount` — per-market maxima marks that floor the derived 最高, exactly like the MPF seeds.

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
| `note` | Optional free-text note |
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
      → upcoming list (end_date > today, earliest first)
      → active totals, month buckets, bank rollups
      → year tables for every end year present, plus the current year

DepositHistoryView (定期 → 記錄)
  → GET /api/deposits/summary
      → history_years for the year selector
  → GET /api/deposits?year=YYYY&order=desc
      → history rows for the selected year
```

Status and rollups are point-in-time: they derive from today, so a deposit moves from 未到期定期 to history on its end date without any stored change.

The 手動步驟提醒 checklists (定期 start step / 定期 end step) are static hints for the still-unmigrated `Month Stat`, `回報率`, `Overview`, and money-master bookkeeping in the workbook.

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
  → PATCH /api/dividends/:id { received_amount, received_price? }
  → status becomes received; both rates now use the received amount
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
```

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
  → PATCH /api/coupons/:id { received_amount }
  → status becomes RECEIVED; variance = received − expected shows
```

Rate fields are entered as a percent (4); the API stores the fraction. Clearing `received_amount` flips the coupon back to pending. A matured bond's coupons stay editable — the receipt history remains completable after maturity.

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
- `MPF` — the account table under the 總供款額/帳戶結存 headers; the fund-details table below it and the remark row are ignored
- `債券` — the registry table under the `end` header (label / 發行編號 / principal / maturity), then each bond's coupon block under its 發行編號 label line: 付息日 / 利息釐定日 / 年息率 / 每1萬利息 / cached 利息; `待定` cells import as NULL
- `AIA` — the D–O policy block: rows with numeric `buy usd`/`now usd` cells and a label or policy number; a blank label inherits the plan name above it, and the remark cells (L onward) join into `note`. A row resuming after a blank gap imports `in_account` false, and the row whose removal reconciles Σ premium/Σ value to the cached `buy usd`/`now usd` cells imports `excluded` — `irene 20%` and `irene 年金` respectively today; unresolvable cases flag nothing and report a warning
- `Overview` — only `N3`, the cached USD→HKD rate, which seeds `aia.usd_hkd_rate` once (a user edit is never overwritten)
- The market sheets' year blocks (B year, C net invested, F 成本, H 總市值) and `YearInReview`'s 股票 rows — seeded into `year_snapshots` for years before the current one; the current year stays live

For each dividend row the importer stores J (stock), K (pay date, Excel serial dates accepted), M (派息 amount, cached value), and O (股數 snapshot). The remaining snapshots are recovered from the cached rates the same way the sheet computed them — `buy_cost = M ÷ L`, `received_price = M ÷ (N × O)` — falling back to trade-derived snapshots when a rate is absent. The M formula text, when present, is kept in `note`. Rows with an N rate, or a pay date already past, import as received; future rows without it import as pending estimates.

The import is idempotent. A second run skips trades already stored with the same stock, date, type, shares, and total, deposits already stored with the same label, end date, principal, and interest, dividends already stored with the same stock, pay date, and amount, bonds already stored with the same 發行編號 (or the same label/principal/maturity when the sheet has none), and coupons already stored with the same bond and pay date. Year snapshots merge at field level: a stored value — seeded or edited — is never overwritten, while empty fields on an existing row are filled from the workbook. MPF accounts are keyed by label: a second run skips them entirely. AIA policies are keyed by `policy_no` (falling back to label + premium + value when the sheet has none), so a second run skips them too — and `aia.usd_hkd_rate` seeds only when unset, never clobbering an edit.

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
- `sold_pl` — reserved, empty for now
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
```

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
