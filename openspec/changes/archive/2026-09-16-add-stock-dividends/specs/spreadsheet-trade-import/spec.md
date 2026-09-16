## ADDED Requirements

### Requirement: Importing dividends from the workbook
The import SHALL also read the dividend block in columns J–O of each market's trade sheet — J stock name, K pay date (accepting Excel serial numbers), M 派息 amount, L yield-on-cost rate, N yield-on-price rate, O 股數 — and create one dividend record per populated row. Cells containing formulas SHALL be read as their computed values. `shares_held` SHALL be taken from O when present, `buy_cost` recovered as M ÷ L when L is present (otherwise both snapshots are derived from imported trades on or before the pay date), and `received_price` recovered as M ÷ (N × O) when N and O are present. A row with an N rate or a pay date on or before the import day SHALL be imported as received (received_amount = M); a future-dated row without an N rate SHALL be imported as pending with estimated_amount = M. When M is a formula its text SHALL be preserved in the record's note.

#### Scenario: Importing the current workbook
- **WHEN** the import command is run against `財富分析報告.xlsx`
- **THEN** every populated J–O row of `港股Trade` (and `美股Trade`, when used) is stored as a dividend, and the report lists how many dividends were imported and skipped per market

#### Scenario: Snapshot recovery from cached rates
- **WHEN** a row's L cell caches 0.0701 for an M of 6767.35
- **THEN** the stored buy_cost is approximately 96523.15 — the sheet's hardcoded denominator — rather than a recomputed figure

#### Scenario: Formula amount
- **WHEN** a row's M cell holds `=(0.12*4000)-30` with cached value 450
- **THEN** the dividend amount is 450 and the formula text is preserved in the note

#### Scenario: Pending import row
- **WHEN** a row's pay date is in the future and it has no N rate
- **THEN** it is imported as pending with estimated_amount equal to M

### Requirement: Idempotent dividend import
Running the import repeatedly SHALL NOT create duplicate dividends; a dividend already present with the same stock, pay date, and amount SHALL be skipped and counted as skipped in the report.

#### Scenario: Re-running the import
- **WHEN** the import command is run twice against the same unchanged workbook
- **THEN** the second run creates no new dividends and reports every dividend row as skipped

### Requirement: Dividend parity report
The parity command SHALL also compare the imported dividend figures against the workbook's cached J–O block — row count and total 派息 amount per market — and report any difference beyond a small floating-point tolerance.

#### Scenario: Figures match
- **WHEN** the parity command is run after a successful import
- **THEN** the dividend section reports zero differences

#### Scenario: Figures differ
- **WHEN** the stored dividend count or total for a market differs from the workbook block beyond tolerance
- **THEN** the command lists the differing figure with both values and exits with a non-zero status
