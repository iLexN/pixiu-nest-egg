# Spec Delta

## MODIFIED Requirements

### Requirement: Monthly ledger rows

The system SHALL store one row per calendar month (`YYYY-MM-01`) carrying `start_cash` (月初出糧後 — the 活期 bank total measured right after salary lands, entered manually), `salary` (the salary in effect that month, snapshotting the current salary setting when the row is created), `interest` (利息 received that month), `pool_input` (Irene + 開心 Pool contributions), and `note`. 娛樂支出 is NOT stored on the row — it is the sum of the month's `entertainment` items. The API SHALL expose `GET /api/months` (optionally filtered by `year`), `GET /api/months/:ym`, `PATCH /api/months/:ym` (creating the row when absent), and `DELETE /api/months/:ym`.

#### Scenario: Create a month row at payday

- **WHEN** the user patches `2026-10-01` with `start_cash` `35000`
- **THEN** the row is stored with `salary` defaulting to the current salary setting

#### Scenario: Update figures

- **WHEN** the user patches a stored month with `interest` `8009.3`
- **THEN** the row reports that value and derived figures recompute on the next read

### Requirement: Derived monthly figures

Each month SHALL derive on read, never storing: `end_cash` (月尾出糧前) = the stored `end_cash_override` when present, else the next stored month's `start_cash` minus this month's own `salary`, absent while no later row exists; `month_spend` (月支出) = `start_cash` + Σ adjustment items − `end_cash`, absent without `end_cash`; `living_spend` (生活支出) = `month_spend` − Σ extra_spend items − Σ `exclude_from_living` entertainment items; `saved` (存) = `salary` − `month_spend` + Σ income items; `entertainment` (娛樂支出) = Σ entertainment items; `total_change` = the next month's `total_assets` − this month's; `liquid_change` likewise on `liquid_assets`; and `living_yoy` (the sheet's K column) = `(living_spend − living_spend of the same month one year earlier) / living_spend`, absent while either side is missing or `living_spend` is zero.

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

### Requirement: Month items

Each month SHALL hold labeled line items in four categories: `adjustment` (bank in/out that must not count as 支出 — 定期 end credits, stock buys/sells, received dividends and coupons, transfers to the IBKR account), `extra_spend` (real spending excluded from 生活支出 — a TV, an AIA premium, tax, doctor), `income` (non-salary income added to 存 — e.g. a yearly bonus), and `entertainment` (娛樂支出 — fun spending counted by the 開心Pool). An `entertainment` item MAY carry `exclude_from_living`, which also subtracts its amount from `living_spend` — covering the sheet's habit of writing the same figure inside both the O sum and the J formula; the flag SHALL be rejected on other categories (`extra_spend` is excluded by definition). The API SHALL expose `POST /api/months/:ym/items`, `PATCH /api/month-items/:id`, and `DELETE /api/month-items/:id`; items SHALL be created implicitly when their month row is first patched.

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

### Requirement: 開心Pool balance

The system SHALL keep a per-year 娛樂 rate (`overview.pool_rate.<year>` in `app_meta`, editable through `PATCH /api/months/settings`, falling back to the latest earlier year's rate). Each year's pool income SHALL derive as Σ `interest` × that year's rate, and the pool balance at a month/year boundary SHALL derive as the previous year's closing balance + pool income − Σ `entertainment` items + Σ `pool_input`, matching the sheet's `M` column chain.

#### Scenario: Rate applies to the whole year

- **WHEN** the 2026 rate is set to `0.337` mid-year
- **THEN** all of 2026's pool income reprices to `0.337` without affecting other years

#### Scenario: Balance rolls forward

- **WHEN** 2025 closed at `5717.57` and 2026 shows interest `62033.15`, entertainment items totalling `8593`, pool inputs `4142.66` at rate `0.337`
- **THEN** the current pool balance is approximately `22172.4`

### Requirement: Yearly aggregates and running averages

`GET /api/months/summary` SHALL return, per year present, the sheet's yearly block — 總數+ (Σ `total_change`), 平均總數, 支出 (Σ `month_spend`), 平均支出, 生活平均支出, 娛樂支出 (Σ `entertainment` items), 利息回報 (Σ `interest`), 平均回報, 投資純利, 開心Pool結餘 and Irene + 開心 Pool (Σ `pool_input`) — plus the all-time running averages of `total_change`, `liquid_change`, `saved` and `interest` (the sheet's row-8 cells).

#### Scenario: Yearly rollup

- **WHEN** 2026 has nine stored months with spends totalling `239477.08` and interest totalling `62033.15`
- **THEN** the year's aggregate reports 支出 `239477.08` and 利息回報 `62033.15`

### Requirement: 月結 page

The frontend SHALL show a 月結 view with a last-3-years aggregate table matching the sheet's yearly block, a month table (month, 總數, Changed, 流動資產, 流動資產 Changed, 月初, 調整, 月尾, 月支出, 生活支出, 生活按年 — the YoY percent colored green when negative and red above +2%, 存, 利息, 娛樂支出, Irene + 開心 Pool, note) whose derived cells show the computed figures, an item editor per month listing stored items and pending auto-suggestions with accept/dismiss actions plus manual add (娛樂 items offering a 不計入生活支出 flag), and a settings card for cash/asset balances, salary and the current pool rate. The month editor SHALL show 娛樂支出 as the derived item sum rather than an editable field. The view SHALL reload after every mutation.

#### Scenario: Enter a payday balance

- **WHEN** the user types the new month's 月初 and saves
- **THEN** the table reloads, the new row shows captured totals, and the previous month's 月尾/月支出/存 appear

#### Scenario: Accept a suggestion

- **WHEN** the user accepts a suggested 定期-end adjustment on the current month
- **THEN** the view reloads with the item stored and 月支出 unchanged by the inflow
