# month-stat Specification

## Purpose
Tracks the user's month-by-month cash bookkeeping — the payday bank balance, non-spending bank flows, and interest/entertainment/pool figures — so spending, saving and the 開心Pool balance fall out of balance differences without transaction logging, replacing the workbook's `Month Stat` sheet.

## Requirements

### Requirement: Monthly ledger rows

The system SHALL store one row per calendar month (`YYYY-MM-01`) carrying `start_cash` (月初出糧後 — the 活期 bank total measured right after salary lands, entered manually in the month editor or captured from the live 活期 sum by 重新擷取), `salary` (the salary in effect that month, snapshotting the current salary setting when the row is created), `pool_input` (Irene + 開心 Pool contributions), and `note`. The add-month form SHALL NOT collect 月初 — a created row stores no `start_cash` unless the patch supplies one. 利息 is NOT stored on the row — it is derived from auto events plus the month's `interest` items. 娛樂支出 is NOT stored on the row — it is the sum of the month's `entertainment` items. The API SHALL expose `GET /api/months` (optionally filtered by `year`), `GET /api/months/:ym`, `PATCH /api/months/:ym` (creating the row when absent), and `DELETE /api/months/:ym`.

#### Scenario: Create a month row at payday

- **WHEN** the user patches `2026-10-01` with `start_cash` `35000`
- **THEN** the row is stored with `salary` defaulting to the current salary setting

#### Scenario: Adding a month stores no 月初

- **WHEN** the user adds month `2026-10` from the 新增月份 form (which has no 月初 input)
- **THEN** the row is created with `start_cash` NULL — the column shows `—` until 重新擷取 or a manual edit fills it

#### Scenario: Update figures

- **WHEN** the user patches a stored month with `pool_input` `1500`
- **THEN** the row reports that value and derived figures recompute on the next read

### Requirement: Derived monthly figures

Each month SHALL derive on read, never storing: `end_cash` (月尾出糧前) = the stored `end_cash_override` when present, else the next stored month's `start_cash` minus this month's own `salary`, absent while no later row exists; `month_spend` (月支出) = `start_cash` + Σ adjustment items − `end_cash`, absent without `end_cash`; `living_spend` (生活支出) = `month_spend` − Σ extra_spend items − Σ `exclude_from_living` entertainment items; `saved` (存) = `salary` − `month_spend` + Σ income items; `interest` (利息) = Σ auto interest components + Σ `interest` items; `entertainment` (娛樂支出) = Σ entertainment items; `total_change` = the next month's `total_assets` − this month's — the last stored row, while it is the current month, diffs the live 總數 instead (the sheet's last-row C cell reads live `B1`); `liquid_change` likewise on `liquid_assets` (live `H1`); and `living_yoy` (the sheet's K column) = `(living_spend − living_spend of the same month one year earlier) / living_spend`, absent while either side is missing or `living_spend` is zero.

#### Scenario: Spending falls out of the balance difference

- **WHEN** month `2026-09-01` has `start_cash` `30000`, `salary` `52700`, adjustments `+8009.3`, and the next month has `start_cash` `32000`
- **THEN** September reports `end_cash` `−20700`, `month_spend` `58709.3`, `living_spend` `58709.3` minus its extra_spend and excluded entertainment items, and `saved` `−6009.3` plus its income items

#### Scenario: Latest month has no month-end yet

- **WHEN** a month is the latest stored row
- **THEN** `end_cash`, `month_spend`, `living_spend` and `saved` are absent until the next month's `start_cash` exists

#### Scenario: Hand-frozen month-end wins over the chain

- **WHEN** month `2023-12-01` stores `end_cash_override` `24610.32` (the real bank balance typed before the 月初 convention existed) with `start_cash` `64925.18`, `salary` `45500`, and the next month has `start_cash` `26391.71`
- **THEN** December reports `end_cash` `24610.32` — not the chain's `−19108.29` — so `month_spend` and `saved` match the hand-kept history; clearing the override with `null` restores derivation

#### Scenario: Flagged entertainment also leaves living spend

