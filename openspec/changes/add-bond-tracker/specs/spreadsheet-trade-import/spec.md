# Spec Delta

## ADDED Requirements

### Requirement: Importing bonds from the workbook

The import command SHALL also read the `債券` sheet's registry table — the rows under the `end` column header carrying label, issue number (發行編號, e.g. `03GB2710R`), principal, and maturity date — and create one bond per row. Cells containing formulas SHALL be read as their computed values, and Excel serial dates SHALL be converted to ISO dates. Rows SHALL be imported in workbook order, recorded as each bond's `sort_order`.

#### Scenario: Importing the current workbook

- **WHEN** the import command is run against `財富分析報告.xlsx`
- **THEN** the `silver bond` row is stored with issue number `03GB2710R`, principal `50000` and maturity `2027-10-23`, and the report lists how many bonds were imported and skipped

#### Scenario: Content outside the registry table

- **WHEN** the sheet contains unrelated cells such as the `Total` header row or the coupon schedule blocks
- **THEN** they are ignored and only the registry rows create bonds

### Requirement: Importing coupon schedules from the workbook

For each bond the import SHALL also read its coupon schedule block on the `債券` sheet — a label row containing the bond's 發行編號 followed by the 付息日 / 利息釐定日 / 年息率 / 每1萬港元債券利息 header and coupon rows — and create one coupon per populated row. Cells containing formulas SHALL be read as their computed values, and Excel serial dates SHALL be converted to ISO dates. A 年息率 or 每1萬利息 cell holding 待定 SHALL be imported as an absent (null) field, leaving the coupon unfixed. A coupon whose pay date is on or before the import day SHALL be imported with `received_amount` equal to its sheet interest value; a future-dated coupon SHALL be imported unreceived.

#### Scenario: Fixed and unfixed coupons imported

- **WHEN** the silver bond's block lists three coupons at 4% and three at 待定
- **THEN** three coupons carry `annual_rate` `0.04` and their per-10k interest, and three carry neither

#### Scenario: Schedule matched to its bond

- **WHEN** a coupon block's label row reads `於2027年到期的銀色債券 (發行編號03GB2710R)`
- **THEN** its coupons are attached to the bond whose `issue_no` is `03GB2710R`

#### Scenario: Past coupon imported as received

- **WHEN** a coupon's pay date has passed and its row carries a computed interest value
- **THEN** the stored coupon's `received_amount` equals that interest value

### Requirement: Idempotent bond import

Running the import repeatedly SHALL NOT create duplicate bonds or coupons; a bond already present with the same issue number (or same label, principal and maturity when the issue number is absent) SHALL be skipped and counted as skipped, and its coupons SHALL NOT be duplicated.

#### Scenario: Re-running the import

- **WHEN** the import command is run twice against the same unchanged workbook
- **THEN** the second run creates no new bonds or coupons and reports every row as skipped

### Requirement: Bond parity report

The parity command SHALL also compare the app's bond figures against the workbook's cached values — the active principal total against the `債券` sheet's `Total` cell, and each coupon's effective amount (received amount when present, else the derived expected amount) against the cached interest column — and report any difference beyond a small floating-point tolerance. Coupons whose sheet cells hold 待定 SHALL be skipped in the amount comparison. Because active status depends on the current date, a bond that matured after the workbook last recalculated MAY legitimately differ, and the report SHALL make the differing values visible rather than failing silently.

#### Scenario: Figures match

- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the bonds section reports zero differences

#### Scenario: Figures differ

- **WHEN** the computed active principal total or a coupon's expected amount differs from the cached value beyond tolerance
- **THEN** the command lists the differing figure with both values and exits with a non-zero status
