## MODIFIED Requirements

### Requirement: Parity report against the workbook
The system SHALL provide a command that compares its computed per-stock summary figures (shares held, 總買入成本, 加權平均買入單價) against the corresponding cached values on the workbook's `港股` and `美股` sheets, and reports any difference beyond a small floating-point tolerance. For the `港股` sheet the command SHALL also compare the cached 累計派息 (K), 累計派息% (P), 淨投入總本金 (U) and 淨攤薄單價 (V) columns, computing the dividend-dependent figures from each stock's effective dividend amount — received amount when present, else the estimated amount — because the sheet's K column sums every J–O row including not-yet-received ones. The cached 實質動態總回報% (W) SHALL NOT be compared: the sheet's market value uses live market-data prices while the app uses manual 現價, so differences there do not indicate an import problem.

#### Scenario: Figures match
- **WHEN** the parity command is run after a successful import
- **THEN** it reports zero differences and exits successfully

#### Scenario: Figures differ
- **WHEN** any stock's computed shares, cost or average price differs from the workbook value beyond tolerance
- **THEN** the command lists each differing stock with both values and exits with a non-zero status

#### Scenario: Dividend-adjusted figure differs
- **WHEN** any HK stock's effective 累計派息, 累計派息%, 淨投入總本金 or 淨攤薄單價 differs from the workbook's cached value beyond tolerance
- **THEN** the command lists the stock with both values and exits with a non-zero status