- **WHEN** a month has `month_spend` `25000`, an extra_spend item `12000` labeled `AIA`, and an entertainment item `4700` flagged `exclude_from_living`
- **THEN** `living_spend` reports `8300`, `entertainment` still counts the `4700`, and `month_spend` stays `25000`

#### Scenario: Interest combines events and manual items

- **WHEN** a month has auto components totalling `16603.37` and an `interest` item `2698` labeled `HS promo rebate int`
- **THEN** the month reports `interest` `19301.37`

### Requirement: Derived 利息 breakdown

`GET /api/months/:ym` SHALL return the month's auto interest components as a list — each entry carrying `source` (`deposit`, `coupon` or `dividend`), `label`, `amount` (received amount, else expected/estimated, NULL when nothing is known) and `received`: the `interest` of every 定期 whose `end_date` falls in the month, and every bond coupon and HK dividend paid in the month (US dividends stay inside IBKR and are never counted). Unreceived entries carry `received` false and preview only — the month's reported `interest` SHALL equal Σ **received** auto components + Σ the month's `interest` items, so any 收訖 action — deposit, dividend or coupon — updates the figure with no further write.

#### Scenario: Breakdown lists every source

- **WHEN** month `2026-08-01` has a deposit ending with interest `2097` and one with `347.67`, plus received HK dividends `5939.05` and `8219.65`
- **THEN** the detail response lists four auto components totalling `16603.37` and the month reports `interest` `16603.37` plus its `interest` items

#### Scenario: Receipt updates the figure

- **WHEN** a received HK dividend dated in the month is stored, or its `received_amount` is later set via 收訖
- **THEN** the month's `interest` and breakdown include it on the next read — no month write is needed

#### Scenario: Unreceived events preview but do not count

- **WHEN** 定期 deposits with interest `993` and `285` have `end_date` in `2026-10` but are not yet received, and a 待定 bond coupon pays `2026-10-23`
- **THEN** October reports `interest` `0` (plus any `interest` items) while the breakdown lists all three as pending (`received` false) preview lines — the coupon with no amount until its rate is fixed

#### Scenario: 收訖 back-fills the maturity month

- **WHEN** the user later marks a deposit whose `end_date` fell in `2026-10` as received
- **THEN** October's `interest` and breakdown include its interest on the next read — even if the 收訖 click happens in a later month

### Requirement: Month items

Each month SHALL hold labeled line items in five categories: `adjustment` (bank in/out that must not count as 支出 — 定期 end credits, stock buys/sells, received dividends and coupons, transfers to the IBKR account), `extra_spend` (real spending excluded from 生活支出 — a TV, an AIA premium, tax, doctor), `income` (non-salary income added to 存 — e.g. a yearly bonus), `entertainment` (娛樂支出 — fun spending counted by the 開心Pool), and `interest` (利息 received that the auto events do not cover — bank 活期 interest, promo rebates). An `entertainment` item MAY carry `exclude_from_living`, which also subtracts its amount from `living_spend` — covering the sheet's habit of writing the same figure inside both the O sum and the J formula; the flag SHALL be rejected on other categories (`extra_spend` is excluded by definition). The API SHALL expose `POST /api/months/:ym/items`, `PATCH /api/month-items/:id`, and `DELETE /api/month-items/:id`; items SHALL be created implicitly when their month row is first patched.

#### Scenario: Adjustment keeps spending clean

- **WHEN** a month shows `start_cash` `30000`, an adjustment `−50000` (定期 start) and `+55000` (定期 end), and next month's figures give `month_spend` `25000`
- **THEN** `month_spend` stays `25000` — the deposit flows moved the balance but not the spending figure

#### Scenario: Extra spend lowers living spend only

- **WHEN** a month has `month_spend` `25000` and an extra_spend item `12000` labeled `AIA`
- **THEN** `living_spend` reports `13000` while `month_spend` stays `25000`

#### Scenario: One entertainment entry serves both columns

- **WHEN** a month has entertainment items `500`, `4700` (flagged `exclude_from_living`) and `75`
- **THEN** `entertainment` reports `5275` and the `4700` also subtracts from `living_spend` — no second entry needed

