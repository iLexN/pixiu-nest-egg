# Spec Delta

## MODIFIED Requirements

### Requirement: Forecast projection block

`GET /api/forecast` SHALL return a `months` array mirroring the sheet's `預測` grid `A20:H36`: seven entries for the current calendar month through current +6, each carrying `month` (`YYYY-MM-01`) and the sheet's rows as derived fields — `start`, `salary`, `spend`, `deposit_finish`, `interest`, `bill`, `plan_items`, `deposit_placements`, `deposit_return`, `cash`, `locked`, `semi_liquid`, and `ref_check`.

Per month `m`:

- `start` SHALL be the current month's stored `start_cash` (月初出糧後) when `m` is the first month — falling back to the live 活期 sum (Σ `cash` manual assets) while no `start_cash` is stored — and the previous month's `cash` otherwise; absent while its source is absent.
- `salary` SHALL be the stored `overview.salary`, absent while unset.
- `spend` SHALL be −`living_budget` from the overview averages block, absent while no budget exists.
- `deposit_finish` SHALL be Σ `principal` of deposits whose `end_date` falls in `m` (the sheet's `定期 finish` row sourced from `2026回報率`/`定期Info` SUMIFs), `0` when none.
- `interest` SHALL be Σ `interest` of deposits ending in `m` + Σ HK dividend amounts with a pay/record date in `m` + Σ bond coupon amounts due or received in `m` + Σ `interest`-kind forecast items in `m` — counting known expected or received amounts and skipping components with no known amount, `0` when none. The month entry SHALL also carry `interest_components` listing the auto receipts (source `deposit`/`coupon`/`dividend`, label, amount, `received`) so the breakdown behind the figure is inspectable.
- `bill` SHALL be − the stored `forecast.bill_amount` setting on Jan/Apr/Jul/Oct months (the sheet's 差餉 quarters reading `B39`), absent on other months; when one or more `bill` forecast items exist for `m`, `bill` SHALL be their Σ instead (an override, not an addition).
- `plan_items` SHALL list the month's stored forecast items.
- `deposit_placements` SHALL list the real deposits whose `start_date` falls in `m` — each with `deposit_id`, `label`, `bank`, `principal`, and `start_date` — empty when none. Deposits without a `start_date` are never placements.
- `deposit_return` SHALL be the sum described by the deposit-returns requirement, `0` when none.
- `cash` (活期) SHALL be `start + salary + spend + deposit_finish + interest + bill + deposit_return + Σ amounts of `tax`/`stock`/`other` items in `m` + Σ amounts of `hs_deposit`/`sc_deposit` items in `m` − Σ `principal` of `deposit_placements` in `m`, absent while `start` is absent; absent components count as `0`. (`interest` and `bill` items are already inside `interest`/`bill`, so they are not summed again.)

`ref_check` SHALL be `semi_liquid − the configured `overview.semi_liquid_target` ratio × liquid_assets` using the live `liquid_assets` (the sheet's `B20:H20` against `$N$7` — `H1 ÷ 4`, the same threshold as `C14` written as a division), absent while either side is absent.

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

#### Scenario: Real deposit placed in the month

- **WHEN** deposit `9077` with `principal` `55000`, bank `SC`, `start_date` `2026-09-30` exists and the window starts at `2026-09`
- **THEN** `2026-09` lists it in `deposit_placements` and its `cash` is `55000` lower than it would be without the deposit

#### Scenario: Quarter month bill

- **WHEN** `forecast.bill_amount` is `2158` and the window covers `2027-01`
- **THEN** `2027-01` reports `bill` `-2158` and `2027-02` reports no `bill`

### Requirement: Locked 定期+SC chain

Each forecast month SHALL derive `locked` (the sheet's `定期 + SC` row) from a month-start anchor: Σ `principal` of deposits held on the window's first day — `start_date` absent or before the first month's 1st, and `end_date` on or after that 1st. The anchor SHALL NOT use the live "end date not passed" total, so a deposit that matured earlier in the current month stays in the anchor and leaves through `deposit_finish`, and a deposit starting on or after the 1st enters through its placement month. Each month `m` (the first included, chaining from the anchor) SHALL derive `locked(m) = locked(m−1) − deposit_finish(m) − Σ hs_deposit/sc_deposit plan amounts(m) + Σ principal of deposit_placements(m) − deposit_return(m)` — plan amounts are stored negative, so subtracting them adds to `locked`. `semi_liquid` SHALL be `cash + locked`, absent while either side is absent. The overview `半流動資金` block SHALL keep using the live active-deposit total.

#### Scenario: Locked evolves

- **WHEN** the anchor is `445000`, `2026-09` has no flows, and `2026-10` finishes `145000`, places an `sc_deposit` plan `-180000`, and returns `55000`
- **THEN** `2026-10.locked` is `445000 − 145000 + 180000 − 55000 = 425000`

#### Scenario: Anchor excludes deposits starting in the window

- **WHEN** the window starts `2026-09` and deposits are: `140000` started `2026-07-01` ending `2026-11-03`, `80000` with no `start_date` ending `2026-11-20`, `80000` started `2026-08-31` ending `2027-01-05`, and `55000` started `2026-09-30` ending `2027-01-05`
- **THEN** the anchor is `300000` and `2026-09.locked` is `355000`

#### Scenario: Matured earlier this month

- **WHEN** today is `2026-10-13`, the window starts `2026-10`, and deposits of `110000` and `35000` (started before October) ended `2026-10-12`
- **THEN** the anchor includes both, `2026-10.deposit_finish` is `145000`, and `2026-10.locked` is the anchor minus `145000` — they are not subtracted twice

#### Scenario: Margin check

- **WHEN** `2026-09` derives `semi_liquid` `520071.25` and live `liquid_assets` is `1964860.57`
- **THEN** `ref_check` is approximately `28856.10`

### Requirement: Convert plan to deposit

`POST /api/forecast-items/:id/convert` SHALL accept the deposit fields (`principal` prefilled with `−amount` by the frontend, `end_date` required, `rate`/`interest`/`start_date`/`label`/`note` optional, `bank` defaulting from the item kind), create the deposit row, and delete the forecast item in one transaction. The item's scheduled `deposit_return` disappears with it since returns are derived. When the deposit's `start_date` falls in the plan's month, and its `end_date` falls in the plan's return month, the forecast's `cash`, `locked`, `semi_liquid`, and `ref_check` SHALL be unchanged by the conversion except for the deposit's own `interest` now counting in its end month.

#### Scenario: Conversion replaces the plan

- **WHEN** `sc_deposit` item `-180000` in `2026-10` is converted with `start_date` `2026-10-12` and `end_date` `2027-02-10`
- **THEN** a deposit row exists with `principal` `180000` and `bank` `SC`, the forecast item is gone, `2026-10` lists the deposit in `deposit_placements` in place of the plan, and `2027-02` gains the deposit's real `deposit_finish`/`interest` instead of the plan's derived return

#### Scenario: Conversion keeps the grid totals

- **WHEN** the current month `2026-09` holds an `sc_deposit` plan `-55000` and it is converted with `start_date` `2026-09-30`, `end_date` `2027-01-05`, and `interest` `435`
- **THEN** every month's `cash` and `semi_liquid` are the same as before the conversion, except that from `2027-01` onward they are `435` higher, and every month's `locked` is unchanged

### Requirement: 總覽 預測 grid card

The 總覽 page SHALL show a 預測 card rendering the grid with months as columns and the sheet's lines as rows — `ref check`, `半流動`, month-end dates, `活期`, `定期+SC`, `start`, `salary`, `支出`, `定期 finish`, `定期 HS`, `SC高息馬拉松`, `利息`, `Tax/基金/醫療保險`, `股票`, `繳費`, `TBC - 定期 end`, `TBC` — `—` while a figure is absent. Plan cells SHALL offer inline add/edit/delete per item and a `轉為定期` action on `hs_deposit`/`sc_deposit` items that opens the deposit form prefilled. Each of the month's `deposit_placements` SHALL appear as a read-only line (label, `−principal`, `start_date`, marked `已定期`) in the `定期 HS` row when its bank is `HS` and in the `SC高息馬拉松` row otherwise; the cell's figure SHALL be Σ its plan amounts plus Σ `−principal` of its placements, and placements SHALL offer no edit, delete, or convert action. `ref_check` SHALL render in the negative/red style while below `0`. The view SHALL reload after every mutation.

#### Scenario: Grid renders

- **WHEN** the forecast returns seven months
- **THEN** the card shows the sheet's row labels down the side and seven month columns

#### Scenario: Convert from the grid

- **WHEN** the user clicks `轉為定期` on an `sc_deposit` item of `-180000`
- **THEN** the deposit form opens with `principal` `180000` and bank `SC`, and saving it calls the convert endpoint

#### Scenario: Placement shown in its bank row

- **WHEN** `2026-09` has no plans and one placement `9077` of `55000` with bank `SC`
- **THEN** the `SC高息馬拉松` cell for `2026-09` shows `-55000`, its editor lists `9077` as a read-only `已定期` line, and the `定期 HS` cell stays `—`
