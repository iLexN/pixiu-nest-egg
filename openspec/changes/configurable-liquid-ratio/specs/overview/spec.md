# Spec Delta

## ADDED Requirements

### Requirement: 半流動資金 target ratio setting

`PATCH /api/months/settings` SHALL also accept `semi_liquid_target` — the share of 流動資產 that 半流動資金 should cover — stored as `overview.semi_liquid_target` in `app_meta`. The value SHALL be a finite fraction in `[0, 1)`; `null` clears the stored value. `GET /api/months/settings` SHALL return the effective value (`0.25` — the sheet's `C14`/`N7` literal — while unset), and `GET /api/overview` SHALL carry the effective ratio so the 總覽 footer can label the configured percent.

#### Scenario: Updated ratio flows through

- **WHEN** `semi_liquid_target` is patched to `0.3`
- **THEN** the overview's `vs_quarter_liquid` and every forecast month's `ref_check` compare against `30%` of `liquid_assets`, and the settings response reports `0.3`

#### Scenario: Reset restores the sheet literal

- **WHEN** `semi_liquid_target` is patched to `null`
- **THEN** the effective ratio is `0.25` and both figures compare against `25%` of `liquid_assets` again

#### Scenario: Out-of-range ratio rejected

- **WHEN** `semi_liquid_target` is patched to `1.5` or a negative value
- **THEN** the request fails with 400 and a field error on `semi_liquid_target`

## MODIFIED Requirements

### Requirement: 半流動資金 block

`semi_liquid` SHALL carry `deposits` (已定期 = Σ principal over active deposits — principal only, matching `定期!B1`), `cash_rows` (every manual `cash` row, e.g. HS, 渣打), `cash_sum` (活期 = Σ cash rows), `total` (半流動資金 = `deposits` + `cash_sum`), `vs_quarter_liquid` (the sheet's `C14` = `total` − the configured `overview.semi_liquid_target` ratio × `liquid_assets`; the sheet's `25%` literal promoted to the setting, `0.25` while unset), and `share` (the sheet's `A13` = `total` ÷ (港股 + 債券 + `total` + IBKR), absent while any term is missing or the divisor is zero).

#### Scenario: Block derived

- **WHEN** active deposits hold principal 445000 and cash rows HS 30538.78 + 渣打 1364.89
- **THEN** `cash_sum` is 31903.67 and `total` is 476903.67

#### Scenario: Configured ratio applied

- **WHEN** `overview.semi_liquid_target` is `0.3`, `total` is `476903.67`, and `liquid_assets` is `1962919.66`
- **THEN** `vs_quarter_liquid` is approximately `-111972.23`

### Requirement: 總覽 page

The frontend SHALL show a 總覽 view with a headline strip (總數, 流動資產, J1 as a percentage, and the live 開心Pool balance), the asset table (label, HKD amount, share — `—` while an amount is absent), the 半流動資金 block (已定期, each cash row, 活期, and the 半流動資金 total — rendered red while below the configured `overview.semi_liquid_target` share of 流動資產 and green while above — with its `A13` share and `C14` difference on a second footer row whose label reflects the configured percent), and the IBKR block (stored inputs plus the derived figures). Manual `asset`/`cash` row amounts SHALL be editable inline via the existing manual-assets endpoints; the view SHALL reload after every mutation.

#### Scenario: Edit a manual row inline

- **WHEN** the user edits the HS cash row to `31000` on 總覽 and saves
- **THEN** the view reloads showing 活期 and 半流動資金 updated by the same amount

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

#### Scenario: Quarter month bill

- **WHEN** `forecast.bill_amount` is `2158` and the window covers `2027-01`
- **THEN** `2027-01` reports `bill` `-2158` and `2027-02` reports no `bill`