#### Scenario: Flag rejected off entertainment

- **WHEN** an item is created or patched with `category` `adjustment` and `exclude_from_living` `true`
- **THEN** the API rejects it with a field error on `exclude_from_living`

#### Scenario: Manual interest entry joins the derived figure

- **WHEN** a month has auto interest components totalling `16603.37` and the user adds an `interest` item `2698` labeled `HS promo rebate int`
- **THEN** the month reports `interest` `19301.37` and the item appears in the breakdown alongside the auto lines

### Requirement: Auto-suggested items

For the current month and later, `GET /api/months/:ym` SHALL also return candidate items derived from events dated in that month: each 定期 starting (an `adjustment` of − its principal, only while the deposit carries a `start_date`), each 定期 ending (an `adjustment` of + its principal + interest), each HK BUY/SELL trade (an `adjustment` of −/+ its total), each received HK dividend (an `adjustment` of + `received_amount`; US dividends stay inside IBKR and are never suggested), each received bond coupon (an `adjustment` of + `received_amount`), each AIA premium payment (an `extra_spend` of its HKD amount via the stored rate, absent with no rate), and the month's `pool_input` (an `adjustment` of − `pool_input`, suggested only while it is positive). Each candidate SHALL carry a stable `auto_key`; accepting a candidate stores it as an item carrying that key, and dismissing one SHALL record the key so it never reappears. Candidates SHALL NOT be suggested for months before the current month, and SHALL NOT be stored until accepted.

#### Scenario: Deposit starting suggests an adjustment

- **WHEN** a 定期 with principal `100000` and `start_date` in the current month exists
- **THEN** the month's response suggests an `adjustment` item of `−100000` for it

#### Scenario: Deposit ending suggests an adjustment

- **WHEN** a 定期 with principal `100000` and interest `1200` ends in the current month
- **THEN** the month's response suggests an `adjustment` item of `+101200` for it

#### Scenario: Accept then dismiss semantics

- **WHEN** the user accepts a suggested trade adjustment and dismisses a suggested coupon adjustment
- **THEN** the accepted item is stored and listed, and the dismissed suggestion is absent from later responses

#### Scenario: Manual-only items stay manual

- **WHEN** the user adds an IBKR transfer and a yearly bonus as items
- **THEN** they are stored as ordinary items with no `auto_key` and behave like any other item

### Requirement: Manual balances and salary

The system SHALL store named manual balances in two kinds — `cash` accounts (e.g. HS, 渣打 — the 活期 behind 月初) and `asset` rows (e.g. Irene, HS人壽) — each with `label`, `amount`, `sort_order`, `updated_at`, and a `liquidity` classification of `short` or `long` (default `long`; meaningful for `asset` rows, which the Overview 策略 block places in 短期可取回 or 長期可取回 accordingly — `cash` rows carry it but it has no effect on them), exposed through `GET/POST /api/manual-assets` and `PATCH/DELETE /api/manual-assets/:id`; `POST` SHALL accept `liquidity` optionally and `PATCH` SHALL accept it as a field to change, rejecting any value other than `short`/`long`. The system SHALL also store a current salary in `app_meta` (`overview.salary`) exposed through `PATCH /api/months/settings`. Month rows SHALL snapshot the salary at creation so past months keep their era's figure. The 資產 editor SHALL offer a 流動性 choice (短期 / 長期) for `asset` rows.

#### Scenario: Update cash balances

- **WHEN** the user saves HS `30538.78` and 渣打 `1364.89`
- **THEN** later live totals use 活期 `31903.67`

#### Scenario: Salary snapshot per row

- **WHEN** the salary setting is `52700` and a new month row is created, then the setting is raised to `55000`
- **THEN** the existing row still reports `52700` while the next created row reports `55000`

#### Scenario: Liquidity defaults to long

- **WHEN** the user creates an `asset` row without specifying `liquidity`
- **THEN** the stored row reports `liquidity` `long`

#### Scenario: Liquidity reclassified

