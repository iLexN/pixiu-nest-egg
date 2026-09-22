# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Monthly ledger rows

The system SHALL store one row per calendar month (`YYYY-MM-01`) carrying `start_cash` (月初出糧後 — the 活期 bank total measured right after salary lands, entered manually), `salary` (the salary in effect that month, snapshotting the current salary setting when the row is created), `pool_input` (Irene + 開心 Pool contributions), and `note`. 利息 is NOT stored on the row — it is derived from auto events plus the month's `interest` items. 娛樂支出 is NOT stored on the row — it is the sum of the month's `entertainment` items. The API SHALL expose `GET /api/months` (optionally filtered by `year`), `GET /api/months/:ym`, `PATCH /api/months/:ym` (creating the row when absent), and `DELETE /api/months/:ym`.

#### Scenario: Create a month row at payday

- **WHEN** the user patches `2026-10-01` with `start_cash` `35000`
- **THEN** the row is stored with `salary` defaulting to the current salary setting

#### Scenario: Update figures

- **WHEN** the user patches a stored month with `pool_input` `1500`
- **THEN** the row reports that value and derived figures recompute on the next read

### Requirement: Derived monthly figures

Each month SHALL derive on read, never storing: `end_cash` (月尾出糧前) = the stored `end_cash_override` when present, else the next stored month's `start_cash` minus this month's own `salary`, absent while no later row exists; `month_spend` (月支出) = `start_cash` + Σ adjustment items − `end_cash`, absent without `end_cash`; `living_spend` (生活支出) = `month_spend` − Σ extra_spend items − Σ `exclude_from_living` entertainment items; `saved` (存) = `salary` − `month_spend` + Σ income items; `interest` (利息) = Σ auto interest components + Σ `interest` items; `entertainment` (娛樂支出) = Σ entertainment items; `total_change` = the next month's `total_assets` − this month's; `liquid_change` likewise on `liquid_assets`; and `living_yoy` (the sheet's K column) = `(living_spend − living_spend of the same month one year earlier) / living_spend`, absent while either side is missing or `living_spend` is zero.

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

### Requirement: Yearly aggregates and running averages

`GET /api/months/summary` SHALL return, per year present, the sheet's yearly block — 總數+ (Σ `total_change`), 平均總數, 支出 (Σ `month_spend`), 平均支出, 生活平均支出, 娛樂支出 (Σ `entertainment` items), 利息回報 (Σ derived `interest`), 平均回報, 投資純利, 開心Pool結餘 and Irene + 開心 Pool (Σ `pool_input`) — plus the all-time running averages of `total_change`, `liquid_change`, `saved` and `interest` (the sheet's row-8 cells).

#### Scenario: Yearly rollup

- **WHEN** 2026 has nine stored months with spends totalling `239477.08` and derived interest totalling `62033.15`
- **THEN** the year's aggregate reports 支出 `239477.08` and 利息回報 `62033.15`

### Requirement: 月結 page

The frontend SHALL show a 月結 view with a last-3-years aggregate table matching the sheet's yearly block, a month table (month, 總數, Changed, 流動資產, 流動資產 Changed, 月初, 調整, 月尾, 月支出, 生活支出, 生活按年 — the YoY percent colored green when negative and red above +2%, 存, 利息, 娛樂支出, Irene + 開心 Pool, note) whose derived cells show the computed figures, an item editor per month listing stored items and pending auto-suggestions with accept/dismiss actions plus manual add (娛樂 items offering a 不計入生活支出 flag, 利息 among the categories), and a settings card for cash/asset balances, salary and the current pool rate. The month editor SHALL show 娛樂支出 and 利息 as derived figures rather than editable fields — 利息 with a breakdown listing each auto component (deposit interest, received coupon, received dividend; unreceived deposit lines rendered muted with a 未收 marker) and each `interest` item. The view SHALL reload after every mutation.

#### Scenario: Enter a payday balance

- **WHEN** the user types the new month's 月初 and saves
- **THEN** the table reloads, the new row shows captured totals, and the previous month's 月尾/月支出/存 appear

#### Scenario: Accept a suggestion

- **WHEN** the user accepts a suggested 定期-end adjustment on the current month
- **THEN** the view reloads with the item stored and 月支出 unchanged by the inflow

#### Scenario: Breakdown explains the figure

- **WHEN** the user opens `2026-08` whose interest is `19301.37`
- **THEN** the 利息 breakdown shows the two deposit-interest lines, the two received dividends and the `HS promo rebate int` item summing to that total
