# Spec Delta

## ADDED Requirements

### Requirement: Deposit placement debit

Creating a deposit SHALL accept an optional `debit_account_id`. When given, the principal is subtracted from that account in the same write as the deposit, and the deposit stores the account and amount it debited. When absent, or when the deposit has no positive principal, nothing is debited and both fields stay empty. The account MUST be an HKD bank account holding at least the principal, else 400 on `debit_account_id` and nothing is stored.

#### Scenario: Placement debits the chosen account

- **WHEN** the user creates a deposit with principal `130000` and `debit_account_id` naming HS, which holds `195608.15`
- **THEN** the deposit is stored with debited account HS and debited amount `130000`, and HS holds `65608.15`

#### Scenario: 不扣活期

- **WHEN** the user creates a deposit with no `debit_account_id`
- **THEN** no cash account changes and the deposit's debited account and amount are empty

#### Scenario: Insufficient balance

- **WHEN** the user creates a deposit with principal `130000` and `debit_account_id` naming an account holding `50000`
- **THEN** the request returns 400 naming `debit_account_id`, and no deposit, month item, or balance change is stored

#### Scenario: Not an HKD bank account

- **WHEN** `debit_account_id` names a broker account or a USD account
- **THEN** the request returns 400 naming `debit_account_id` and nothing is stored

#### Scenario: Already-received deposit never debits

- **WHEN** a deposit is created with `end_date` already past, so it is stored received, and `debit_account_id` is given
- **THEN** no cash account changes and the deposit's debited account and amount are empty

#### Scenario: Editing leaves the debit alone

- **WHEN** a deposit that debited HS `130000` is edited to principal `120000`
- **THEN** HS is unchanged and the stored debited amount stays `130000`

### Requirement: Placement account choice in the form

The 新增定期 and 轉為定期 forms SHALL offer a 扣自 choice listing the HKD bank accounts plus 不扣活期. It is preselected from the 銀行 field: `HS` → the HS account, `SC` → the 渣打 account, otherwise the HKD default account, or 不扣活期 when none exists. The edit form SHALL NOT offer it.

#### Scenario: Preselected from the bank

- **WHEN** the user opens 轉為定期 on an `hs_deposit` plan
- **THEN** 銀行 shows `HS`, 扣自 preselects the HS account, and saving debits HS by the principal

#### Scenario: Choosing 不扣活期

- **WHEN** the user picks 不扣活期 and saves
- **THEN** the deposit is created without a debit

## MODIFIED Requirements

### Requirement: Deposit creation month item

`POST /api/deposits` SHALL record the `dep-start:<id>` `adjustment` item of −`principal` in the `start_date` month, in the same write as the deposit itself — as the dep-start suggestion would have. The item is skipped when the deposit carries no `start_date` or when the start month has no month row; a skipped item still surfaces as the month's `dep-start` suggestion once the month exists.

#### Scenario: Create records the start month item

- **WHEN** the user creates a deposit with principal `140000` and `start_date` `2026-10-13`, and the `2026-10` month row exists
- **THEN** the deposit is stored and the `2026-10` month holds a `dep-start:<id>` adjustment item of `-140000`

#### Scenario: No start_date, no item

- **WHEN** the user creates a deposit with no `start_date`
- **THEN** no `dep-start` item is recorded for it

#### Scenario: dep-start item skipped when already stored or monthless

- **WHEN** the start month has no month row
- **THEN** the deposit is stored but no item is recorded, and the `dep-start:<id>` suggestion still appears for that month

#### Scenario: Debit and item in one write

- **WHEN** the user creates a deposit with principal `140000`, `start_date` `2026-10-13`, and `debit_account_id` naming HS, and the `2026-10` month row exists
- **THEN** the deposit, the HS debit of `140000`, and the `dep-start:<id>` item of `-140000` are all stored together

### Requirement: Deposit deletion

`DELETE /api/deposits/{id}` SHALL delete an unreceived deposit, in one write, together with:

- every `dep-start:<id>` month item and every `dep-start:<id>` dismissal, in any month;
- the reversal of its placement debit: the stored debited amount is added back to the stored debited account, and skipped when that account no longer exists.

A received (收訖) deposit SHALL NOT be deletable: the request returns 409 and changes nothing. The UI SHALL NOT offer 刪除 for a received deposit.

#### Scenario: Delete removes the dep-start item

- **WHEN** a deposit created with principal `140000` and `start_date` `2026-10-13` holds a `dep-start:<id>` item of `-140000` in `2026-10`, and the user deletes the deposit
- **THEN** the deposit and its `dep-start:<id>` item are both gone, and the `2026-10` month's other items are unchanged

#### Scenario: Item removed even after the start date moved

- **WHEN** the deposit's `start_date` was edited to another month after its `dep-start:<id>` item was recorded, and the user deletes the deposit
- **THEN** the originally recorded `dep-start:<id>` item is removed too

#### Scenario: Delete reverses the debit

- **WHEN** a deposit that debited HS `130000` is deleted, and the HKD default has since moved to 渣打
- **THEN** HS gains `130000` and 渣打 is unchanged

#### Scenario: Debited account since deleted

- **WHEN** a deposit debited 渣打, 渣打 has since been deleted, and the user deletes the deposit
- **THEN** the deposit is deleted and no remaining account changes

#### Scenario: Received deposit cannot be deleted

- **WHEN** the user deletes a deposit that has been 收訖
- **THEN** the request returns 409, and the deposit, its `dep-end:<id>` item, and its cash credit are unchanged

#### Scenario: Unknown deposit

- **WHEN** the user deletes a deposit id that does not exist
- **THEN** the request returns 404

### Requirement: Reminder checklists
The deposits view SHALL display the workbook's "定期 start step" and "定期 end step" checklists as static reminder text. The start step SHALL state that 新增定期 / 轉為定期 automatically debit the chosen 活期 account and record the 月結 調整 −本金. The checklist still lists "money master" as a step — the user performs it in the app's settings — while 回報率 and Overview 預測 still require manual workbook edits until those sections are migrated.

#### Scenario: Checklist visible
- **WHEN** the user opens the deposits view
- **THEN** the start-step and end-step reminders are visible without editing capability, and the muted note no longer lists money master among the workbook-manual steps

#### Scenario: Start step names the automatic debit
- **WHEN** the user opens the deposits view
- **THEN** the start step's first line says placing a deposit automatically 扣活期 and records the 月結調整 −本金
