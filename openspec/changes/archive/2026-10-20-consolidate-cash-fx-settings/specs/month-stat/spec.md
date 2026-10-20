# Spec Delta

## MODIFIED Requirements

### Requirement: Manual balances and salary

The system SHALL store the 活期 balances behind 月初 (e.g. HS, 渣打) as `bank` cash accounts (see the cash-accounts capability).

It SHALL store the remaining manual balances (e.g. Irene, HS人壽) as `asset` rows, exposed through `GET/POST /api/manual-assets` and `PATCH/DELETE /api/manual-assets/:id`. Each asset row carries `label`, `amount`, `sort_order`, `updated_at`, and a `liquidity` of `short` or `long` (default `long`). The Overview 策略 block uses `liquidity` to place the row in 短期可取回 or 長期可取回. `POST` SHALL accept `liquidity` optionally and `PATCH` SHALL accept it as a field to change; any value other than `short`/`long` is rejected.

The system SHALL also store a current salary as the setting `ledger.salary`, exposed through `GET/PATCH /api/settings`. Month rows SHALL snapshot the salary at creation, so past months keep their era's figure. The 資產 editor SHALL offer a 流動性 choice (短期 / 長期) for asset rows.

#### Scenario: Update cash balances

- **WHEN** the user saves HS `30538.78` and 渣打 `1364.89`
- **THEN** later live totals use 活期 `31903.67`

#### Scenario: Salary snapshot per row

- **WHEN** the salary setting is `52700` and a new month row is created, then the setting is raised to `55000`
- **THEN** the existing row still reports `52700` while the next created row reports `55000`

#### Scenario: Liquidity defaults to long

- **WHEN** the user creates an `asset` row without specifying `liquidity`
- **THEN** the stored row reports `liquidity` `long`

#### Scenario: Liquidity reclassified

- **WHEN** the user patches an `asset` row's `liquidity` to `short`
- **THEN** the row reports `short` and the next `GET /api/overview` counts its amount in `short_term` instead of `long_term`

#### Scenario: Invalid liquidity rejected

- **WHEN** a request carries `liquidity` `medium`
- **THEN** the request is rejected with a validation error and nothing is stored

### Requirement: Frozen and live asset totals

