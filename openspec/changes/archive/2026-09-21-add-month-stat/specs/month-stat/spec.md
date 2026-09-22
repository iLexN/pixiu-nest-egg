# Spec Delta

## Purpose

Tracks the user's month-by-month cash bookkeeping — the payday bank balance, non-spending bank flows, and interest/entertainment/pool figures — so spending, saving and the 開心Pool balance fall out of balance differences without transaction logging, replacing the workbook's `Month Stat` sheet.

## ADDED Requirements

### Requirement: Monthly ledger rows

The system SHALL store one row per calendar month (`YYYY-MM-01`) carrying `start_cash` (月初出糧後 — the 活期 bank total measured right after salary lands, entered manually), `salary` (the salary in effect that month, snapshotting the current salary setting when the row is created), `interest` (利息 received that month), `entertainment` (娛樂支出), `pool_input` (Irene + 開心 Pool contributions), and `note`. The API SHALL expose `GET /api/months` (optionally filtered by `year`), `GET /api/months/:ym`, `PATCH /api/months/:ym` (creating the row when absent), and `DELETE /api/months/:ym`.

#### Scenario: Create a month row at payday

- **WHEN** the user patches `2026-10-01` with `start_cash` `35000`
- **THEN** the row is stored with `salary` defaulting to the current salary setting

#### Scenario: Update figures

- **WHEN** the user patches a stored month with `interest` `8009.3` and `entertainment` `263`
- **THEN** the row reports those values and derived figures recompute on the next read

### Requirement: Derived monthly figures

Each month SHALL derive on read, never storing: `end_cash` (月尾出糧前) = the stored `end_cash_override` when present, else the next stored month's `start_cash` minus this month's own `salary`, absent while no later row exists; `month_spend` (月支出) = `start_cash` + Σ adjustment items − `end_cash`, absent without `end_cash`; `living_spend` (生活支出) = `month_spend` − Σ extra_spend items; `saved` (存) = `salary` − `month_spend` + Σ income items; `total_change` = the next month's `total_assets` − this month's; and `liquid_change` likewise on `liquid_assets`.

#### Scenario: Spending falls out of the balance difference

- **WHEN** month `2026-09-01` has `start_cash` `30000`, `salary` `52700`, adjustments `+8009.3`, and the next month has `start_cash` `32000`
- **THEN** September reports `end_cash` `−20700`, `month_spend` `58709.3`, `living_spend` `58709.3` minus its extra_spend items, and `saved` `−6009.3` plus its income items

#### Scenario: Latest month has no month-end yet

- **WHEN** a month is the latest stored row
- **THEN** `end_cash`, `month_spend`, `living_spend` and `saved` are absent until the next month's `start_cash` exists

#### Scenario: Hand-frozen month-end wins over the chain

- **WHEN** month `2023-12-01` stores `end_cash_override` `24610.32` (the real bank balance typed before the 月初 convention existed) with `start_cash` `64925.18`, `salary` `45500`, and the next month has `start_cash` `26391.71`
- **THEN** December reports `end_cash` `24610.32` — not the chain's `−19108.29` — so `month_spend` and `saved` match the hand-kept history; clearing the override with `null` restores derivation

### Requirement: Month items

Each month SHALL hold labeled line items in three categories: `adjustment` (bank in/out that must not count as 支出 — 定期 end credits, stock buys/sells, received dividends and coupons, transfers to the IBKR account), `extra_spend` (real spending excluded from 生活支出 — a TV, an AIA premium, tax, doctor), and `income` (non-salary income added to 存 — e.g. a yearly bonus). The API SHALL expose `POST /api/months/:ym/items`, `PATCH /api/month-items/:id`, and `DELETE /api/month-items/:id`; items SHALL be created implicitly when their month row is first patched.

#### Scenario: Adjustment keeps spending clean

- **WHEN** a month shows `start_cash` `30000`, an adjustment `−50000` (定期 start) and `+55000` (定期 end), and next month's figures give `month_spend` `25000`
- **THEN** `month_spend` stays `25000` — the deposit flows moved the balance but not the spending figure

#### Scenario: Extra spend lowers living spend only

- **WHEN** a month has `month_spend` `25000` and an extra_spend item `12000` labeled `AIA`
- **THEN** `living_spend` reports `13000` while `month_spend` stays `25000`

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

