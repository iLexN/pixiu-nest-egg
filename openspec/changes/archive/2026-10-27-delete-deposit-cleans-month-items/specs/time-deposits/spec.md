# Spec Delta

## MODIFIED Requirements

### Requirement: Deposit registry

The system SHALL maintain a list of deposit records, each carrying an optional `label` (the sheet's `id` column, e.g. `SC-9632`), an optional `bank` code (e.g. `SC` for 渣打, `HS` for 恒生), optional `principal` (the sheet's `input`), optional `rate` (annual rate as a fraction, e.g. `0.03`), and optional `interest` (利息).

#### Scenario: Recording a deposit

- **WHEN** the user records a deposit with label `SC-9632`, principal 110000, rate 0.028, interest 993, and end_date `2026-10-12`
- **THEN** the deposit is stored and appears in the deposit list

#### Scenario: Interest-only row

- **WHEN** the user records a deposit with no label and no principal, interest 539.25, and end_date `2026-05-14`
- **THEN** the deposit is stored, matching the interest-only entries the workbook list contains

#### Scenario: Start date is optional

- **WHEN** the user records a deposit with no `start_date`
- **THEN** the deposit is stored with an empty start date

#### Scenario: Editing a deposit

- **WHEN** the user changes a deposit's end_date or interest
- **THEN** the stored record is updated and every aggregate view reflects the change on next read

#### Scenario: Deleting a deposit

- **WHEN** the user deletes an unreceived deposit
- **THEN** it no longer appears in the list, in any aggregate, in 預測, or in any month's 調整 items

## ADDED Requirements

### Requirement: Deposit deletion

`DELETE /api/deposits/{id}` SHALL delete an unreceived deposit together with every `dep-start:<id>` month item and every `dep-start:<id>` dismissal, in any month, in one write. A received (收訖) deposit SHALL NOT be deletable: the request returns 409 and changes nothing. The UI SHALL NOT offer 刪除 for a received deposit.

#### Scenario: Delete removes the dep-start item

- **WHEN** a deposit created with principal `140000` and `start_date` `2026-10-13` holds a `dep-start:<id>` item of `-140000` in `2026-10`, and the user deletes the deposit
- **THEN** the deposit and its `dep-start:<id>` item are both gone, and the `2026-10` month's other items are unchanged

#### Scenario: Item removed even after the start date moved

- **WHEN** the deposit's `start_date` was edited to another month after its `dep-start:<id>` item was recorded, and the user deletes the deposit
- **THEN** the originally recorded `dep-start:<id>` item is removed too

#### Scenario: Received deposit cannot be deleted

- **WHEN** the user deletes a deposit that has been 收訖
- **THEN** the request returns 409, and the deposit, its `dep-end:<id>` item, and its cash credit are unchanged

#### Scenario: Unknown deposit

- **WHEN** the user deletes a deposit id that does not exist
- **THEN** the request returns 404
