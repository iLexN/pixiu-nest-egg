# Spec Delta

## ADDED Requirements

### Requirement: Import MPF accounts

The import command SHALL also read the `MPF` sheet's account table — the rows under the 總供款額 / 帳戶結存 headers — and create one MPF account per row, carrying the label (column A), trustee (column B), 總供款額, 帳戶結存, 計劃名稱, and 成員編號 columns. Cells containing formulas SHALL be read as their computed values. Rows SHALL be imported in workbook order, recorded as each account's `sort_order`. The fund-details table and the remark row on the `MPF` sheet SHALL NOT be imported.

#### Scenario: Account rows imported

- **WHEN** the workbook's `MPF` sheet has account rows `new type` and `強積金個人帳戶`
- **THEN** two MPF accounts are created with their contributions, balances, trustee, and metadata

#### Scenario: Fund details skipped

- **WHEN** the `MPF` sheet contains the fund-details table below the account rows
- **THEN** no fund records are created from it

### Requirement: Seed MPF history and maxima

For each imported MPF account the import SHALL seed one synthetic history row dated at the end of the previous calendar month, reconstructing its balance from the cached last-month rate. When the sheet has exactly two accounts, the per-account rates plus the portfolio's cached last-month rate and gain SHALL be used to recover the actual month-end contributions; otherwise the seed uses the current contributions and its gain is an approximation because contributions have since grown. The import SHALL also seed the account's `max_rate` and `max_gain` high-water marks from the cached max column.

#### Scenario: Last-month seeded

- **WHEN** an account row carries a cached last-month rate of `0.2855`
- **THEN** after import the account's last-month figures show rate `0.2855`, sourced from a synthetic previous-month history row

#### Scenario: Max seeded

- **WHEN** an account row carries a cached max rate of `0.5273`
- **THEN** the account's stored max rate is `0.5273` and a lower current rate does not overwrite it

### Requirement: MPF import idempotent

Running the import repeatedly SHALL NOT create duplicate MPF accounts; an account already present with the same label SHALL be skipped and counted as skipped, and its seeded history row SHALL NOT be duplicated.

#### Scenario: Second import skips

- **WHEN** the import is run a second time on the same workbook
- **THEN** no MPF accounts or history rows are added, and the report counts the existing accounts as skipped

### Requirement: MPF parity comparison

The parity command SHALL also compare the app's computed MPF figures — per-account contributions, balance, and 回報率, plus the portfolio buy/now/rate/net-gain totals — against the workbook's cached `MPF` sheet values, and report any difference beyond a small floating-point tolerance. Seeded last-month and max net gains SHALL be compared with tolerance or reported as informational rather than failing, because the workbook stores rates only and the seeded gains are reconstructions.

#### Scenario: Totals match

- **WHEN** the imported contributions and balances equal the workbook's
- **THEN** the parity report shows the MPF buy/now/rate figures as matching

#### Scenario: Seeded gain approximation reported

- **WHEN** the seeded last-month net gain differs from the workbook's cached figure because contributions grew since the snapshot
- **THEN** the parity report shows both values rather than silently failing
