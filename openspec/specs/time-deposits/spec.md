# Time Deposits Specification

## Purpose

Tracks time deposits (定期) — principal, rate, interest, and maturity — replacing the `定期`/`定期Info` sheets so deposits can be recorded once and the app computes the active totals, maturity calendar, and bank rollups the spreadsheet produces today.

## Requirements

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

### Requirement: Derived deposit fields
The system SHALL compute, on read and never store: `total = COALESCE(principal, 0) + COALESCE(interest, 0)`; `status` = `End` once 收訖 (`received_at` set), otherwise active — a deposit stays active past its `end_date` until the user confirms receipt; and `end_month`/`end_year` taken from `end_date`.

#### Scenario: Future end date is active
- **WHEN** a deposit's end_date is later than today
- **THEN** its status is active and it appears in the Upcoming section

#### Scenario: End date reached
- **WHEN** a deposit's end_date is today or in the past and it is not 收訖
- **THEN** it stays in the Upcoming section marked 已到期未收 until the user confirms receipt — the end date alone no longer ends it

#### Scenario: Total from partial fields
- **WHEN** a deposit has interest 539.25 and no principal
- **THEN** its derived total is 539.25

### Requirement: Upcoming deposit view
The system SHALL present the **unreceived** deposits ordered by end date (a matured-but-unreceived deposit is included and flagged 已到期未收), each showing label, principal, rate, interest, total, end_date, and end_month, with a 收訖 action per row. The totals and rollups remain end_date-based to match the sheet's formulas: the total active principal (Σ principal of deposits whose `end_date` is in the future); a month rollup grouping those deposits by end year and month with Σ total, Σ interest, and Σ principal; and a bank rollup by `bank` with the same three sums. Deposits without a bank SHALL be grouped under their own bucket rather than matched to a bank.

#### Scenario: Active principal total
- **WHEN** the imported workbook data is live
- **THEN** the total active principal equals the sum of principal over deposits whose end date is in the future (445,000 for the current workbook)

#### Scenario: Overdue deposit stays listed
- **WHEN** a deposit ended `2026-10-12` is not yet 收訖
- **THEN** it still appears in 未到期定期 with a 已到期未收 marker while the totals no longer count it

#### Scenario: Month rollup
- **WHEN** active deposits end in October 2026 and November 2026
- **THEN** the rollup lists those months with their Σ total, Σ interest, and Σ principal

#### Scenario: Bank rollup by bank
- **WHEN** active deposits carry banks `SC` (four deposits) and `HS` (one deposit)
- **THEN** the bank rollup reports an `SC` group (Σ principal 365,000) and an `HS` group (Σ principal 80,000)

### Requirement: History view by year
The system SHALL present a year filter defaulting to the current year, listing the deposits whose `end_date` falls in the selected year, and SHALL show a per-month aggregation for that year — for each month, Σ interest and Σ total over the deposits ending that month — equivalent to the workbook's per-year tables (表_2027定期, 表_2028定期) for every year present in the data.

#### Scenario: Default year
- **WHEN** the user opens the deposit history without choosing a year
- **THEN** the current year is selected and its deposits and month aggregation are shown

#### Scenario: Month aggregation covers the whole year
- **WHEN** deposits end in January 2027 and no other 2027 month
- **THEN** the 2027 table shows January with that month's Σ interest and Σ total, and the remaining months with zero

### Requirement: Reminder checklists
The deposits view SHALL display the workbook's "定期 start step" and "定期 end step" checklists as static reminder text; the checklist still lists "money master" as a step — the user performs it in the app's settings — while 回報率 and Overview 預測 still require manual workbook edits until those sections are migrated.

#### Scenario: Checklist visible
- **WHEN** the user opens the deposits view
- **THEN** the start-step and end-step reminders are visible without editing capability, and the muted note no longer lists money master among the workbook-manual steps

### Requirement: Display formatting
Deposit display SHALL follow the app's conventions: money to 2 decimal places and rates shown as percentages; stored values are never rounded.

#### Scenario: Rate display
- **WHEN** a deposit stores rate `0.028162004662004664`
- **THEN** the view displays it as a percentage such as `2.82%` while the stored value is unchanged

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

### Requirement: Deposit 收訖 lifecycle

Each deposit SHALL carry a `received_at` date (NULL while unreceived). The system SHALL expose `POST /api/deposits/:id/receive` — marking the deposit received at a given date (defaulting to today), optionally correcting the stored `interest` to the amount actually received, optionally crediting a chosen cash `manual_assets` row by an amount defaulting to `principal + interest`, and recording the month's `dep-end:<id>` `adjustment` item (skipped when already stored, when the month has no row, or after re-entry); receiving an already-received deposit SHALL return 409. The system SHALL expose `POST /api/deposits/:id/unreceive` — reversing the stored cash credit, deleting the `dep-end` item, and clearing the flag; unreceiving an unreceived deposit SHALL return 409. Import and create SHALL record `received_at = end_date` when the entered `end_date` is already past, matching the previously date-based behavior.

#### Scenario: One-click maturity

- **WHEN** the user 收訖's a deposit (principal `110000`, interest `993`, ending `2026-10-12`) choosing cash row `渣打`
- **THEN** the deposit is stored received, `渣打` gains `110993`, the `2026-10` month gains a `dep-end` adjustment item of `110993`, and the deposit leaves 未到期定期

#### Scenario: Receipt without a bank-in

- **WHEN** the user 收訖's a deposit with `不存入` selected
- **THEN** only `received_at` and the dep-end item are recorded — no cash row changes

#### Scenario: Undo restores the pending state

- **WHEN** the user 取消收訖's a deposit that had credited `渣打` `110993` and created a `dep-end` item
- **THEN** `渣打` loses `110993`, the item is deleted, and the deposit returns to 未到期定期
