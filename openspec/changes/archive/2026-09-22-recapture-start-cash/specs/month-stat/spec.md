# Spec Delta

## MODIFIED Requirements

### Requirement: Monthly ledger rows

The system SHALL store one row per calendar month (`YYYY-MM-01`) carrying `start_cash` (月初出糧後 — the 活期 bank total measured right after salary lands, entered manually in the month editor or captured from the live 活期 sum by 重新擷取), `salary` (the salary in effect that month, snapshotting the current salary setting when the row is created), `pool_input` (Irene + 開心 Pool contributions), and `note`. The add-month form SHALL NOT collect 月初 — a created row stores no `start_cash` unless the patch supplies one. 利息 is NOT stored on the row — it is derived from auto events plus the month's `interest` items. 娛樂支出 is NOT stored on the row — it is the sum of the month's `entertainment` items. The API SHALL expose `GET /api/months` (optionally filtered by `year`), `GET /api/months/:ym`, `PATCH /api/months/:ym` (creating the row when absent), and `DELETE /api/months/:ym`.

#### Scenario: Create a month row at payday

- **WHEN** the user patches `2026-10-01` with `start_cash` `35000`
- **THEN** the row is stored with `salary` defaulting to the current salary setting

#### Scenario: Adding a month stores no 月初

- **WHEN** the user adds month `2026-10` from the 新增月份 form (which has no 月初 input)
- **THEN** the row is created with `start_cash` NULL — the column shows `—` until 重新擷取 or a manual edit fills it

#### Scenario: Update figures

- **WHEN** the user patches a stored month with `pool_input` `1500`
- **THEN** the row reports that value and derived figures recompute on the next read

### Requirement: Frozen and live asset totals

Each month SHALL carry `total_assets` (總數, the sheet's `Overview!B1`) and `liquid_assets` (流動資產, `Overview!H1`). A stored value SHALL win; when absent the figure SHALL be derived live as: `total_assets` = HK market value + (US market value + IBKR USD cash) × rate + IBKR HKD cash + active 定期 principal + active 債券 principal + AIA value × rate + MPF balance + Σ manual `asset` balances + Σ `cash` balances, and `liquid_assets` = HK market value + active 定期 principal + cash + active 債券 principal + (US market value + IBKR USD cash) × rate + IBKR HKD cash − the current 開心Pool balance. Active 定期 SHALL count at principal only (the sheet's `定期!B1 = sum(input)`), not principal + expected interest — deposit interest enters the totals via 活期 when the deposit ends. The IBKR cash positions (`ibkr.hkd_cash`/`ibkr.usd_cash` in `app_meta`) SHALL be included because the sheet's `Overview!B9` counts the whole `美股!B7` figure. Components needing a rate SHALL be absent when no `aia.usd_hkd_rate` is stored, and the totals SHALL omit them. Creating a month row SHALL snapshot the live values into the row (the sheet's paste-values step); patching `recapture` SHALL re-snapshot the totals AND `start_cash` (月初 = the live 活期 sum, Σ `cash` manual assets), and clearing the fields SHALL return the totals to live derivation.

#### Scenario: New row captures the live totals

- **WHEN** the user creates month `2026-10-01` at payday
- **THEN** its `total_assets`/`liquid_assets` store the live values at that moment and no longer move with prices

#### Scenario: Recapture also fills 月初

- **WHEN** cash manual assets total `120000` and a stored month has no `start_cash`
- **THEN** patching `recapture: true` stores `start_cash` `120000` alongside the re-snapshotted totals — and an explicit `start_cash` in the same patch wins over the snapshot

#### Scenario: Live row before capture

- **WHEN** a month row has no stored totals
- **THEN** its `total_assets`/`liquid_assets` reflect the current portfolio on every read

#### Scenario: Deposits count at principal

- **WHEN** active deposits hold principal `445000` with expected interest `2689.93` and no other components
- **THEN** live `total_assets` includes `445000` for the deposits, not `447689.93`
