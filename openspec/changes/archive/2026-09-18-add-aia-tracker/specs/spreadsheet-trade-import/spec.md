# Spec Delta

## ADDED Requirements

### Requirement: Importing AIA policies from the workbook

The import command SHALL also read the `AIA` sheet's policy rows — the rows in the policy block carrying numeric `buy usd` and `now usd` cells together with a label (name column) or policy number — and create one policy per row with its `next pay` date, `remaining years`, `Withdrew` amount and remark text. Cells containing formulas SHALL be read as their computed values, and Excel serial dates SHALL be converted to ISO dates. A row with no label cell SHALL inherit the label of the nearest labelled row above it (the sheet groups several policy numbers under one plan name). Rows SHALL be imported in workbook order, recorded as each policy's `sort_order`.

The import SHALL reproduce the sheet's totals flags: the `irene 20%` share row (excluded from the sheet's `buy usd`/`now usd` sums) SHALL be imported with `excluded` set, and the out-of-block `irene 年金` row (counted in those sums but outside the sheet's `AIA display value` range) SHALL be imported with `in_account` unset. Scratch cells outside the policy block SHALL be ignored.

#### Scenario: Importing the current workbook

- **WHEN** the import command is run against `財富分析報告.xlsx`
- **THEN** the eight policy rows are stored in sheet order — including `B632611401` under label `年金 - 2024 - 2029` with next pay `2026-07-01`, premium `24960` and value `14284.35` — and the report lists how many policies were imported and skipped

#### Scenario: Continuation row inherits the plan label

- **WHEN** a policy row such as `B335167809` has no label cell of its own
- **THEN** it is stored under the label of the row above it (`年金 - 2024 - 2029`)

#### Scenario: Share rows keep the sheet's totals flags

- **WHEN** the import reads the `irene 20%` row and the `irene 年金` row
- **THEN** `irene 20%` is stored excluded from totals, and `irene 年金` is stored out of `display_value` while still counting toward totals

#### Scenario: Content outside the policy block

- **WHEN** the sheet contains unrelated cells such as the HKD summary block or the `GG` scratch cells
- **THEN** they are ignored and only the policy rows create policies

### Requirement: Seed AIA exchange rate from cached cell

The import SHALL seed the `aia.usd_hkd_rate` meta value from the workbook's cached `Overview!N3` cell so the app's HKD figures start from the same rate the sheet last used. A rate already stored SHALL NOT be overwritten on re-import.

#### Scenario: Rate seeded once

- **WHEN** the import runs against a workbook caching `7.84522932` in `Overview!N3`
- **THEN** the stored rate is `7.84522932`, and a second import leaves it untouched even if the user has since edited it

### Requirement: Idempotent AIA import

Running the import repeatedly SHALL NOT create duplicate policies; a policy already present with the same policy number (or same label, premium and value when the policy number is absent) SHALL be skipped and counted as skipped.

#### Scenario: Re-running the import

- **WHEN** the import command is run twice against the same unchanged workbook
- **THEN** the second run creates no new policies and reports every row as skipped

### Requirement: AIA parity report

The parity command SHALL also compare the app's AIA figures against the workbook's cached values — each policy's premium and value against its `buy usd`/`now usd` cells, and the derived totals (`buy usd`, `now usd`, `AIA display value`, withdrew, `balance %%`, and the HKD summary cells converted with the same seeded rate) against the sheet's cached summary cells — and report any difference beyond a small floating-point tolerance.

#### Scenario: Figures match

- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the AIA section reports zero differences

#### Scenario: Figures differ

- **WHEN** a computed premium, value or total differs from the cached value beyond tolerance
- **THEN** the command lists the differing figure with both values and exits with a non-zero status
