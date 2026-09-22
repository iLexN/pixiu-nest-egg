# Spec Delta

## MODIFIED Requirements

### Requirement: Deposit registry

The system SHALL maintain a list of deposit records, each carrying an optional `label` (the sheet's `id` column, e.g. `SC-9632`), an optional `bank` code (e.g. `SC` for 渣打, `HS` for 恒生), optional `principal` (the sheet's `input`), optional `rate` (annual rate as a fraction, e.g. `0.03`), optional `interest` (利息), an optional `start_date` (`YYYY-MM-DD` — when the principal left the bank account; the workbook has no such column, so imported rows carry it only when `note2` yields one), a required `end_date` (`YYYY-MM-DD`), and two optional free-text notes (`note1`, `note2`). The system SHALL support creating, listing, editing, and deleting deposit records, and SHALL preserve each record's position via a stable `sort_order` initially taken from the workbook row order.

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

- **WHEN** the user deletes a deposit
- **THEN** it no longer appears in the list or in any aggregate

### Requirement: Deposit field validation

The system SHALL validate deposit input before storing it: `end_date` MUST be a calendar date in `YYYY-MM-DD` form; `start_date` MUST be a calendar date in `YYYY-MM-DD` form when present; `principal`, `rate`, and `interest` MUST NOT be negative when present; `rate` MUST be less than 1; and at least one of `label`, `principal`, or `interest` MUST be present.

#### Scenario: Malformed end date

- **WHEN** the user submits a deposit with end_date `2026/13/45`
- **THEN** the system rejects it with a validation error naming the date field, and stores nothing

#### Scenario: Malformed start date

- **WHEN** the user submits a deposit with start_date `12/10/2026`
- **THEN** the system rejects it with a validation error naming the date field, and stores nothing

#### Scenario: Empty record

- **WHEN** the user submits a deposit with no label, no principal, and no interest
- **THEN** the system rejects it with a validation error, and stores nothing

#### Scenario: Negative amount

- **WHEN** the user submits a deposit with principal `-5000`
- **THEN** the system rejects it with a validation error naming the principal field

### Requirement: Date entry in YYYY-MM-DD

Every `end_date` and `start_date` entry field in the deposit UI — the 新增定期 form and inline deposit editing — SHALL present and accept dates in `YYYY-MM-DD` form regardless of the OS or browser locale, and SHALL offer a calendar picker for choosing a date. An entry that is not in `YYYY-MM-DD` form SHALL be flagged or rejected before submission; the existing server-side validation remains the final check. The `start_date` field SHALL default to today when creating a deposit.

#### Scenario: Entry field shows ISO format

- **WHEN** the user opens the 新增定期 form or edits a deposit with end_date 2026-10-12
- **THEN** the end date field displays `2026-10-12` in year-month-day order, not the OS locale order

#### Scenario: Picking a date from the calendar

- **WHEN** the user chooses 2026-10-12 from the calendar picker
- **THEN** the field shows `2026-10-12` and that value is submitted

#### Scenario: Typing a non-ISO date

- **WHEN** the user types `12/10/2026` into the end date field and submits
- **THEN** the input is flagged or rejected before or at submission and no deposit is stored with a misread date
