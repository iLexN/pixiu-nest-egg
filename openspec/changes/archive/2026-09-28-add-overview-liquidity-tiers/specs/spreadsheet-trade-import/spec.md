# Spec Delta

## MODIFIED Requirements

### Requirement: Seeding Month Stat settings from the workbook

The import SHALL seed, each only when unset: `overview.salary` from `Overview!E1`, the current year's `overview.pool_rate.<year>` from `Overview!N8`, and `manual_assets` rows from `Overview`'s manual balance cells — HS (B16) and 渣打 (B17) as `cash`, Irene (B7) as `asset` with `liquidity` `short` (the sheet counts B7 in `K6` 短期可取回), and HS人壽 (B8) as `asset` with `liquidity` `long` (B8 sits in `K7` 長期可取回). It SHALL also derive each past year's pool rate from the imported figures — `(M(y) − M(y−1) + Σ娛樂(y) − Σpool_input(y)) ÷ Σ利息(y)` — and store it under `overview.pool_rate.<year>` so imported pool balances reproduce exactly.

#### Scenario: Settings seeded once

- **WHEN** the import runs against a workbook caching salary `52700`, pool rate `0.337` and the four balance cells
- **THEN** the settings are stored, and a second import leaves all of them untouched even if the user has since edited them

#### Scenario: Historical pool rate recovered

- **WHEN** 2025 imported with closing balance `5717.57` from `1667.27`, entertainment `38729.5`, pool inputs `20722.03` and interest `51900.65`
- **THEN** `overview.pool_rate.2025` stores approximately `0.425`

#### Scenario: Manual asset liquidity seeded

- **WHEN** the workbook caches Irene `B7` `20000` and HS人壽 `B8` `69440.47` and `manual_assets` is empty
- **THEN** the Irene row is stored with `liquidity` `short` and the HS人壽 row with `long`; a later user reclassification survives re-import

### Requirement: Overview parity report

The parity command SHALL also compare the app's derived Overview figures against the workbook's cached cells — asset rows `B3:B9` and `Sum` `B10`, the `C3:C9` shares, the `A13` ratio, 半流動資金 `B14`/`B15`/`B18`, `C14`, headline `B1`/`H1`/`J1`, the 美股 account cells `B1`/`B2`/`B4`/`B5`/`B7`, the averages block `G4:G8`/`H6`/`G10`, the 投資目標 block (`J22`, `K23:K26`, `L24:L26`, `M26`, `N24:N26`), and the 策略 block `K4:K7` — and report any difference beyond a small floating-point tolerance. Figures whose inputs the user has edited since import (e.g. manual balances, IBKR cells, stale GOOGLEFINANCE prices), the hand-maintained OFFSET averages window, the live pool balance, and the 投資目標 `L`/`M` cells plus the current-year `N` cell SHALL be reported as informational differences rather than failures — the sheet's per-year target formulas legitimately differ from the app's unified one. The `K4:K7` comparisons SHALL follow the same informational treatment as the headline cells they derive from, since they inherit the manual-balance and live-price drift.

#### Scenario: Block matches

- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the Overview section reports zero differences beyond the expected drift on manual, live-priced, averages-window, and 投資目標 target cells

#### Scenario: Seeded 2023 invested matches

- **WHEN** import seeded the 2023 `invested_adjustment` `110000` and the sheet's `K23` is `206523.15`
- **THEN** the `K23` comparison reports a match and 2024's `N24` growth comparison matches too

#### Scenario: 策略 cells compared

- **WHEN** the sheet caches `K5` `316200` and the app stores salary `52700`
- **THEN** the `K5` comparison reports a match, and `K4`/`K6`/`K7` are compared against the app's `can_use`/`short_term`/`long_term`