Each month SHALL carry `total_assets` (總數, the sheet's `Overview!B1`) and `liquid_assets` (流動資產, `Overview!H1`). A stored value SHALL win. When absent, each figure SHALL be derived live:

- `total_assets` = HK market value + (US market value + IBKR USD cash) × rate + IBKR HKD cash + active 定期 principal + active 債券 principal + AIA value × rate + MPF balance + Σ manual `asset` balances + Σ `bank` cash account balances
- `liquid_assets` = HK market value + active 定期 principal + Σ `bank` cash account balances + active 債券 principal + (US market value + IBKR USD cash) × rate + IBKR HKD cash − the current 開心Pool balance

Active 定期 SHALL count at principal only (the sheet's `定期!B1 = sum(input)`), not principal + expected interest. Deposit interest enters the totals via 活期 when the deposit ends. The IBKR cash positions (the amounts of the HKD and USD `broker` cash accounts) SHALL be included, because the sheet's `Overview!B9` counts the whole `美股!B7` figure. Components needing a rate SHALL be absent when no USD fx rate is stored, and the totals SHALL omit them.

A create patch that omits the totals SHALL snapshot the live values into the row (the sheet's paste-values step). An explicit `null` creates a live row. The 新增月份 form SHALL create rows live, and 重新擷取 is the freeze step. Patching `recapture` SHALL re-snapshot the totals AND `start_cash` (月初 = the live 活期 sum, Σ `bank` cash account balances). Clearing the fields SHALL return the totals to live derivation.

#### Scenario: 新增月份 creates a live row

- **WHEN** the user adds month `2026-10` from the 新增月份 form
- **THEN** the row is created with `total_assets`/`liquid_assets` NULL — they reflect the current portfolio on every read and are marked 即時 until 重新擷取 freezes them

#### Scenario: New row captures the live totals

- **WHEN** a create patch for month `2026-10-01` omits `total_assets`/`liquid_assets`
- **THEN** the row stores the live values at that moment and no longer moves with prices

#### Scenario: Recapture also fills 月初

- **WHEN** bank cash accounts total `120000` and a stored month has no `start_cash`
- **THEN** patching `recapture: true` stores `start_cash` `120000` alongside the re-snapshotted totals — and an explicit `start_cash` in the same patch wins over the snapshot

#### Scenario: Live row before capture

- **WHEN** a month row has no stored totals
- **THEN** its `total_assets`/`liquid_assets` reflect the current portfolio on every read

#### Scenario: Deposits count at principal

- **WHEN** active deposits hold principal `445000` with expected interest `2689.93` and no other components
- **THEN** live `total_assets` includes `445000` for the deposits, not `447689.93`

#### Scenario: IBKR cash included

- **WHEN** the USD broker account holds `1200.25`, the HKD broker account holds `765.42`, and the rate is `7.845135`
- **THEN** both live totals include approximately `10183.79` for the IBKR cash on top of the US stock value

### Requirement: 開心Pool balance

The system SHALL keep a per-year 娛樂 rate: one stored rate per year, editable through `PATCH /api/settings` as `pool_rate` with an optional `pool_rate_year` (default the current year). A year with no stored rate falls back to the latest earlier year's rate.

Each year's pool income SHALL derive as Σ `interest` × that year's rate. The pool balance at a month/year boundary SHALL derive as the previous year's closing balance + pool income − Σ `entertainment` items + Σ `pool_input`, matching the sheet's `M` column chain.

#### Scenario: Rate applies to the whole year

- **WHEN** the 2026 rate is set to `0.337` mid-year
- **THEN** all of 2026's pool income reprices to `0.337` without affecting other years

#### Scenario: Balance rolls forward

- **WHEN** 2025 closed at `5717.57` and 2026 shows interest `62033.15`, entertainment items totalling `8593`, pool inputs `4142.66` at rate `0.337`
- **THEN** the current pool balance is approximately `22172.4`

#### Scenario: Earlier year's rate carries forward

- **WHEN** rates are stored for 2025 (`0.425`) but not 2026
- **THEN** 2026's pool income uses `0.425`

### Requirement: Money Master settings

`PATCH /api/settings` SHALL also accept the Money Master challenge fields, stored as `money_master.*` keys in `app_meta`; `GET /api/settings` SHALL return them. The fields and their rules:

- `start_date`: a valid `YYYY-MM-DD` date, or `null`
- `target_months` and `month_now`: finite positive integers (`month_now` is an optional override)
- `target_amount`: finite and positive
- `saved` and `coming_save`: finite, negative allowed, because the bank's figure goes negative while ahead of target (`coming_save` is an optional override)

`null` clears a stored value; clearing `month_now` or `coming_save` returns that figure to its derivation. These figures are copied from the bank app, which holds the challenge's own data, so they are user inputs, not derivations of tracked data.

#### Scenario: Settings updated

- **WHEN** `saved` is patched to `1094405.06` and `target_amount` to `1000000`
- **THEN** the settings response reports both and the overview block's `saved_progress`/`coming_save`/`can_use` follow

#### Scenario: Invalid field rejected

- **WHEN** `target_months` is patched to `0` or `start_date` to `not-a-date`
- **THEN** the request fails with 400 and field errors on the offending fields

#### Scenario: Override cleared restores derivation

- **WHEN** `month_now` is patched to `null` while `start_date` is `2023-10-27`
- **THEN** the overview block derives `month_now` from `start_date` again

#### Scenario: New challenge reconfigured

- **WHEN** the user patches `start_date`, `saved`, and the targets for a new challenge with no overrides stored
- **THEN** `month_now` restarts from `1`-indexed derivation and `coming_save` re-derives against the new targets
