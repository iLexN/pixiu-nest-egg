# app-settings Specification

## Purpose

The dedicated 設定 page: a single home for the app's parameter settings and the 活期帳戶/資產 balance editors, organized into independently editable sections under a top-level 設定 nav group.

## Requirements

### Requirement: 設定 page

The 設定 view SHALL fetch `GET /api/settings`, `GET /api/manual-assets`, and `GET /api/cash-accounts`, and render a 活期帳戶 table, a 資產 table, and the parameter sections 一般, 開心Pool, Money Master, 目標, and 日曆 — always visible, not behind a collapse toggle.

#### Scenario: Page renders all sections

- **WHEN** the user opens the 設定 tab
- **THEN** the 活期帳戶 and 資產 tables and the 一般, 開心Pool, Money Master, 目標, and 日曆 sections are shown with their stored values

### Requirement: Per-section settings editing

Each parameter section SHALL show a read-only summary line with an 編輯 action; 編輯 SHALL open a form containing only that section's fields, and saving SHALL `PATCH /api/settings` with only that section's fields, leaving other settings untouched.

#### Scenario: Saving one section leaves others untouched

- **WHEN** the user edits 薪金 in the 一般 form and saves
- **THEN** `PATCH /api/settings` carries only the 一般 fields, and stored Money Master, 目標, and 日曆 values are unchanged

#### Scenario: Blank field keeps its reset semantics

- **WHEN** the user blanks 預測繳費 in the 一般 form and saves
- **THEN** `bill_amount` is patched to `null` and the forecast returns to the 2158 default

### Requirement: 一般 settings section

The 一般 section SHALL edit 薪金 (`salary`), 預測繳費 (`bill_amount`), and 半流動資金目標 (`semi_liquid_target`). Target SHALL be a percent in 0–100 (exclusive); 繳費 SHALL be non-negative.

#### Scenario: Out-of-range target rejected

- **WHEN** the user enters a target of `105`
- **THEN** saving is rejected with a field error and nothing is stored

### Requirement: 開心Pool rates table

`GET /api/settings` SHALL return `pool_rates`, the stored per-year 開心Pool rates. The 開心Pool section SHALL show one row per stored year （年份, 利率） with 編輯 and 刪除 row actions, a 新增 action, and the current year's effective rate. Saving a row SHALL `PATCH /api/settings` with `pool_rate` and `pool_rate_year`; deleting SHALL patch `pool_rate` `null` for that year, restoring the earlier-year fallback.

#### Scenario: Past years listed

- **WHEN** `pool_rates` stores rates for 2024 and 2026
- **THEN** the table shows both years' rates alongside the current year's effective rate

#### Scenario: Editing a past year

- **WHEN** the user edits the 2024 row to `30` and saves
- **THEN** `PATCH /api/settings` carries `pool_rate` `0.30` and `pool_rate_year` `2024`

#### Scenario: Deleting a year restores the fallback

- **WHEN** the user deletes the 2026 row while 2024 stores `0.30`
- **THEN** 2026's rate falls back to `0.30`

### Requirement: Money Master section

The Money Master section SHALL edit the six `money_master_*` fields: 挑戰開始日, 已儲, 目標月數, 目標金額, 月份覆寫, and 每月需儲覆寫. 月數 fields SHALL be positive integers and 目標金額 positive; blank clears as the API defines.

#### Scenario: Section saves only Money Master fields

- **WHEN** the user saves 已儲 `1094405.06` in the Money Master form
- **THEN** `PATCH /api/settings` carries `money_master_saved` `1094405.06` and no other settings fields

### Requirement: 目標 section

The 目標 section SHALL edit 1st/2nd 目標金額 (`goal1_amount`, `goal2_amount`), 3rd 通脹倍數 (`goal3_inflation`), 3rd 年數 (`goal3_years`), and Final 薪金替代率 (`final_replacement`). Amounts SHALL be non-negative and constants positive; blank clears an amount or resets a constant to its default.

#### Scenario: Clearing a milestone

- **WHEN** the user blanks 1st 目標金額 and saves
- **THEN** `goal1_amount` is patched to `null` and the overview's 1st milestone is blank

### Requirement: 日曆 section

The 日曆 section SHALL edit the Private 日曆 ID (`calendar_private_id`). Saving a blank field SHALL clear the stored ID.

#### Scenario: Clearing the calendar ID

- **WHEN** the user blanks the Private 日曆 ID field and saves
- **THEN** `calendar_private_id` is patched to `null`

### Requirement: 活期帳戶 editor

The 活期帳戶 table SHALL list each account's 名稱, 幣別, 類型, 金額, and 預設收款 flag, with 編輯 and 刪除 row actions and a 新增帳戶 action. The editor SHALL offer 名稱, 幣別 (HKD/USD, USD disabled for 銀行), 類型 (銀行/券商), 金額, and 預設收款. 券商 rows SHALL NOT offer 刪除.

#### Scenario: Adding a bank account

- **WHEN** the user saves a new account 名稱 `渣打`, 類型 銀行, 金額 `1364.89`
- **THEN** the table lists it and live totals include the amount

#### Scenario: Broker row not deletable

- **WHEN** an account's 類型 is 券商
- **THEN** its row offers no 刪除 action

### Requirement: 資產 editor

The 資產 table SHALL list each row's 名稱 (with 備註 beneath it), 流動性, and 金額, with 編輯 and 刪除 row actions and a 新增結餘 action. The editor SHALL offer 名稱, 流動性 (短期/長期), 金額, and 備註.

#### Scenario: Note shown under the name

- **WHEN** the user saves the HS人壽 row with a 備註
- **THEN** the 資產 table shows the note beneath HS人壽