The system SHALL store named manual balances in two kinds — `cash` accounts (e.g. HS, 渣打 — the 活期 behind 月初) and `asset` rows (e.g. Irene, HS人壽) — each with `label`, `amount`, `sort_order` and `updated_at`, exposed through `GET/POST /api/manual-assets` and `PATCH/DELETE /api/manual-assets/:id`; and a current salary in `app_meta` (`overview.salary`) exposed through `PATCH /api/months/settings`. Month rows SHALL snapshot the salary at creation so past months keep their era's figure.

#### Scenario: Update cash balances

- **WHEN** the user saves HS `30538.78` and 渣打 `1364.89`
- **THEN** later live totals use 活期 `31903.67`

#### Scenario: Salary snapshot per row

- **WHEN** the salary setting is `52700` and a new month row is created, then the setting is raised to `55000`
- **THEN** the existing row still reports `52700` while the next created row reports `55000`

### Requirement: Frozen and live asset totals

Each month SHALL carry `total_assets` (總數, the sheet's `Overview!B1`) and `liquid_assets` (流動資產, `Overview!H1`). A stored value SHALL win; when absent the figure SHALL be derived live as: `total_assets` = HK market value + US market value × rate + active 定期 total + active 債券 principal + AIA value × rate + MPF balance + Σ manual `asset` balances + Σ `cash` balances, and `liquid_assets` = HK market value + active 定期 total + cash + active 債券 principal + US market value × rate − the current 開心Pool balance. Components needing a rate SHALL be absent when no `aia.usd_hkd_rate` is stored, and the totals SHALL omit them. Creating a month row SHALL snapshot the live values into the row (the sheet's paste-values step); patching `recapture` SHALL re-snapshot, and clearing the fields SHALL return the row to live derivation.

#### Scenario: New row captures the live totals

- **WHEN** the user creates month `2026-10-01` at payday
- **THEN** its `total_assets`/`liquid_assets` store the live values at that moment and no longer move with prices

#### Scenario: Live row before capture

- **WHEN** a month row has no stored totals
- **THEN** its `total_assets`/`liquid_assets` reflect the current portfolio on every read

### Requirement: 開心Pool balance

The system SHALL keep a per-year 娛樂 rate (`overview.pool_rate.<year>` in `app_meta`, editable through `PATCH /api/months/settings`, falling back to the latest earlier year's rate). Each year's pool income SHALL derive as Σ `interest` × that year's rate, and the pool balance at a month/year boundary SHALL derive as the previous year's closing balance + pool income − Σ `entertainment` + Σ `pool_input`, matching the sheet's `M` column chain.

#### Scenario: Rate applies to the whole year

- **WHEN** the 2026 rate is set to `0.337` mid-year
- **THEN** all of 2026's pool income reprices to `0.337` without affecting other years

#### Scenario: Balance rolls forward

- **WHEN** 2025 closed at `5717.57` and 2026 shows interest `62033.15`, entertainment `8593`, pool inputs `4142.66` at rate `0.337`
- **THEN** the current pool balance is approximately `22172.4`

### Requirement: Yearly aggregates and running averages

`GET /api/months/summary` SHALL return, per year present, the sheet's yearly block — 總數+ (Σ `total_change`), 平均總數, 支出 (Σ `month_spend`), 平均支出, 生活平均支出, 娛樂支出 (Σ `entertainment`), 利息回報 (Σ `interest`), 平均回報, 投資純利, 開心Pool結餘 and Irene + 開心 Pool (Σ `pool_input`) — plus the all-time running averages of `total_change`, `saved` and `interest` (the sheet's row-8 cells).

#### Scenario: Yearly rollup

- **WHEN** 2026 has nine stored months with spends totalling `239477.08` and interest totalling `62033.15`
- **THEN** the year's aggregate reports 支出 `239477.08` and 利息回報 `62033.15`

### Requirement: 月結 page

The frontend SHALL show a 月結 view with a per-year aggregate strip, a month table (month, 總數, Changed, 流動資產, 流動資產 Changed, 月初, 調整, 月尾, 月支出, 生活支出, 存, 利息, 娛樂支出, Irene + 開心 Pool, note) whose derived cells show the computed figures, an item editor per month listing stored items and pending auto-suggestions with accept/dismiss actions plus manual add, a settings card for cash/asset balances, salary and the current pool rate, and the current pool balance. The view SHALL reload after every mutation.

#### Scenario: Enter a payday balance

- **WHEN** the user types the new month's 月初 and saves
- **THEN** the table reloads, the new row shows captured totals, and the previous month's 月尾/月支出/存 appear

#### Scenario: Accept a suggestion

- **WHEN** the user accepts a suggested 定期-end adjustment on the current month
- **THEN** the view reloads with the item stored and 月支出 unchanged by the inflow
