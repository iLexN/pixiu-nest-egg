# Spec Delta

## MODIFIED Requirements

### Requirement: IBKR account figures

The system SHALL store the IBKR app's displayed account total as the setting `ibkr.reported_value` (`B2`; the app uses a different FX rate than the stored USD fx rate). The cash positions SHALL be the amounts of the `broker` cash accounts: `hkd_cash` (`B4`) from the HKD broker account, `usd_cash` (`B5`) from the USD broker account.

#### Scenario: Computed total

- **WHEN** US stocks are worth 15813.73 USD, `usd_cash` is 1200.25, `hkd_cash` is 765.42, and the rate is 7.845135
- **THEN** `computed_total_hkd` is approximately 134242.39

#### Scenario: Net vs transferred

- **WHEN** `transferred_hkd` is 131000 and `now_value` is 134232.01
- **THEN** `net` is 3232.01 and `net_pct` is approximately 0.0247

#### Scenario: Clearing a field

- **WHEN** the user patches `usd_cash` to `null`
- **THEN** the USD broker account's amount becomes `0` and `computed_total_hkd` no longer includes any USD cash

#### Scenario: Cash edit is the account balance

- **WHEN** the user patches `usd_cash` to `1200.25`
- **THEN** `GET /api/cash-accounts` reports the USD broker account at `1200.25`

#### Scenario: Recording a transfer

- **WHEN** the user posts `amount_hkd` `50000` to `POST /api/ibkr/transfers` with no `transfer_date`
- **THEN** a row dated today is appended to `ibkr_transfers` and `transferred_hkd` grows by `50000`

#### Scenario: Undoing a transfer

- **WHEN** the user deletes a `10000` transfer through `DELETE /api/ibkr/transfers/{id}`
- **THEN** its row is removed and `transferred_hkd` drops by `10000`; no negative row is appended

#### Scenario: A transfer raises the HKD cash position

- **WHEN** a 轉入 of `50000` is recorded while `hkd_cash` is `4439.55`
- **THEN** `hkd_cash` becomes `54439.55` and `transferred_hkd` grows by `50000`

### Requirement: IBKR transfers log

The system SHALL keep a dated `ibkr_transfers` log whose sum is `transferred_hkd` (美股 `B1`: cumulative bank→IBKR transfers in HKD, absent while the log is empty); each year's sum is the year's IBKR 轉入. Each row SHALL record the source bank account and the IBKR HKD broker account it moved cash between; rows recorded before transfers moved cash carry neither and are log-only.

#### Scenario: Yearly transfer sum

- **WHEN** the log holds transfers dated in a year
- **THEN** that year's IBKR 轉入 is their sum

#### Scenario: Legacy rows stay log-only

- **WHEN** the log holds the pre-existing rows dated `2026-06-02` (`131000`) and `2026-10-30` (`90000`)
- **THEN** they still count in `transferred_hkd` and their years' 轉入, and carry no source or broker account

### Requirement: IBKR patch semantics

The IBKR figures are exposed through `GET /api/ibkr` and `PATCH /api/ibkr`, as `now_value`, `hkd_cash` and `usd_cash`. `now_value` SHALL accept a finite non-negative number, or `null` to clear. `hkd_cash`/`usd_cash` SHALL accept a finite non-negative number, which is written to the broker account, or `null`, which sets that account to `0`. One PATCH SHALL apply all its fields in a single transaction. `PATCH /api/ibkr` SHALL NOT record transfers.

#### Scenario: Patch writes the broker account

- **WHEN** `PATCH /api/ibkr` carries `usd_cash` `1234.56`
- **THEN** the USD broker cash account's amount becomes `1234.56`

#### Scenario: Patch never appends a transfer

- **WHEN** `PATCH /api/ibkr` is sent with any combination of its fields
- **THEN** `ibkr_transfers` gains no row and no bank account balance changes

### Requirement: IBKR transfer entry

`POST /api/ibkr/transfers` with `amount_hkd`, optional `transfer_date`, and optional `from_account_id` SHALL, in one transaction, append a log row, subtract `amount_hkd` from the source bank account, add it to the IBKR HKD broker account, and record the month's 調整 item (see month-stat). It SHALL respond `201` with the created transfer.

#### Scenario: Negative delta undoes an entry

- **WHEN** the user posts a negative `amount_hkd`
- **THEN** the response is `400` and nothing is recorded; a mistyped entry is undone by deleting its row instead

#### Scenario: Transfer moves the cash

