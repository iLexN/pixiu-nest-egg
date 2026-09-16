## ADDED Requirements

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
