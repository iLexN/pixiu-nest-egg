## ADDED Requirements

### Requirement: Importing deposits from the workbook
The import command SHALL also read the `表_定期List` table on the `定期Info` sheet — located by its `status`/`id`/`input`/`rate`/`利息`/`total`/`end date`/`note1`/`note2` headers — and create one deposit per row. Cells containing formulas SHALL be read as their computed values. The derived `status`, `total`, `month`, and `year` columns SHALL NOT be stored. Rows SHALL be imported in workbook order, recorded as each deposit's `sort_order`.

#### Scenario: Importing the current workbook
- **WHEN** the import command is run against `財富分析報告.xlsx`
- **THEN** every deposit row of `表_定期List` (23 rows at the time of writing) is stored, and the report lists how many deposits were imported and skipped

#### Scenario: Formula and blank cells
- **WHEN** a source `input` cell holds a formula such as `=90000-32000` or a `rate`/`利息`/`id` cell is blank
- **THEN** the computed value is imported, and blanks become absent (null) fields

#### Scenario: Content outside the table
- **WHEN** the sheet contains unrelated cells such as the 渣打高息馬拉松 scratch block or the year tables in columns A–D
- **THEN** they are ignored and only the `表_定期List` rows are imported

### Requirement: Idempotent deposit import
Running the import repeatedly SHALL NOT create duplicate deposits; a deposit already present with the same label, end date, principal, and interest SHALL be skipped and counted as skipped, while genuinely duplicated source rows are preserved on first import.

#### Scenario: Re-running the import
- **WHEN** the import command is run twice against the same unchanged workbook
- **THEN** the second run creates no new deposits and reports every row as skipped

### Requirement: Deposit parity report
The parity command SHALL also compare the app's computed deposit aggregates against the workbook's cached values — the active principal total (`定期!B1`), the active month rollup and bank rollups on `定期`, and the per-year month tables on `定期Info` — and report any difference beyond a small floating-point tolerance. Because active status depends on the current date, a deposit that matured after the workbook last recalculated MAY legitimately differ, and the report SHALL make the differing values visible rather than failing silently.

#### Scenario: Aggregates match
- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the deposit section reports zero differences

#### Scenario: Aggregates differ
- **WHEN** a computed month or bank rollup differs from the cached value beyond tolerance
- **THEN** the command lists the differing figure with both values and exits with a non-zero status
