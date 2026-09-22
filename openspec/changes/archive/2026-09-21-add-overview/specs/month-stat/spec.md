# Spec Delta

## MODIFIED Requirements

### Requirement: Frozen and live asset totals

Each month SHALL carry `total_assets` (總數, the sheet's `Overview!B1`) and `liquid_assets` (流動資產, `Overview!H1`). A stored value SHALL win; when absent the figure SHALL be derived live as: `total_assets` = HK market value + (US market value + IBKR USD cash) × rate + IBKR HKD cash + active 定期 principal + active 債券 principal + AIA value × rate + MPF balance + Σ manual `asset` balances + Σ `cash` balances, and `liquid_assets` = HK market value + active 定期 principal + cash + active 債券 principal + (US market value + IBKR USD cash) × rate + IBKR HKD cash − the current 開心Pool balance. Active 定期 SHALL count at principal only (the sheet's `定期!B1 = sum(input)`), not principal + expected interest — deposit interest enters the totals via 活期 when the deposit ends. The IBKR cash positions (`ibkr.hkd_cash`/`ibkr.usd_cash` in `app_meta`) SHALL be included because the sheet's `Overview!B9` counts the whole `美股!B7` figure. Components needing a rate SHALL be absent when no `aia.usd_hkd_rate` is stored, and the totals SHALL omit them. Creating a month row SHALL snapshot the live values into the row (the sheet's paste-values step); patching `recapture` SHALL re-snapshot, and clearing the fields SHALL return the row to live derivation.

#### Scenario: New row captures the live totals

- **WHEN** the user creates month `2026-10-01` at payday
- **THEN** its `total_assets`/`liquid_assets` store the live values at that moment and no longer move with prices

#### Scenario: Live row before capture

- **WHEN** a month row has no stored totals
- **THEN** its `total_assets`/`liquid_assets` reflect the current portfolio on every read

#### Scenario: Deposits count at principal

- **WHEN** active deposits hold principal `445000` with expected interest `2689.93` and no other components
- **THEN** live `total_assets` includes `445000` for the deposits, not `447689.93`

#### Scenario: IBKR cash included

- **WHEN** `ibkr.usd_cash` is `1200.25`, `ibkr.hkd_cash` is `765.42`, and the rate is `7.845135`
- **THEN** both live totals include approximately `10183.79` for the IBKR cash on top of the US stock value
