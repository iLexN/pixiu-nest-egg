# Spec Delta

## ADDED Requirements

### Requirement: Forecast projection block

`GET /api/forecast` SHALL return a `months` array mirroring the sheet's `預測` grid `A20:H36`: seven entries for the current calendar month through current +6, each carrying `month` (`YYYY-MM-01`) and the sheet's rows as derived fields — `start`, `salary`, `spend`, `deposit_finish`, `interest`, `bill`, `plan_items`, `deposit_return`, `cash`, `locked`, `semi_liquid`, and `ref_check`.

Per month `m`:

- `start` SHALL be the current month's stored `start_cash` (月初出糧後) when `m` is the first month — falling back to the live 活期 sum (Σ `cash` manual assets) while no `start_cash` is stored — and the previous month's `cash` otherwise; absent while its source is absent.
- `salary` SHALL be the stored `overview.salary`, absent while unset.
- `spend` SHALL be −`living_budget` from the overview averages block, absent while no budget exists.
- `deposit_finish` SHALL be Σ `principal` of deposits whose `end_date` falls in `m` (the sheet's `定期 finish` row sourced from `2026回報率`/`定期Info` SUMIFs), `0` when none.
- `interest` SHALL be Σ `interest` of deposits ending in `m` + Σ HK dividend amounts with a pay/record date in `m` + Σ bond coupon amounts due or received in `m` + Σ `interest`-kind forecast items in `m` — counting known expected or received amounts and skipping components with no known amount, `0` when none. The month entry SHALL also carry `interest_components` listing the auto receipts (source `deposit`/`coupon`/`dividend`, label, amount, `received`) so the breakdown behind the figure is inspectable.
- `bill` SHALL be − the stored `forecast.bill_amount` setting on Jan/Apr/Jul/Oct months (the sheet's 差餉 quarters reading `B39`), absent on other months; when one or more `bill` forecast items exist for `m`, `bill` SHALL be their Σ instead (an override, not an addition).
- `plan_items` SHALL list the month's stored forecast items.
- `deposit_return` SHALL be the sum described by the deposit-returns requirement, `0` when none.
- `cash` (活期) SHALL be `start + salary + spend + deposit_finish + interest + bill + deposit_return + Σ amounts of `tax`/`stock`/`other` items in `m` + Σ amounts of `hs_deposit`/`sc_deposit` items in `m`, absent while `start` is absent; absent components count as `0`. (`interest` and `bill` items are already inside `interest`/`bill`, so they are not summed again.)

`ref_check` SHALL be `semi_liquid − 0.25 × liquid_assets` using the live `liquid_assets` (the sheet's `B20:H20` against `$N$7`), absent while either side is absent.

#### Scenario: First column anchors on 月初

- **WHEN** the current month `2026-09` has `start_cash` `23746.72`, salary `52700`, `living_budget` `14900`, and no deposits, plans, bills or receipts in the month
- **THEN** month `2026-09` reports `start` `23746.72`, `salary` `52700`, `spend` `-14900`, and `cash` approximately `61546.72`

#### Scenario: Start falls back to live cash

- **WHEN** the current month has no stored `start_cash` and the `cash` manual rows total `32934.62`
- **THEN** the first month's `start` is `32934.62`

#### Scenario: Chained start

- **WHEN** month `2026-09` derives `cash` `61546.72`
- **THEN** month `2026-10` reports `start` `61546.72` regardless of `2026-10`'s own `start_cash`

#### Scenario: Deposit finish and interest

- **WHEN** deposits SC-9632 `110000` and SC-7006 `35000` end `2026-10-12` with interest `993` and `285`, and a `145`-HKD dividend pays in `2026-10`
- **THEN** `2026-10` reports `deposit_finish` `145000` and `interest` includes `993 + 285 + 1423` equivalents — `interest` totals `993 + 285 + 145` plus any coupons due that month

#### Scenario: Quarter month bill

- **WHEN** `forecast.bill_amount` is `2158` and the window covers `2027-01`
- **THEN** `2027-01` reports `bill` `-2158` and `2027-02` reports no `bill`

### Requirement: Locked 定期+SC chain

Each forecast month SHALL derive `locked` (the sheet's `定期 + SC` row): the first month's `locked` SHALL be Σ `principal` of deposits whose `end_date` has not passed (matching `semi_liquid.deposits`), and each later month SHALL be `locked(m−1) − deposit_finish(m) − Σ hs_deposit/sc_deposit plan amounts(m) − deposit_return(m)` — plan amounts are stored negative, so subtracting them adds to `locked`. `semi_liquid` SHALL be `cash + locked`, absent while either side is absent.

#### Scenario: Locked evolves

- **WHEN** the first month locks `445000`, `2026-10` finishes `145000`, places an `sc_deposit` plan `-180000`, and returns `55000`
- **THEN** `2026-10.locked` is `445000 − 145000 + 180000 − 55000 = 425000`

#### Scenario: Margin check

- **WHEN** `2026-09` derives `semi_liquid` `520071.25` and live `liquid_assets` is `1964860.57`
- **THEN** `ref_check` is approximately `28856.10`

### Requirement: Forecast plan items

The system SHALL store forecast items — `id`, `month` (`YYYY-MM-01`), `kind`, signed `amount`, `note`, and optional `return_month` — exposed through `POST /api/forecast/:ym/items`, `PATCH /api/forecast-items/:id`, and `DELETE /api/forecast-items/:id`. `kind` SHALL be one of `hs_deposit`, `sc_deposit` (planned lockups, stored negative — they reduce `cash`, add to `locked`, and schedule a `deposit_return`), `interest` (manual interest additions, the sheet's `+1000`-style cell), `tax` (Tax/基金/醫療保險), `stock` (股票), `bill` (a `bill` override), or `other` (TBC). Multiple items SHALL be allowed per month and kind — the sheet's `=7000+6000` cell is two items. `return_month` SHALL be rejected on kinds other than `hs_deposit`/`sc_deposit`, and `amount` SHALL be finite and non-zero. Items dated outside the current window SHALL still be stored: items ahead of the window enter it as months slide forward; items whose month is before the window are stale — their placement and return effects apply only while the item's own month is inside the window (the sheet's shifted-out cells fire nothing either, and this also prevents a real deposit created without conversion from double-counting a stale plan's return).

#### Scenario: Two placements in one cell's place

- **WHEN** items `{month: 2026-10, kind: hs_deposit, amount: -70000}` and `{month: 2026-10, kind: hs_deposit, amount: -60000}` exist
- **THEN** `2026-10` lists both in `plan_items`, `cash` drops by `130000`, and `locked` rises by `130000`

#### Scenario: return_month rejected on wrong kind

- **WHEN** an item is created with `kind` `tax` and `return_month` `2027-03`
- **THEN** the request fails with 400

### Requirement: Deposit plan returns

Each in-window `hs_deposit` item SHALL schedule a `deposit_return` of `−amount` in `month + 3`, and each `sc_deposit` item in `month + 4` (the sheet's `TBC - 定期 end` formulas `=-(B30+C29)`). A stored `return_month` SHALL override the default. Returns SHALL count toward `cash` and reduce `locked` in the return month when that month is inside the window; return months landing beyond the window's last column are invisible.

#### Scenario: Default lags

- **WHEN** `sc_deposit` `-55000` exists in `2026-09` and `hs_deposit` `-20000` exists in `2026-10`
- **THEN** `2027-01` reports `deposit_return` `75000`

#### Scenario: Override wins

- **WHEN** `hs_deposit` `-80000` in `2026-11` carries `return_month` `2027-05`
- **THEN** `2027-02` reports no return from it and `2027-05` (when in window) reports `80000`

### Requirement: Convert plan to deposit

`POST /api/forecast-items/:id/convert` SHALL accept the deposit fields (`principal` prefilled with `−amount` by the frontend, `end_date` required, `rate`/`interest`/`start_date`/`label`/`note` optional, `bank` defaulting from the item kind), create the deposit row, and delete the forecast item in one transaction. The item's scheduled `deposit_return` disappears with it since returns are derived.

#### Scenario: Conversion replaces the plan

- **WHEN** `sc_deposit` item `-180000` in `2026-10` is converted with `end_date` `2027-02-10`
- **THEN** a deposit row exists with `principal` `180000` and `bank` `SC`, the forecast item is gone, `2026-10`'s `cash`/`locked` no longer include the plan, and `2027-02` gains the deposit's real `deposit_finish`/`interest` instead of the plan's derived return

### Requirement: Bill amount setting

`PATCH /api/months/settings` SHALL also accept `bill_amount` (finite, non-negative), stored as `forecast.bill_amount` in `app_meta`. The seeded default SHALL be `2158` (the sheet's `B39`).

#### Scenario: Updated amount flows through

- **WHEN** `bill_amount` is patched to `2260` and no `bill` items exist
- **THEN** the next quarter month in the window reports `bill` `-2260`

### Requirement: 總覽 預測 grid card

The 總覽 page SHALL show a 預測 card rendering the grid with months as columns and the sheet's lines as rows — `ref check`, `半流動`, month-end dates, `活期`, `定期+SC`, `start`, `salary`, `支出`, `定期 finish`, `定期 HS`, `SC高息馬拉松`, `利息`, `Tax/基金/醫療保險`, `股票`, `繳費`, `TBC - 定期 end`, `TBC` — `—` while a figure is absent. Plan cells SHALL offer inline add/edit/delete per item and a `轉為定期` action on `hs_deposit`/`sc_deposit` items that opens the deposit form prefilled. `ref_check` SHALL render in the negative/red style while below `0`. The view SHALL reload after every mutation.

#### Scenario: Grid renders

- **WHEN** the forecast returns seven months
- **THEN** the card shows the sheet's row labels down the side and seven month columns

#### Scenario: Convert from the grid

- **WHEN** the user clicks `轉為定期` on an `sc_deposit` item of `-180000`
- **THEN** the deposit form opens with `principal` `180000` and bank `SC`, and saving it calls the convert endpoint
