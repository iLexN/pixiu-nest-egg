## ADDED Requirements

### Requirement: Date entry in YYYY-MM-DD
Every 日期 entry field in the trade UI — the 新增交易 form, inline trade editing, and the trade-history 由/至 date-range filters — SHALL present and accept dates in `YYYY-MM-DD` form regardless of the OS or browser locale, and SHALL offer a calendar picker for choosing a date. An entry that is not in `YYYY-MM-DD` form SHALL be flagged or rejected before submission; the existing server-side validation remains the final check.

#### Scenario: Entry field shows ISO format
- **WHEN** the user opens the 新增交易 form or edits a trade dated 2026-09-16
- **THEN** the 日期 field displays `2026-09-16` in year-month-day order, not the OS locale order

#### Scenario: Picking a date from the calendar
- **WHEN** the user chooses 2026-09-16 from the calendar picker
- **THEN** the field shows `2026-09-16` and that value is submitted

#### Scenario: Typing a non-ISO date
- **WHEN** the user types `16/09/2026` into a 日期 field and submits
- **THEN** the input is flagged or rejected before or at submission and no trade is stored with a misread date

#### Scenario: Partial date-range filter
- **WHEN** the user has typed an incomplete value such as `2026-0` into a 由/至 filter
- **THEN** the filter is not applied until it holds a complete `YYYY-MM-DD` date
