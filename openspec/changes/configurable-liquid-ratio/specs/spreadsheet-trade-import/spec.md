# Spec Delta

## MODIFIED Requirements

### Requirement: Overview parity report

The parity command SHALL also compare the app's derived Overview figures against the workbook's cached cells — asset rows `B3:B9` and `Sum` `B10`, the `C3:C9` shares, the `A13` ratio, 半流動資金 `B14`/`B15`/`B18`, `C14`, headline `B1`/`H1`/`J1`, the 美股 account cells `B1`/`B2`/`B4`/`B5`/`B7`, the averages block `G4:G8`/`H6`/`G10`, the 投資目標 block (`J22`, `K23:K26`, `L24:L26`, `M26`, `N24:N26`), and the 策略 block `K4:K7` — and report any difference beyond a small floating-point tolerance. The `C14` comparison SHALL check the sheet's own literal formula — `semi_liquid.total − 25% × liquid_assets` recomputed from the response's other fields — rather than the response's `vs_quarter_liquid`, which follows the configured `overview.semi_liquid_target` ratio. Figures whose inputs the user has edited since import (e.g. manual balances, IBKR cells, stale GOOGLEFINANCE prices), the hand-maintained OFFSET averages window, the live pool balance, and the 投資目標 `L`/`M` cells plus the current-year `N` cell SHALL be reported as informational differences rather than failures — the sheet's per-year target formulas legitimately differ from the app's unified one. The `K4:K7` comparisons SHALL follow the same informational treatment as the headline cells they derive from, since they inherit the manual-balance and live-price drift.

#### Scenario: Block matches

- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the Overview section reports zero differences beyond the expected drift on manual, live-priced, averages-window, and 投資目標 target cells

#### Scenario: Seeded 2023 invested matches

- **WHEN** import seeded the 2023 `invested_adjustment` `110000` and the sheet's `K23` is `206523.15`
- **THEN** the `K23` comparison reports a match and 2024's `N24` growth comparison matches too

#### Scenario: 策略 cells compared

- **WHEN** the sheet caches `K5` `316200` and the app stores salary `52700`
- **THEN** the `K5` comparison reports a match, and `K4`/`K6`/`K7` are compared against the app's `can_use`/`short_term`/`long_term`

#### Scenario: Configured ratio does not disturb the `C14` check

- **WHEN** `overview.semi_liquid_target` is set to `0.3` and the parity command is run
- **THEN** the `C14` comparison still checks `semi_liquid.total − 0.25 × liquid_assets` against the workbook's cached `C14` and reports a match while the sheet's formula holds
