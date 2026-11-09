# Spec Delta

## ADDED Requirements

### Requirement: Forecast grid cells

Each forecast month SHALL carry `cells`, one per plan row in sheet order — `hs_deposit`, `sc_deposit`, `interest`, `tax`, `stock`, `bill`, `other` — each with `kind`, `figure`, `plan_items` (the month's items of that kind), `placements`, `trade_lines`, and `ibkr_lines`. Lists a row never carries SHALL be empty. The response SHALL NOT carry month-level `plan_items`, `deposit_placements`, `trade_lines`, or `ibkr_lines`.

#### Scenario: Seven cells per month

- **WHEN** the forecast returns a month with no plans, placements, trades or transfers
- **THEN** that month's `cells` lists seven entries in the order `hs_deposit`, `sc_deposit`, `interest`, `tax`, `stock`, `bill`, `other`, each with empty lists

#### Scenario: Items land in their kind's cell

- **WHEN** `2026-10` holds a `tax` item of `-3000` and an `hs_deposit` item of `-70000`
- **THEN** the `tax` cell's `plan_items` holds only the tax item and the `hs_deposit` cell's `plan_items` holds only the deposit plan

### Requirement: Forecast cell placement routing

A month's real deposit placements SHALL land in the `hs_deposit` cell's `placements` when the deposit's bank is `HS`, and in the `sc_deposit` cell's `placements` for any other bank, including an unset bank. Other cells SHALL carry no placements.

#### Scenario: HS placement

- **WHEN** a deposit with bank `HS` and `principal` `130000` has `start_date` in `2026-11`
- **THEN** the `2026-11` `hs_deposit` cell lists it in `placements` and the `sc_deposit` cell does not

#### Scenario: Unset bank goes to the SC row

- **WHEN** a deposit with no bank has `start_date` in a window month
- **THEN** it appears in that month's `sc_deposit` cell `placements`

### Requirement: Forecast cell figure

Each cell's `figure` SHALL be the value the grid shows for that row. For the `bill` cell it SHALL be the month's `bill`, and for the `interest` cell the month's `interest`. For every other cell it SHALL be Σ plan item amounts + Σ `−principal` of placements + Σ trade line amounts + Σ IBKR line amounts, absent while the cell has none of these.

#### Scenario: Empty cell has no figure

- **WHEN** a month's `tax` cell has no plan items
- **THEN** its `figure` is absent

#### Scenario: Bill cell mirrors the bill row

- **WHEN** a quarter month reports `bill` `-2158` with no `bill` items
- **THEN** its `bill` cell's `figure` is `-2158` and its `plan_items` is empty

#### Scenario: Interest cell mirrors the interest row

- **WHEN** a month reports `interest` `0` with no interest items or receipts
- **THEN** its `interest` cell's `figure` is `0`

## MODIFIED Requirements

### Requirement: Forecast projection block

`GET /api/forecast` SHALL return a `months` array mirroring the sheet's `預測` grid `A20:H36`: seven entries for the current calendar month through current +6, each carrying `month` (`YYYY-MM-01`) and the sheet's rows as derived fields — `start`, `salary`, `spend`, `deposit_finish`, `interest`, `interest_components`, `bill`, `cells`, `deposit_return`, `cash`, `locked`, `semi_liquid`, and `ref_check`.

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
- **THEN** the `2026-09` `sc_deposit` cell lists it in `placements` and its `cash` is `55000` lower than it would be without the deposit

#### Scenario: Quarter month bill

- **WHEN** `forecast.bill_amount` is `2158` and the window covers `2027-01`
- **THEN** `2027-01` reports `bill` `-2158` and `2027-02` reports no `bill`

### Requirement: Forecast plan and placement rows

For each forecast month `m`:

- Each cell's `plan_items` SHALL list the month's stored items of its kind.
- A deposit cell's `placements` SHALL list the real deposits routed to it whose `start_date` falls in `m` — each with `deposit_id`, `label`, `bank`, `principal`, `start_date` — empty when none. Deposits without a `start_date` are never placements.
- `deposit_return` SHALL be the sum described by the deposit-returns requirement, `0` when none.

#### Scenario: Placement requires a start date

- **WHEN** a deposit has no `start_date`
- **THEN** it never appears in any cell's `placements`

### Requirement: Forecast trade lines

For each forecast month `m`, the `stock` cell's `trade_lines` SHALL list the real HK-market trades whose `trade_date` falls in `m` — each with `trade_id`, `code`, `trade_type`, `trade_date`, and a signed `amount` (`−total` for `BUY`, `+total` for `SELL`), ordered by date then id — empty when none. Every other cell's `trade_lines` SHALL be empty. US-market trades are never trade lines.

#### Scenario: HK buy becomes a negative trade line

- **WHEN** an HK `BUY` of 匯豐 dated `2026-10-27` has `total` `54063.17` and `2026-10` is in the window
- **THEN** the `2026-10` `stock` cell's `trade_lines` holds that trade with `amount` `-54063.17`

#### Scenario: HK sell becomes a positive trade line

- **WHEN** an HK `SELL` dated in a window month has `total` `12000`
- **THEN** that month's `stock` cell `trade_lines` holds it with `amount` `12000`

#### Scenario: US trades are excluded

- **WHEN** a US-market trade is dated in a window month
- **THEN** it appears in no `trade_lines` and does not affect `cash`

#### Scenario: Trades outside the window fire nothing

- **WHEN** an HK trade is dated before the window's first month
- **THEN** it appears in no column's `trade_lines` and does not affect any `cash`

### Requirement: Forecast cash figure

For each forecast month `m`:

- `cash` (活期) SHALL be `start + salary + spend + deposit_finish + interest + bill + deposit_return + Σ amounts of `tax`/`stock`/`other` items in `m` + Σ amounts of `hs_deposit`/`sc_deposit` items in `m` + Σ `amount` of the month's trade lines + Σ `amount` of the month's IBKR lines − Σ `principal` of the month's placements`, absent while `start` is absent; absent components count as `0`. (`interest`/`bill` items are already inside those figures.)

#### Scenario: Cash chains the rows

- **WHEN** `start` is present
- **THEN** `cash` derives as the stated sum with absent components counting as `0`

#### Scenario: Plan and real trade both count

- **WHEN** `2026-10` holds a `stock` item of `-50000` and a trade line of `-54063.17`
- **THEN** both reduce that month's `cash` (a combined `-104063.17`) until the user edits or deletes the plan item — stock plans are never suppressed automatically

#### Scenario: IBKR transfer lowers cash

- **WHEN** `2026-11` holds an IBKR line of `-50000`
- **THEN** `cash` for `2026-11` and every later column is `50000` lower, and `locked` is unchanged

### Requirement: Trade lines in the 股票 row

Each of the `stock` cell's `trade_lines` SHALL appear as a read-only line (code, trade type, signed `amount`, `trade_date`, marked `已交易`) in the `股票` row. The `股票` cell SHALL show the cell's `figure`, `—` while it is absent, and trade lines SHALL offer no edit, delete, or copy action.

#### Scenario: Trade shown beside a plan

- **WHEN** `2026-10` has a `stock` plan of `-50000` and a trade line 匯豐 `BUY` of `-54063.17`
- **THEN** the `股票` cell for `2026-10` shows `-104063.17` and its editor lists the plan as editable and 匯豐 as a read-only `已交易` line

#### Scenario: Trade alone fills an empty cell

- **WHEN** a month has no `stock` plans and one trade line of `-20000`
- **THEN** the `股票` cell shows `-20000` instead of `—`

### Requirement: Locked chain and semi-liquid

Each month `m` (the first included, chaining from the anchor) SHALL derive `locked(m) = locked(m−1) − deposit_finish(m) − Σ hs_deposit/sc_deposit plan amounts(m) + Σ principal of the month's placements − deposit_return(m)` — plan amounts are stored negative, so subtracting them adds to `locked`. `semi_liquid` SHALL be `cash + locked`, absent while either side is absent.

#### Scenario: Chain subtracts finishes

- **WHEN** month `m` follows a month with `deposit_finish`
- **THEN** `locked(m)` is `locked(m−1)` minus that finish plus placements, minus `deposit_return`

### Requirement: Forecast plan items

The system SHALL store forecast items — `id`, `month` (`YYYY-MM-01`), `kind`, signed `amount`, `note`, and optional `return_month` — exposed through `POST /api/forecast/:ym/items`, `PATCH /api/forecast-items/:id`, and `DELETE /api/forecast-items/:id`. `kind` SHALL be one of an enumerated set that includes `hs_deposit` and `sc_deposit` (planned lockups, stored negative — they reduce `cash`, add to `locked`, and schedule a `deposit_return`).

#### Scenario: Two placements in one cell's place

- **WHEN** items `{month: 2026-10, kind: hs_deposit, amount: -70000}` and `{month: 2026-10, kind: hs_deposit, amount: -60000}` exist
- **THEN** the `2026-10` `hs_deposit` cell lists both in `plan_items`, `cash` drops by `130000`, and `locked` rises by `130000`

#### Scenario: return_month rejected on wrong kind

- **WHEN** an item is created with `kind` `tax` and `return_month` `2027-03`
- **THEN** the request fails with 400

### Requirement: Convert plan to deposit

`POST /api/forecast-items/:id/convert` SHALL accept the deposit fields (`principal` prefilled with `−amount` by the frontend, `end_date` required, `rate`/`interest`/`start_date`/`label`/`note`/`debit_account_id` optional, `bank` defaulting from the item kind). In one transaction it SHALL create the deposit under `POST /api/deposits`'s placement rules (optional 活期 debit, start month `dep-start:<id>` item) and delete the forecast item. A rejected placement SHALL leave the forecast item in place.

#### Scenario: Conversion replaces the plan

- **WHEN** `sc_deposit` item `-180000` in `2026-10` is converted with `start_date` `2026-10-12` and `end_date` `2027-02-10`
- **THEN** a deposit row exists with `principal` `180000` and bank `SC`, the forecast item is gone, the `2026-10` `sc_deposit` cell lists the deposit in `placements` in place of the plan, and `2027-02` gains the deposit's real `deposit_finish`/`interest` instead of the plan's derived return

#### Scenario: Conversion keeps the grid totals

- **WHEN** the current month `2026-09` holds an `sc_deposit` plan `-55000` and it is converted with `start_date` `2026-09-30`, `end_date` `2027-01-05`, and `interest` `435`
- **THEN** every month's `cash` and `semi_liquid` are the same as before the conversion, except that from `2027-01` onward they are `435` higher, and every month's `locked` is unchanged

#### Scenario: Conversion records the start month item

- **WHEN** an `hs_deposit` plan `-130000` is converted with `start_date` `2026-11-03`, and the `2026-11` month row exists
- **THEN** the `2026-11` month holds a `dep-start:<id>` adjustment item of `-130000`, shown as 自動 rather than as a 建議

#### Scenario: Conversion debits the chosen account

- **WHEN** an `hs_deposit` plan `-130000` is converted with `debit_account_id` naming HS
- **THEN** HS loses `130000` and the deposit records HS as its debited account

#### Scenario: Rejected debit keeps the plan

- **WHEN** a conversion names a `debit_account_id` holding less than the principal
- **THEN** the request returns 400, no deposit is created, and the forecast item still exists

### Requirement: Deposit placement lines

Each of a deposit cell's `placements` SHALL appear as a read-only line (label, `−principal`, `start_date`, marked `已定期`) in that cell's row — `定期 HS` for the `hs_deposit` cell, `SC高息馬拉松` for the `sc_deposit` cell. The cell SHALL show the cell's `figure`, and placements SHALL offer no edit, delete, or convert action.

#### Scenario: Placement routed by bank

- **WHEN** a placement's bank is `HS`
- **THEN** its read-only line appears in the `定期 HS` row

### Requirement: Forecast IBKR transfer lines

For each forecast month `m`, the `stock` cell's `ibkr_lines` SHALL list the `ibkr_transfers` rows whose `transfer_date` falls in `m`, each with `transfer_id`, `transfer_date`, and a signed `amount` of `−amount_hkd`, ordered by date then id; empty when none. Every other cell's `ibkr_lines` SHALL be empty. Rows dated outside the window SHALL appear in no column.

#### Scenario: Transfer becomes a negative line

- **WHEN** a `50000` transfer is dated `2026-11-03` and `2026-11` is in the window
- **THEN** the `2026-11` `stock` cell's `ibkr_lines` holds it with `amount` `-50000`

#### Scenario: Transfers before the window fire nothing

- **WHEN** a transfer is dated before the window's first month
- **THEN** it appears in no column's `ibkr_lines` and affects no `cash`

### Requirement: IBKR transfer lines in the 股票 row

Each of the `stock` cell's `ibkr_lines` SHALL appear as a read-only line (`IBKR 轉入`, signed `amount`, `transfer_date`, marked `已轉帳`) in the `股票` row beside its trade lines. The `股票` cell's `figure` SHALL also include Σ its `ibkr_lines` amounts, and these lines SHALL offer no edit, delete, or copy action.

#### Scenario: Transfer alone fills an empty cell

- **WHEN** a month has no `stock` plans or trade lines and one IBKR line of `-50000`
- **THEN** the `股票` cell shows `-50000` instead of `—`

#### Scenario: Plan and real transfer both count

- **WHEN** a month holds a `stock` plan of `-50000` and an IBKR line of `-50000`
- **THEN** the cell shows `-100000` until the user edits or deletes the plan — plans are never suppressed automatically
