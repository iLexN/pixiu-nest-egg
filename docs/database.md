# Database tables

Part of the [data-flow guide](DATA_FLOW.md). One section per SQLite table in `data/wealth.db`, plus the figures the backend derives on read and never stores.

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

A generic key-value table for section-level state that no account row can hold. Currently: `mpf.note` (the MPF page's free-text note), the portfolio-level seeded maxima `mpf.seed_max_rate` / `mpf.seed_max_gain`, the Overview settings `overview.salary` / `overview.pool_rate.<year>` / `overview.semi_liquid_target` (the 半流動資金 buffer's target share of 流動資產, `0.25` while unset), the forecast's 差餉 charge `forecast.bill_amount` (2158 while unset), the manual USD→HKD rate `aia.usd_hkd_rate`, and the 美股 sheet's IBKR account cells `ibkr.now_value` / `ibkr.hkd_cash` / `ibkr.usd_cash`.

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

The manual Overview cells as rows: `label`, `kind` (`cash` = 活期 like HS/渣打, `asset` = 資產 like Irene/HS人壽), `liquidity` (`short`/`long`, default `long`), `amount`, `sort_order`. They feed the live 總數/流動資產 derivation and are editable inline on the 總覽 tab. `liquidity` places `asset` rows in the Overview 策略 block's 短期可取回 or 長期可取回; `cash` rows carry it (the column is NOT NULL) but it has no effect on them.

### `ibkr_transfers`

One row per bank→IBKR transfer (negative amounts record a withdrawal/correction): `transfer_date`, `amount_hkd`, `created_at`. 累計轉入 (美股!B1) is the log's sum, and each year's sum joins the year review's `invested`. The import seeds it with the workbook's cumulative B1 value as one row dated to the first US trade; saving the IBKR form's 轉入 delta appends a dated row.

### `forecast_items`

One row per planned cash line of the Overview 預測 grid (`A20:H36`): `month` (`YYYY-MM-01` of the month the line lands in), `kind`, signed `amount`, `return_month`, `note`, `sort_order`. `hs_deposit`/`sc_deposit` are planned lockups (stored negative) that schedule a derived return at `return_month` or the default lag (+3 / +4 months); `bill` overrides the quarter-month 差餉 line (`forecast.bill_amount` in `app_meta`, default 2158); `interest`, `tax`, `stock` and `other` are plain lines. Everything else in the grid — start, salary, spend, deposit finish, returns, the locked 定期+SC chain — is derived on read, and `POST /api/forecast-items/:id/convert` turns a plan into a real `deposits` row and deletes the item in one transaction.

## Values not stored

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
- The whole forecast grid: per-month `cash`/`locked`/`semi_liquid`/`ref_check`, deposit finish/interest sums, plan `deposit_return`s, and the quarter-month `bill` line
- MPF portfolio totals/last-month/max, via the as-of merge described in [investments.md](investments.md)
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
