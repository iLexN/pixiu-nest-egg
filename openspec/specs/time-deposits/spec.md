# Time Deposits Specification

## Purpose

Tracks time deposits (定期) — principal, rate, interest, and maturity — replacing the `定期`/`定期Info` sheets so deposits can be recorded once and the app computes the active totals, maturity calendar, and bank rollups the spreadsheet produces today.

## Requirements

### Requirement: Deposit registry
The system SHALL maintain a list of deposit records, each carrying an optional `label` (the sheet's `id` column, e.g. `SC-9632`), an optional `bank` code (e.g. `SC` for 渣打, `HS` for 恒生), optional `principal` (the sheet's `input`), optional `rate` (annual rate as a fraction, e.g. `0.03`), optional `interest` (利息), a required `end_date` (`YYYY-MM-DD`), and two optional free-text notes (`note1`, `note2`). The system SHALL support creating, listing, editing, and deleting deposit records, and SHALL preserve each record's position via a stable `sort_order` initially taken from the workbook row order.

#### Scenario: Recording a deposit
- **WHEN** the user records a deposit with label `SC-9632`, principal 110000, rate 0.028, interest 993, and end_date `2026-10-12`
- **THEN** the deposit is stored and appears in the deposit list

#### Scenario: Interest-only row
- **WHEN** the user records a deposit with no label and no principal, interest 539.25, and end_date `2026-05-14`
- **THEN** the deposit is stored, matching the interest-only entries the workbook list contains

#### Scenario: Editing a deposit
- **WHEN** the user changes a deposit's end_date or interest
- **THEN** the stored record is updated and every aggregate view reflects the change on next read

#### Scenario: Deleting a deposit
- **WHEN** the user deletes a deposit
- **THEN** it no longer appears in the list or in any aggregate

### Requirement: Deposit field validation
The system SHALL validate deposit input before storing it: `end_date` MUST be a calendar date in `YYYY-MM-DD` form; `principal`, `rate`, and `interest` MUST NOT be negative when present; `rate` MUST be less than 1; and at least one of `label`, `principal`, or `interest` MUST be present.

#### Scenario: Malformed end date
- **WHEN** the user submits a deposit with end_date `2026/13/45`
- **THEN** the system rejects it with a validation error naming the date field, and stores nothing

#### Scenario: Empty record
- **WHEN** the user submits a deposit with no label, no principal, and no interest
- **THEN** the system rejects it with a validation error, and stores nothing

#### Scenario: Negative amount
- **WHEN** the user submits a deposit with principal `-5000`
- **THEN** the system rejects it with a validation error naming the principal field

### Requirement: Derived deposit fields
The system SHALL compute, on read and never store: `total = COALESCE(principal, 0) + COALESCE(interest, 0)`; `status` = `End` when `end_date` is today or earlier, otherwise active (the same rule as the sheet's `IF(TODAY() >= end date, "End", "")`); and `end_month`/`end_year` taken from `end_date`.

#### Scenario: Future end date is active
- **WHEN** a deposit's end_date is later than today
- **THEN** its status is active and it appears in the Upcoming section

#### Scenario: End date reached
- **WHEN** a deposit's end_date is today or in the past
- **THEN** its status is `End` and it leaves the Upcoming section without any manual flag

#### Scenario: Total from partial fields
- **WHEN** a deposit has interest 539.25 and no principal
- **THEN** its derived total is 539.25

### Requirement: Upcoming deposit view
The system SHALL present the active deposits (end date in the future) ordered by end date, each showing label, principal, rate, interest, total, end_date, and end_month. The view SHALL also present: the total active principal (Σ principal of active deposits); a month rollup grouping active deposits by end year and month with Σ total, Σ interest, and Σ principal; and a bank rollup grouping active deposits by `bank` with the same three sums. Deposits without a bank SHALL be grouped under their own bucket rather than matched to a bank.

#### Scenario: Active principal total
- **WHEN** the imported workbook data is live
- **THEN** the total active principal equals the sum of principal over deposits whose end date is in the future (445,000 for the current workbook)

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
The deposits view SHALL display the workbook's "定期 start step" and "定期 end step" checklists as static reminder text, since the steps they describe (Month Stat, money master, 回報率, Overview 預測) still require manual workbook edits until those sections are migrated.

#### Scenario: Checklist visible
- **WHEN** the user opens the deposits view
- **THEN** the start-step and end-step reminders are visible without editing capability

### Requirement: Display formatting
Deposit display SHALL follow the app's conventions: money to 2 decimal places and rates shown as percentages; stored values are never rounded.

#### Scenario: Rate display
- **WHEN** a deposit stores rate `0.028162004662004664`
- **THEN** the view displays it as a percentage such as `2.82%` while the stored value is unchanged

### Requirement: Date entry in YYYY-MM-DD
Every `end_date` entry field in the deposit UI — the 新增定期 form and inline deposit editing — SHALL present and accept dates in `YYYY-MM-DD` form regardless of the OS or browser locale, and SHALL offer a calendar picker for choosing a date. An entry that is not in `YYYY-MM-DD` form SHALL be flagged or rejected before submission; the existing server-side validation remains the final check.

#### Scenario: Entry field shows ISO format
- **WHEN** the user opens the 新增定期 form or edits a deposit with end_date 2026-10-12
- **THEN** the end date field displays `2026-10-12` in year-month-day order, not the OS locale order

#### Scenario: Picking a date from the calendar
- **WHEN** the user chooses 2026-10-12 from the calendar picker
- **THEN** the field shows `2026-10-12` and that value is submitted

#### Scenario: Typing a non-ISO date
- **WHEN** the user types `12/10/2026` into the end date field and submits
- **THEN** the input is flagged or rejected before or at submission and no deposit is stored with a misread date