- **WHEN** the user patches an `asset` row's `liquidity` to `short`
- **THEN** the row reports `short` and the next `GET /api/overview` counts its amount in `short_term` instead of `long_term`

#### Scenario: Invalid liquidity rejected

- **WHEN** a request carries `liquidity` `medium`
- **THEN** the request is rejected with a validation error and nothing is stored

### Requirement: Frozen and live asset totals

Each month SHALL carry `total_assets` (總數, the sheet's `Overview!B1`) and `liquid_assets` (流動資產, `Overview!H1`). A stored value SHALL win; when absent the figure SHALL be derived live as: `total_assets` = HK market value + (US market value + IBKR USD cash) × rate + IBKR HKD cash + active 定期 principal + active 債券 principal + AIA value × rate + MPF balance + Σ manual `asset` balances + Σ `cash` balances, and `liquid_assets` = HK market value + active 定期 principal + cash + active 債券 principal + (US market value + IBKR USD cash) × rate + IBKR HKD cash − the current 開心Pool balance. Active 定期 SHALL count at principal only (the sheet's `定期!B1 = sum(input)`), not principal + expected interest — deposit interest enters the totals via 活期 when the deposit ends. The IBKR cash positions (`ibkr.hkd_cash`/`ibkr.usd_cash` in `app_meta`) SHALL be included because the sheet's `Overview!B9` counts the whole `美股!B7` figure. Components needing a rate SHALL be absent when no `aia.usd_hkd_rate` is stored, and the totals SHALL omit them. Creating a month row SHALL snapshot the live values into the row (the sheet's paste-values step); patching `recapture` SHALL re-snapshot the totals AND `start_cash` (月初 = the live 活期 sum, Σ `cash` manual assets), and clearing the fields SHALL return the totals to live derivation.

#### Scenario: New row captures the live totals

- **WHEN** the user creates month `2026-10-01` at payday
- **THEN** its `total_assets`/`liquid_assets` store the live values at that moment and no longer move with prices

#### Scenario: Recapture also fills 月初

- **WHEN** cash manual assets total `120000` and a stored month has no `start_cash`
- **THEN** patching `recapture: true` stores `start_cash` `120000` alongside the re-snapshotted totals — and an explicit `start_cash` in the same patch wins over the snapshot

#### Scenario: Live row before capture

- **WHEN** a month row has no stored totals
- **THEN** its `total_assets`/`liquid_assets` reflect the current portfolio on every read

#### Scenario: Deposits count at principal

- **WHEN** active deposits hold principal `445000` with expected interest `2689.93` and no other components
- **THEN** live `total_assets` includes `445000` for the deposits, not `447689.93`

#### Scenario: IBKR cash included

- **WHEN** `ibkr.usd_cash` is `1200.25`, `ibkr.hkd_cash` is `765.42`, and the rate is `7.845135`
- **THEN** both live totals include approximately `10183.79` for the IBKR cash on top of the US stock value

### Requirement: 開心Pool balance

