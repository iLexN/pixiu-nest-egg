# Spreadsheet Trade Import Specification

## Purpose

Moves the existing trade history out of `財富分析報告.xlsx` into the app without retyping, and proves the app reproduces the spreadsheet's own numbers before the owner switches workflow.

## Requirements

### Requirement: Importing trades from the workbook
The system SHALL provide a command that reads a given workbook file and imports the HK trade rows from the `港股Trade` sheet (股票代碼, 股數, buy total, 單價, 日期, 類別) and the US trade rows from the `美股Trade` sheet (股票代碼, 股數, 單價, fee, 日期, 類別), creating the corresponding trades. Cells containing formulas SHALL be read as their computed values.

#### Scenario: Importing the current workbook
- **WHEN** the import command is run against `財富分析報告.xlsx`
- **THEN** all 24 HK trade rows and all 10 US trade rows are stored, and a report lists how many trades and stocks were created per market

#### Scenario: Formula cell
- **WHEN** a source cell holds a formula such as `=32825.78+70`
- **THEN** the computed value 32895.78 is imported

#### Scenario: Rows outside the trade table
- **WHEN** the sheet contains unrelated columns such as the 派息 table in `港股Trade` columns J–O, or template/scratch cells
- **THEN** they are ignored and only the trade table columns are imported

#### Scenario: Blank rows
- **WHEN** a row inside the scanned range has no 股票代碼
- **THEN** it is skipped without failing the import

### Requirement: Importing stock metadata
The import SHALL create each referenced stock and populate its ticker, exchange and sector from the `港股` and `美股` summary sheets where available, including stocks listed there that have no trades.

#### Scenario: HK stock metadata
- **WHEN** `香港中華煤氣` appears on the `港股` sheet with ticker `0003` and sector `Utilities`
- **THEN** the imported stock carries that ticker and sector, and exchange `HKG`

#### Scenario: US stock metadata
- **WHEN** `VOO` appears on the `美股` sheet with exchange `NYSEARCA`
- **THEN** the imported stock carries exchange `NYSEARCA`

#### Scenario: Listed stock with no trades
- **WHEN** a stock appears on a summary sheet but has no trade rows
- **THEN** the stock is still created, with no trades

### Requirement: Idempotent import
Running the import repeatedly SHALL NOT create duplicate stocks or duplicate trades; a trade already present with the same stock, 日期, 類別, 股數 and total SHALL be skipped and counted as skipped in the report.

#### Scenario: Re-running the import
- **WHEN** the import command is run twice against the same unchanged workbook
- **THEN** the second run creates no new trades or stocks and reports every row as skipped

#### Scenario: Genuine duplicate trades in the source
- **WHEN** the workbook legitimately contains two identical trade rows for the same stock, date, quantity and total
- **THEN** the import preserves both rows on the first run and still creates nothing new on a second run

### Requirement: Parity report against the workbook
The system SHALL provide a command that compares its computed per-stock summary figures (shares held, 總買入成本, 加權平均買入單價) against the corresponding cached values on the workbook's `港股` and `美股` sheets, and reports any difference beyond a small floating-point tolerance.

#### Scenario: Figures match
- **WHEN** the parity command is run after a successful import
- **THEN** it reports zero differences and exits successfully

#### Scenario: Figures differ
- **WHEN** any stock's computed shares, cost or average price differs from the workbook value beyond tolerance
- **THEN** the command lists each differing stock with both values and exits with a non-zero status

### Requirement: Workbook is never modified
The import and parity commands SHALL open the workbook read-only and SHALL NOT write to it, so the existing spreadsheet keeps working during the migration.

#### Scenario: Workbook untouched
- **WHEN** the import and parity commands have been run
- **THEN** `財富分析報告.xlsx` is byte-for-byte unchanged

#### Scenario: Missing workbook
- **WHEN** the given workbook path does not exist or lacks the expected sheets
- **THEN** the command fails with a clear message naming the file or missing sheet, and stores nothing

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
