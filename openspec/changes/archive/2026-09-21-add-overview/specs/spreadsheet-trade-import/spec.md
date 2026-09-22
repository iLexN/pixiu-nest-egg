# Spec Delta

## ADDED Requirements

### Requirement: Seeding IBKR account figures from the workbook

The import SHALL seed, each only when unset, the `app_meta` keys `ibkr.transferred_hkd`, `ibkr.now_value`, `ibkr.hkd_cash`, and `ibkr.usd_cash` from the 美股 sheet's cached cells `B1`, `B2`, `B4`, and `B5` respectively. A second import SHALL leave all of them untouched even if the user has since edited them.

#### Scenario: IBKR figures seeded once

- **WHEN** the import runs against a workbook caching 美股 `B1` `131000`, `B2` `134232.01`, `B4` `765.42`, `B5` `1200.25`
- **THEN** the four keys are stored, and a second import leaves them untouched even if the user has since edited them

### Requirement: Overview parity report

The parity command SHALL also compare the app's derived Overview figures against the workbook's cached cells — asset rows `B3:B9` and `Sum` `B10`, the `C3:C9` shares, the `A13` ratio, 半流動資金 `B14`/`B15`/`B18`, `C14`, headline `B1`/`H1`/`J1`, and the 美股 account cells `B1`/`B2`/`B4`/`B5`/`B7` — and report any difference beyond a small floating-point tolerance. Figures whose inputs the user has edited since import (e.g. manual balances, IBKR cells, stale GOOGLEFINANCE prices) SHALL be reported as informational differences rather than failures.

#### Scenario: Block matches

- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the Overview section reports zero differences beyond the expected drift on manual and live-priced cells