The system SHALL keep a per-year 娛樂 rate (`overview.pool_rate.<year>` in `app_meta`, editable through `PATCH /api/months/settings`, falling back to the latest earlier year's rate). Each year's pool income SHALL derive as Σ `interest` × that year's rate, and the pool balance at a month/year boundary SHALL derive as the previous year's closing balance + pool income − Σ `entertainment` items + Σ `pool_input`, matching the sheet's `M` column chain.

#### Scenario: Rate applies to the whole year

- **WHEN** the 2026 rate is set to `0.337` mid-year
- **THEN** all of 2026's pool income reprices to `0.337` without affecting other years

#### Scenario: Balance rolls forward

- **WHEN** 2025 closed at `5717.57` and 2026 shows interest `62033.15`, entertainment items totalling `8593`, pool inputs `4142.66` at rate `0.337`
- **THEN** the current pool balance is approximately `22172.4`

### Requirement: Yearly aggregates and running averages

`GET /api/months/summary` SHALL return, per year present, the sheet's yearly block — 總數+ (Σ `total_change`), 平均總數, 支出 (Σ `month_spend`), 平均支出, 生活平均支出, 娛樂支出 (Σ `entertainment` items), 利息回報 (Σ derived `interest`), 平均回報, 投資純利 (Σ derived `interest` + the year's stored HK `sold_pl`, absent while none is stored), 開心Pool結餘 and Irene + 開心 Pool (Σ `pool_input`) — plus the all-time running averages of `total_change`, `liquid_change`, `saved` and `interest` (the sheet's row-8 cells).

#### Scenario: Yearly rollup

- **WHEN** 2026 has nine stored months with spends totalling `239477.08` and derived interest totalling `62033.15`
- **THEN** the year's aggregate reports 支出 `239477.08` and 利息回報 `62033.15`

#### Scenario: 投資純利 adds stored sold P/L

- **WHEN** 2026 derives interest `62033.15` and the HK `sold_pl` snapshot stores `0`
- **THEN** the year's aggregate reports 投資純利 `62033.15`; with no stored `sold_pl` it is absent

### Requirement: 月結 page

The frontend SHALL show a 月結 view with a last-3-years aggregate table matching the sheet's yearly block, a month table (month, 總數, Changed, 流動資產, 流動資產 Changed, 月初, 調整, 月尾, 月支出, 生活支出, 生活按年 — the YoY percent colored green when negative and red above +2%, 存, 利息, 娛樂支出, Irene + 開心 Pool, note) whose derived cells show the computed figures, an item editor per month listing stored items and pending auto-suggestions with accept/dismiss actions plus manual add (娛樂 items offering a 不計入生活支出 flag, 利息 among the categories), and a settings card for cash/asset balances, salary and the current pool rate. The month editor SHALL show 娛樂支出 and 利息 as derived figures rather than editable fields — 利息 with a breakdown listing each auto component (deposit interest, received coupon, received dividend; unreceived lines rendered muted with a 未收 marker) and each `interest` item. The view SHALL reload after every mutation.

#### Scenario: Enter a payday balance

- **WHEN** the user types the new month's 月初 and saves
- **THEN** the table reloads, the new row shows captured totals, and the previous month's 月尾/月支出/存 appear

#### Scenario: Accept a suggestion

- **WHEN** the user accepts a suggested 定期-end adjustment on the current month
- **THEN** the view reloads with the item stored and 月支出 unchanged by the inflow

### Requirement: Money Master settings

`PATCH /api/months/settings` SHALL also accept the Money Master challenge fields — `start_date` (`YYYY-MM-DD`), `saved`, `target_months`, `target_amount`, and the optional `month_now`/`coming_save` overrides — stored as `money_master.*` keys in `app_meta`; `GET /api/months/settings` SHALL return them. `start_date` SHALL be a valid `YYYY-MM-DD` date or `null`; `month_now` and `target_months` SHALL be finite positive integers; `target_amount` SHALL be finite and positive; `saved` and `coming_save` SHALL be finite (negative allowed — the bank's figure goes negative while ahead of target). `null` clears a stored value; clearing `month_now` or `coming_save` returns that figure to its derivation. These figures are copied from the bank app — the challenge's own data — so they are user inputs, not derivations of tracked data.

#### Scenario: Settings updated

- **WHEN** `saved` is patched to `1094405.06` and `target_amount` to `1000000`
- **THEN** the settings response reports both and the overview block's `saved_progress`/`coming_save`/`can_use` follow

#### Scenario: Invalid field rejected

- **WHEN** `target_months` is patched to `0` or `start_date` to `not-a-date`
- **THEN** the request fails with 400 and field errors on the offending fields

#### Scenario: Override cleared restores derivation

- **WHEN** `month_now` is patched to `null` while `start_date` is `2023-10-27`
- **THEN** the overview block derives `month_now` from `start_date` again

#### Scenario: New challenge reconfigured

- **WHEN** the user patches `start_date`, `saved`, and the targets for a new challenge with no overrides stored
- **THEN** `month_now` restarts from `1`-indexed derivation and `coming_save` re-derives against the new targets