- **WHEN** HS holds `65608.15`, IBKR HKD holds `4439.55`, and the user posts `amount_hkd` `50000` dated `2026-11-03`
- **THEN** HS becomes `15608.15`, IBKR HKD becomes `54439.55`, `transferred_hkd` grows by `50000`, and the row records both accounts

#### Scenario: Totals are unchanged by a transfer

- **WHEN** a transfer is recorded
- **THEN** `total_assets` and `liquid_assets` are unchanged, because the amount leaves bank cash and enters IBKR HKD cash, both of which they count

### Requirement: Forecast cash figure

For each forecast month `m`:

- `cash` (活期) SHALL be `start + salary + spend + deposit_finish + interest + bill + deposit_return + Σ amounts of `tax`/`stock`/`other` items in `m` + Σ amounts of `hs_deposit`/`sc_deposit` items in `m` + Σ `amount` of `trade_lines` in `m` + Σ `amount` of `ibkr_lines` in `m` − Σ `principal` of `deposit_placements` in `m`, absent while `start` is absent; absent components count as `0`. (`interest`/`bill` items are already inside those figures.)

#### Scenario: Cash chains the rows

- **WHEN** `start` is present
- **THEN** `cash` derives as the stated sum with absent components counting as `0`

#### Scenario: Plan and real trade both count

- **WHEN** `2026-10` holds a `stock` item of `-50000` and a trade line of `-54063.17`
- **THEN** both reduce that month's `cash` (a combined `-104063.17`) until the user edits or deletes the plan item — stock plans are never suppressed automatically

#### Scenario: IBKR transfer lowers cash

- **WHEN** `2026-11` holds an IBKR line of `-50000`
- **THEN** `cash` for `2026-11` and every later column is `50000` lower, and `locked` is unchanged

### Requirement: Overview IBKR editable card

The IBKR block SHALL be the same editable card as the 美股 summary: editable `now_value`, `hkd_cash`, and `usd_cash` inputs saving through `PATCH /api/ibkr`, and a separate 「轉入」 form saving through `POST /api/ibkr/transfers`, beside the derived figures. The card SHALL NOT show the transfer history; it lives on the 股票 → 轉入記錄 tab. The view SHALL reload after any save so the asset-table IBKR row and every dependent figure refresh.

#### Scenario: IBKR card save refreshes dependents

- **WHEN** the user saves the IBKR card
- **THEN** the view reloads so the asset-table IBKR row and every dependent figure refresh

#### Scenario: 轉入 refreshes the cash rows

- **WHEN** the user records a 轉入 from the card
- **THEN** the view reloads showing the HS row, 活期, IBKR HKD 現金, and 累計轉入 updated

#### Scenario: 轉入 form fields

- **WHEN** the user opens the card's 「轉入」 form
- **THEN** it offers an amount, a date defaulting to and capped at today, a source selector of HKD bank accounts preselecting `HS`, and a before→after balance preview of the source and IBKR HKD

#### Scenario: 編輯 no longer offers 轉入

- **WHEN** the user opens the card's 編輯 form
- **THEN** it offers only `now_value`, `hkd_cash`, and `usd_cash`

## ADDED Requirements

### Requirement: IBKR transfer source account

The source SHALL be `from_account_id` when given, else the `bank` account labelled `HS`. The source SHALL be an HKD `bank` account; an unknown id SHALL respond `404`, and a broker or non-HKD account, or no `HS` account while `from_account_id` is omitted, SHALL respond `400`.

#### Scenario: Default source is HS

- **WHEN** the user posts a transfer without `from_account_id`
- **THEN** the `HS` bank account is debited, even if another HKD account is the default receipt account

#### Scenario: Explicit source

- **WHEN** the user posts a transfer with `from_account_id` of the 渣打 account
- **THEN** 渣打 is debited and HS is unchanged

#### Scenario: Broker source rejected

- **WHEN** `from_account_id` names the IBKR HKD broker account
- **THEN** the response is `400` and nothing changes

### Requirement: IBKR transfer validation

`amount_hkd` SHALL be a finite number greater than `0`. `transfer_date` SHALL be `YYYY-MM-DD`, not after today, defaulting to today. A transfer larger than the source account's current amount SHALL respond `400`. Any rejection SHALL leave the log, balances, and month items unchanged.

#### Scenario: Non-positive amount rejected

- **WHEN** `amount_hkd` is `0`, negative, or not a number
- **THEN** the response is `400`

#### Scenario: Future date rejected

- **WHEN** `transfer_date` is after today
- **THEN** the response is `400`

#### Scenario: Insufficient balance rejected

- **WHEN** HS holds `1000` and the user posts `amount_hkd` `5000` from HS
- **THEN** the response is `400` naming the insufficient balance, and nothing changes

### Requirement: IBKR transfer list

`GET /api/ibkr/transfers` SHALL list every log row, newest `transfer_date` first (then newest id), each with `id`, `transfer_date`, `amount_hkd`, `from_account_id`, `from_account_label`, `broker_account_id`, and `created_at`; the account fields are `null` for log-only rows or deleted accounts.

#### Scenario: Legacy and new rows listed

- **WHEN** the log holds the two legacy rows and one new transfer from HS
- **THEN** the list shows the new transfer first with `from_account_label` `HS`, and the legacy rows with null account fields

### Requirement: IBKR transfer history page

The 股票 → 轉入記錄 page SHALL list `GET /api/ibkr/transfers` rows in its order, showing `transfer_date`, `amount_hkd`, and `from_account_label` (`—` while null), with a 累計 total of the listed amounts and a confirmed 刪除 per row calling `DELETE /api/ibkr/transfers/{id}`, then reloading the list. It SHALL show a placeholder while the log is empty.

#### Scenario: History lists legacy and new transfers

- **WHEN** the log holds the two legacy rows and a `50000` transfer from HS dated `2026-11-03`
- **THEN** the page lists `2026-11-03` `50000` `HS` first, then `2026-10-30` `90000` `—` and `2026-06-02` `131000` `—`, with a 累計 of `271000`

#### Scenario: Delete from the history page

- **WHEN** the user clicks 刪除 on a transfer and confirms
- **THEN** the transfer is deleted with its exact reversal and the list reloads without it

#### Scenario: Cancelled delete changes nothing

- **WHEN** the user clicks 刪除 and cancels the confirmation
- **THEN** no request is sent and the list is unchanged

### Requirement: IBKR transfer deletion

`DELETE /api/ibkr/transfers/{id}` SHALL, in one transaction, add the amount back to the recorded source account, subtract it from the recorded broker account, remove the transfer's month item and dismissals, and delete the row, responding `204`. A side whose account is unrecorded or deleted SHALL be skipped. An unknown id SHALL respond `404`.

#### Scenario: Delete reverses exactly

- **WHEN** the user deletes a `50000` transfer from HS
- **THEN** HS and IBKR HKD return to their pre-transfer amounts, its `ibkr:<id>` month item is gone, and `transferred_hkd` drops by `50000`

#### Scenario: Deleting a log-only row moves no cash

- **WHEN** the user deletes a legacy row
- **THEN** the row is removed and no account balance or month item changes

#### Scenario: Source account deleted

- **WHEN** the transfer's source account was deleted before the transfer is deleted
- **THEN** only the IBKR HKD side is reversed

### Requirement: Forecast IBKR transfer lines

For each forecast month `m`, `ibkr_lines` SHALL list the `ibkr_transfers` rows whose `transfer_date` falls in `m`, each with `transfer_id`, `transfer_date`, and a signed `amount` of `−amount_hkd`, ordered by date then id; empty when none. Rows dated outside the window SHALL appear in no column.

#### Scenario: Transfer becomes a negative line

- **WHEN** a `50000` transfer is dated `2026-11-03` and `2026-11` is in the window
- **THEN** the `2026-11` column's `ibkr_lines` holds it with `amount` `-50000`

#### Scenario: Transfers before the window fire nothing

- **WHEN** a transfer is dated before the window's first month
- **THEN** it appears in no column's `ibkr_lines` and affects no `cash`

### Requirement: IBKR transfer lines in the 股票 row

Each of the month's `ibkr_lines` SHALL appear as a read-only line (`IBKR 轉入`, signed `amount`, `transfer_date`, marked `已轉帳`) in the `股票` row beside its trade lines. The `股票` cell's figure SHALL also include Σ its `ibkr_lines` amounts, and these lines SHALL offer no edit, delete, or copy action.

#### Scenario: Transfer alone fills an empty cell

- **WHEN** a month has no `stock` plans or trade lines and one IBKR line of `-50000`
- **THEN** the `股票` cell shows `-50000` instead of `—`

#### Scenario: Plan and real transfer both count

- **WHEN** a month holds a `stock` plan of `-50000` and an IBKR line of `-50000`
- **THEN** the cell shows `-100000` until the user edits or deletes the plan — plans are never suppressed automatically
