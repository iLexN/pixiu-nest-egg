# Spec Delta

## MODIFIED Requirements

### Requirement: Overview parity report

The parity command SHALL also compare the app's derived Overview figures against the workbook's cached cells — asset rows `B3:B9` and `Sum` `B10`, the `C3:C9` shares, the `A13` ratio, 半流動資金 `B14`/`B15`/`B18`, `C14`, headline `B1`/`H1`/`J1`, the 美股 account cells `B1`/`B2`/`B4`/`B5`/`B7`, the averages block `G4:G8`/`H6`/`G10`, and the 投資目標 block (`J22`, `K23:K26`, `L24:L26`, `M26`, `N24:N26`) — and report any difference beyond a small floating-point tolerance. Figures whose inputs the user has edited since import (e.g. manual balances, IBKR cells, stale GOOGLEFINANCE prices), the hand-maintained OFFSET averages window, the live pool balance, and the 投資目標 `L`/`M` cells plus the current-year `N` cell SHALL be reported as informational differences rather than failures — the sheet's per-year target formulas legitimately differ from the app's unified one.

#### Scenario: Block matches

- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the Overview section reports zero differences beyond the expected drift on manual, live-priced, averages-window, and 投資目標 target cells

#### Scenario: Seeded 2023 invested matches

- **WHEN** import seeded the 2023 `invested_adjustment` `110000` and the sheet's `K23` is `206523.15`
- **THEN** the `K23` comparison reports a match and 2024's `N24` growth comparison matches too
