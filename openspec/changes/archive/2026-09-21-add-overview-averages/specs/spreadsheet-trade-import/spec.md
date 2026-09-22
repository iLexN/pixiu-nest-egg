# Spec Delta

## MODIFIED Requirements

### Requirement: Overview parity report

The parity command SHALL also compare the app's derived Overview figures against the workbook's cached cells — asset rows `B3:B9` and `Sum` `B10`, the `C3:C9` shares, the `A13` ratio, 半流動資金 `B14`/`B15`/`B18`, `C14`, headline `B1`/`H1`/`J1`, the 美股 account cells `B1`/`B2`/`B4`/`B5`/`B7`, and the averages block `G4:G8`/`H6`/`G10` — and report any difference beyond a small floating-point tolerance. Figures whose inputs the user has edited since import (e.g. manual balances, IBKR cells, stale GOOGLEFINANCE prices), the hand-maintained OFFSET averages window, and the live pool balance SHALL be reported as informational differences rather than failures.

#### Scenario: Block matches

- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the Overview section reports zero differences beyond the expected drift on manual, live-priced, and averages-window cells
