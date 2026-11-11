# Spec Delta

## ADDED Requirements

### Requirement: Adding an upcoming family deposit to Google Calendar

Each row of the 未到期定期 table SHALL offer a 加到日曆 action in its row menu, listed before 編輯. Choosing it SHALL open Google Calendar's new-event editor in a new browser tab, pre-filled with an all-day event on the deposit's end_date. The app SHALL NOT store anything for this action; the user saves the event in Google Calendar. Rows of the year-filtered history table SHALL NOT offer the action.

#### Scenario: Opening the calendar editor

- **WHEN** the user chooses 加到日曆 on an upcoming family deposit with end_date `2027-02-10`
- **THEN** a new tab opens Google Calendar's new-event editor with an all-day event on 2027-02-10 (start `20270210`, exclusive end `20270211`), and no family deposit data changes

#### Scenario: Year-end maturity

- **WHEN** the user chooses 加到日曆 on an upcoming family deposit with end_date `2026-12-31`
- **THEN** the event is all-day on 2026-12-31 with exclusive end `20270101`

#### Scenario: Overdue deposits keep the action

- **WHEN** an unreceived family deposit is past its end_date and shows 已到期未收
- **THEN** its 未到期定期 row still offers 加到日曆

#### Scenario: History rows have no action

- **WHEN** the user opens the row menu of a deposit in the year-filtered history table
- **THEN** the menu does not offer 加到日曆

### Requirement: Family deposit calendar routing

A family deposit whose holder is exactly `Irene` SHALL open the editor on the user's default Google calendar. Every other family deposit SHALL pre-select the calendar named by the `calendar_private_id` setting (see the time-deposits Private calendar setting). While that setting is empty, every family deposit SHALL open on the default calendar.

#### Scenario: Irene goes to the default calendar

- **WHEN** `calendar_private_id` is `7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com` and the user adds an upcoming deposit of holder `Irene` to the calendar
- **THEN** the editor opens without a calendar pre-selection, on the default calendar

#### Scenario: Other holders go to the Private calendar

- **WHEN** `calendar_private_id` is `7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com` and the user adds an upcoming deposit of holder `媽媽` to the calendar
- **THEN** the editor opens with that calendar pre-selected (`src=7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com`)

#### Scenario: No Private calendar set

- **WHEN** `calendar_private_id` is unset and the user adds an upcoming deposit of holder `媽媽` to the calendar
- **THEN** the editor opens on the default calendar

### Requirement: Family deposit calendar event title

The event title SHALL be `定期: 持有人 - 編號 - 銀行`: the prefix `定期: `, then the holder as stored, the label, and the bank's name only (`恒生` for `HS`, `渣打` for `SC`, any other code as itself). Each empty part SHALL be dropped together with its ` - ` separator.

#### Scenario: Full title

- **WHEN** the user adds holder `Irene`, label `HS-0090`, bank `HS` to the calendar
- **THEN** the event title is `定期: Irene - HS-0090 - 恒生`

#### Scenario: Missing parts are dropped

- **WHEN** the user adds a deposit of holder `媽媽` with bank `SC` and no label to the calendar
- **THEN** the event title is `定期: 媽媽 - 渣打`

### Requirement: Family deposit calendar event details

The event details SHALL hold one `label: value` line per field, in this order: 持有人, 到期日, 編號, 銀行, 本金, 利息, 合計, 開始日. Values SHALL use the page's display formatting: money to 2 decimal places with thousands separators, bank as `name (code)`. A field with no value SHALL have no line. The note SHALL NOT be included.

#### Scenario: Full details

- **WHEN** the user adds holder `Irene`, label `HS-0090`, bank `HS`, principal `10000`, interest `28.11`, start_date `2026-11-10`, end_date `2027-02-10` to the calendar
- **THEN** the event details are exactly these lines:
  `持有人: Irene`
  `到期日: 2027-02-10`
  `編號: HS-0090`
  `銀行: 恒生 (HS)`
  `本金: 10,000.00`
  `利息: 28.11`
  `合計: 10,028.11`
  `開始日: 2026-11-10`

#### Scenario: Empty fields are left out

- **WHEN** the user adds a family deposit with no start_date and no bank to the calendar
- **THEN** the details have no `開始日:` line and no `銀行:` line, and the remaining lines keep their order
