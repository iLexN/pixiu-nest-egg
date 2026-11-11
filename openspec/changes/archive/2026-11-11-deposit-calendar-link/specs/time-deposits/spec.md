# Spec Delta

## ADDED Requirements

### Requirement: Adding an upcoming deposit to Google Calendar

Each row of the deposits view's 未到期定期 list SHALL offer a 加到日曆 action in its row menu, listed before 編輯. Choosing it SHALL open Google Calendar's new-event editor in a new browser tab, pre-filled with an all-day event on the deposit's end_date. The app SHALL NOT store anything for this action; the user saves the event in Google Calendar. Rows of the history view SHALL NOT offer the action.

#### Scenario: Opening the calendar editor

- **WHEN** the user chooses 加到日曆 on an upcoming deposit with end_date `2027-02-02`
- **THEN** a new tab opens Google Calendar's new-event editor with an all-day event on 2027-02-02 (start `20270202`, exclusive end `20270203`), and no deposit, cash account, or month data changes

#### Scenario: Overdue deposits keep the action

- **WHEN** an unreceived deposit is past its end_date and shows 已到期未收
- **THEN** its 未到期定期 row still offers 加到日曆

#### Scenario: History rows have no action

- **WHEN** the user opens the row menu of a deposit in the history view
- **THEN** the menu does not offer 加到日曆

### Requirement: Private calendar setting

`GET/PATCH /api/settings` SHALL expose `calendar_private_id`, the ID of the user's Private Google calendar, stored as the `calendar.private_id` key in `app_meta`. A patched value SHALL be stored trimmed. A blank string or `null` SHALL clear it, and an unset value reads as `null`. No ID format SHALL be enforced. The 月結 設定 form SHALL let the user view and edit it.

#### Scenario: Setting round-trips

- **WHEN** `calendar_private_id` is patched to ` 7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com `
- **THEN** `GET /api/settings` reports `7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com`

#### Scenario: Blank clears the setting

- **WHEN** `calendar_private_id` is patched to `""` or `null`
- **THEN** `GET /api/settings` reports `calendar_private_id` as `null`

#### Scenario: Edited in the 設定 form

- **WHEN** the user opens the 月結 設定 form
- **THEN** it offers a `Private 日曆 ID` field prefilled with the stored value, and saving an empty field clears the setting

### Requirement: Deposit calendar routing

Adding an own deposit to the calendar SHALL pre-select the calendar named by `calendar_private_id`. While that setting is empty, the editor SHALL open on the user's default calendar.

#### Scenario: Own deposits go to the Private calendar

- **WHEN** `calendar_private_id` is `7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com` and the user adds an upcoming deposit to the calendar
- **THEN** the editor opens with that calendar pre-selected (`src=7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com`)

#### Scenario: No Private calendar set

- **WHEN** `calendar_private_id` is unset and the user adds an upcoming deposit to the calendar
- **THEN** the editor opens on the default calendar, without a calendar pre-selection

### Requirement: Deposit calendar event title

The event title SHALL be `定期: 編號 - 銀行`: the prefix `定期: `, then the label and the bank's name only (`恒生` for `HS`, `渣打` for `SC`, any other code as itself). Each empty part SHALL be dropped together with its ` - ` separator. When both are empty, the title SHALL be `定期`.

#### Scenario: Full title

- **WHEN** the user adds a deposit with label `SC-1234` and bank `SC` to the calendar
- **THEN** the event title is `定期: SC-1234 - 渣打`

#### Scenario: Missing parts are dropped

- **WHEN** the user adds a deposit with label `HS-0090` and no bank to the calendar
- **THEN** the event title is `定期: HS-0090`

#### Scenario: Fallback title

- **WHEN** the user adds a deposit with no label and no bank to the calendar
- **THEN** the event title is `定期`

### Requirement: Deposit calendar event details

The event details SHALL hold one `label: value` line per field, in this order: 到期日, 編號, 銀行, 本金, 利率, 利息, 合計, 開始日. Values SHALL use the view's display formatting: money to 2 decimal places with thousands separators, bank as `name (code)`, 利率 as the percentage the list shows, whether stored or derived, without marking it derived. A field with no value SHALL have no line. Notes SHALL NOT be included.

#### Scenario: Full details with a derived rate

- **WHEN** the user adds a deposit with label `SC-1234`, bank `SC`, principal `140000`, interest `1279`, start_date `2026-10-13`, end_date `2027-02-02` and no stored rate to the calendar
- **THEN** the event details are exactly these lines:
  `到期日: 2027-02-02`
  `編號: SC-1234`
  `銀行: 渣打 (SC)`
  `本金: 140,000.00`
  `利率: 2.977%`
  `利息: 1,279.00`
  `合計: 141,279.00`
  `開始日: 2026-10-13`

#### Scenario: No rate available

- **WHEN** the user adds a deposit with no stored rate and no start_date to the calendar, so no rate can be derived
- **THEN** the details have no `利率:` line and no `開始日:` line, and the remaining lines keep their order
