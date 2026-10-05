# Spec Delta

## MODIFIED Requirements

### Requirement: Frozen and live asset totals

Each month SHALL carry `total_assets` (總數, the sheet's `Overview!B1`) and `liquid_assets` (流動資產, `Overview!H1`). A stored value SHALL win; when absent the figure SHALL be derived live as: `total_assets` = HK market value + (US market value + IBKR USD cash) × rate + IBKR HKD cash + active 定期 principal + active 債券 principal + AIA value × rate + MPF balance + Σ manual `asset` balances + Σ `cash` balances, and `liquid_assets` = HK market value + active 定期 principal + cash + active 債券 principal + (US market value + IBKR USD cash) × rate + IBKR HKD cash − the current 開心Pool balance. Active 定期 SHALL count at principal only (the sheet's `定期!B1 = sum(input)`), not principal + expected interest — deposit interest enters the totals via 活期 when the deposit ends. The IBKR cash positions (`ibkr.hkd_cash`/`ibkr.usd_cash` in `app_meta`) SHALL be included because the sheet's `Overview!B9` counts the whole `美股!B7` figure. Components needing a rate SHALL be absent when no `aia.usd_hkd_rate` is stored, and the totals SHALL omit them. A create patch that omits the totals SHALL snapshot the live values into the row (the sheet's paste-values step), while explicit `null` creates a live row — the 新增月份 form SHALL create rows live, and 重新擷取 is the freeze step; patching `recapture` SHALL re-snapshot the totals AND `start_cash` (月初 = the live 活期 sum, Σ `cash` manual assets), and clearing the fields SHALL return the totals to live derivation.

#### Scenario: 新增月份 creates a live row

- **WHEN** the user adds month `2026-10` from the 新增月份 form
- **THEN** the row is created with `total_assets`/`liquid_assets` NULL — they reflect the current portfolio on every read and are marked 即時 until 重新擷取 freezes them

#### Scenario: New row captures the live totals

- **WHEN** a create patch for month `2026-10-01` omits `total_assets`/`liquid_assets`
- **THEN** the row stores the live values at that moment and no longer moves with prices

#### Scenario: Recapture also fills 月初

- **WHEN** cash manual assets total `120000` and a stored month has no `start_cash`
- **THEN** patching `recapture: true` stores `start_cash` `120000` alongside the re-snapshotted totals — and an explicit `start_cash` in the same patch wins over the snapshot

#### Scenario: Live row before capture

- **WHEN** a month row has no stored totals
- **THEN** its `total_assets`/`liquid_assets` reflect the current portfolio on every read

#### Scenario: Deposits count at principal

- **WHEN** active deposits hold principal `445000` with expected interest `2689.93` and no other components
- **THEN** live `total_assets` includes `445000` for the deposits, not `447689.93`

#### Scenario: IBKR cash included

- **WHEN** `ibkr.usd_cash` is `1200.25`, `ibkr.hkd_cash` is `765.42`, and the rate is `7.845135`
- **THEN** both live totals include approximately `10183.79` for the IBKR cash on top of the US stock value
