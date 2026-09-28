# 總覽 flows — Overview, Month Stat, Year in Review, IBKR

Part of the [data-flow guide](DATA_FLOW.md). Table definitions: [`month_stats`, `month_items`, `year_review`, `manual_assets`, `ibkr_transfers`, `app_meta`](database.md). See also [MONTH_STAT_OVERVIEW.md](MONTH_STAT_OVERVIEW.md) for the workbook analysis behind these flows.

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

Live 總數/流動資產 count deposits at principal only and include IBKR cash, matching the workbook formulas exactly.

The 過去 12 個月平均 card mirrors `Overview!F3:G10` + `H6`: averages of 總數增加 / 支出 / 生活支出 / 存 / 利息 over the 12 most recent completed month rows (`month <` the current month; months with no value are skipped per column), and 生活預算 = `ROUNDUP(生活支出_avg × 1.05, −2)`, shown green while it stays below `流動資產 × 0.0001 × 30 + 9000` and red while it exceeds it (the sheet's 預測 threshold).

The 投資目標 card mirrors `Overview!J22:N27`: the header shows the mean of `invested` over the last three completed years (J22), and each year row shows `invested`, `target`, `remain` (current year only: `target − invested`), and `增長` — invested YoY for completed years, `(target − prior invested) ÷ prior invested` for the current year. Unlike the sheet's per-year formulas, one formula retro-computes every year:

```text
target(Y) = invested(Y-1) − interest(Y-1) − pool_spend(Y-1)×0.7
          + 月薪增幅(Y)×0.5×12 + interest(Y) + pool_spend(Y)×0.7
```

— the two entertainment terms net to 70% of the *year-over-year increase* in fun spending, so a steady level adds nothing new. `月薪增幅` comes from the stored month salaries (`last salary of the year − last of the prior year`, floored at 0) unless a `year_review.salary_raise` override is set; every other input already derives on the 回顧 rows.

The 策略 card mirrors `Overview!J3:K7`, re-partitioning 總數 into liquidity tiers: 不可動用 = `salary × 6` (the sheet's `N6`, a fixed 6-month multiplier), 可動用 = `半流動資金 − 不可動用` (negative allowed, shown red), 短期可取回 = `港股 + 債券 + IBKR + Σ manual asset rows with liquidity short`, 長期可取回 = `基金 + MPF + Σ manual asset rows with liquidity long`. 可動用/不可動用 are `—` while no salary is stored; 短期/長期可取回 are `—` while `aia.usd_hkd_rate` is unset (their IBKR and 基金 terms need the rate — a partial tier would look plausible and be wrong). While all four are present they sum to 總數 (`K4+K5 = B14`, `K6+K7 = B10`). Manual `asset` rows carry a `liquidity` flag (`short`/`long`, default `long`) editable in the Month Stat 資產 editor; `cash` rows carry the column but it has no effect.

## The 預測 forecast grid (Overview!A20:H36)

```text
OverviewView loads GET /api/forecast together with GET /api/overview
  → routes/forecast.rs derives seven month columns (current .. +6) from the
    stored rows: nothing in the grid is persisted except forecast_items
```

The card mirrors the sheet's row order — `ref check`, `半流動`, `活期`, `定期 + SC`, `start`, `salary`, `支出`, `定期 finish`, `定期 HS`, `SC高息馬拉松`, `利息`, `Tax/基金/醫療保險`, `股票`, `繳費`, `TBC - 定期 end`, `TBC` — with months as columns and `—` for absent figures. The sheet's monthly ritual (re-anchor `B22`/`B24`/`B25`, copy `C28:H36` left, fix `H`) disappears: the window rolls by itself and every line derives:

- `start` — the current month's `start_cash` (月初出糧後, entered in 月結), falling back to the live 活期 sum while unset; later columns chain from the previous column's `cash`
- `salary`/`spend` — `overview.salary` and −`living_budget` from the averages block
- `定期 finish` — Σ `principal` of deposits whose `end_date` falls in the month (the `2026回報率`/`定期Info` SUMIFs)
- `利息` — deposit interest ending + HK dividends + bond coupons at their received or expected figures + `interest` items; each column also carries `interest_components` so clicking the cell lists every auto receipt (定期/債息/派息, 已收 or 預計)
- `繳費` — −`forecast.bill_amount` (default `2158`, editable in 月結 settings) every Jan/Apr/Jul/Oct; a `bill` item in the month replaces the default
- `TBC - 定期 end` — Σ `−amount` of deposit plans returning that month: `return_month` when set, else `hs_deposit` → +3 months, `sc_deposit` → +4 (the sheet's row-35 lags)
- `定期 + SC` (`locked`) — chains from Σ active deposit principal: `locked(m−1) − finish − plan placements − returns`
- `活期` (`cash`) and `半流動` (`cash + locked`); `ref check` = `半流動 − 流動資產 ÷ 4`, rendered red while negative

```text
Click a plan cell → the editor lists that (month, kind)'s items
  → 儲存 PATCHes /api/forecast-items/:id; 刪除 DELETEs it; the add row POSTs
    /api/forecast/:ym/items — each placement is its own row (the sheet's
    =7000+6000 cell is two items); deposit kinds accept a return-month override
轉為定期 on an hs_deposit/sc_deposit item → DepositForm prefilled (principal
  = −amount, bank HS/SC from the kind, end_date = return month's last day)
  → POST /api/forecast-items/:id/convert creates the deposit and deletes the
    plan in one transaction, so the derived return/lock never double-counts
```

Items whose month sits outside the window stay stored: future ones slide in as the months pass; stale ones (month already past) fire nothing — matching the sheet's shifted-out cells, and preventing a real deposit created without conversion from being double-counted by its leftover plan.

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

**Settings**: `PATCH /api/months/settings` writes `overview.salary` and `overview.pool_rate.<pool_rate_year>` to `app_meta`; the manual Overview cells live in `manual_assets`. Deposits carry `start_date` (optional, `YYYY-MM-DD`) which enables `dep-start` suggestions. Settings and `deposits.start_date` are seeded once by import and never overwritten afterward.

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
