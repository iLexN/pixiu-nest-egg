# Time Deposits Specification

## Purpose

Tracks time deposits (定期) — principal, rate, interest, and maturity — replacing the `定期`/`定期Info` sheets so deposits can be recorded once and the app computes the active totals, maturity calendar, and bank rollups the spreadsheet produces today.

## Requirements

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

### Requirement: Deposit date and note fields

Each deposit record SHALL also carry an optional `start_date` (`YYYY-MM-DD` — when the principal left the bank account), a required `end_date` (`YYYY-MM-DD`), and two optional free-text notes (`note1`, `note2`).

#### Scenario: Dates and notes stored

- **WHEN** the user records a deposit with `start_date`, `end_date`, `note1` and `note2`
- **THEN** the record stores all four fields

### Requirement: Deposit record operations

The system SHALL support creating, listing, editing, and deleting deposit records, and SHALL preserve each record's position via a stable `sort_order`, new records appending after existing ones.

#### Scenario: Ordering preserved

- **WHEN** a deposit is created after existing ones
- **THEN** it appends after them via its `sort_order`

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

The system SHALL compute, on read and never store: `total = COALESCE(principal, 0) + COALESCE(interest, 0)`; `status` = `End` once 收訖 (`received_at` set), otherwise active — a deposit stays active past its `end_date` until the user confirms receipt; `end_month`/`end_year` taken from `end_date`.

#### Scenario: Future end date is active
- **WHEN** a deposit's end_date is later than today
- **THEN** its status is active and it appears in the Upcoming section

#### Scenario: End date reached
- **WHEN** a deposit's end_date is today or in the past and it is not 收訖
- **THEN** it stays in the Upcoming section marked 已到期未收 until the user confirms receipt — the end date alone no longer ends it

#### Scenario: Total from partial fields
- **WHEN** a deposit has interest 539.25 and no principal
- **THEN** its derived total is 539.25

#### Scenario: Blank rate is derived from interest and dates
- **WHEN** a deposit has principal 140000, interest 1279, start_date `2026-10-13`, end_date `2027-02-02`, and no stored rate
- **THEN** its reported rate is 1279 ÷ 140000 × 365 ÷ 112 (about 0.02977), it is flagged as derived, and the stored rate stays empty

#### Scenario: Missing inputs leave the rate empty
- **WHEN** a deposit has no stored rate and lacks a start_date, has no interest, has no positive principal, or has an end_date not after its start_date
- **THEN** its reported rate is empty and not flagged as derived

#### Scenario: Entered rate wins
- **WHEN** a deposit stores rate 0.028162004662004664 alongside principal, interest, and both dates
- **THEN** its reported rate is the stored 0.028162004662004664 and it is not flagged as derived

#### Scenario: Derived rate follows an interest correction
- **WHEN** a deposit with a derived rate is 收訖'd with its interest corrected to a different amount
- **THEN** the next read reports a rate derived from the corrected interest

### Requirement: Derived annual rate

The system SHALL compute on read, never store, when no rate is stored, a derived annual `rate = interest ÷ principal × 365 ÷ days` with `days = end_date − start_date` in calendar days, reported only when principal is positive, interest and start_date are present, and end_date is after start_date, and flagged as derived. A stored rate is always reported as entered and never flagged as derived.

#### Scenario: Rate derived from dates

- **WHEN** a deposit has principal `140000`, interest `1279`, `start_date` `2026-10-13` and `end_date` `2027-02-02`, and no stored rate
- **THEN** its reported `rate` is `1279 ÷ 140000 × 365 ÷ 112` (about `0.02977`), flagged as derived

### Requirement: History view by year
The system SHALL present a year filter defaulting to the current year, listing the deposits whose `end_date` falls in the selected year, and SHALL show a per-month aggregation for that year — for each month, Σ interest and Σ total over the deposits ending that month — equivalent to the workbook's per-year tables (表_2027定期, 表_2028定期) for every year present in the data.

#### Scenario: Default year
- **WHEN** the user opens the deposit history without choosing a year
- **THEN** the current year is selected and its deposits and month aggregation are shown

#### Scenario: Month aggregation covers the whole year
- **WHEN** deposits end in January 2027 and no other 2027 month
- **THEN** the 2027 table shows January with that month's Σ interest and Σ total, and the remaining months with zero

### Requirement: Reminder checklists
The deposits view SHALL display the workbook's "定期 start step" and "定期 end step" checklists as static reminder text. The start step SHALL state that 新增定期 / 轉為定期 automatically debit the chosen 活期 account and record the 月結 調整 −本金. The checklist still lists "money master" as a step — the user performs it in the app's settings — while 回報率 and Overview 預測 still require manual workbook edits until those sections are migrated.

#### Scenario: Checklist visible
- **WHEN** the user opens the deposits view
- **THEN** the start-step and end-step reminders are visible without editing capability, and the muted note no longer lists money master among the workbook-manual steps

#### Scenario: Start step names the automatic debit
- **WHEN** the user opens the deposits view
- **THEN** the start step's first line says placing a deposit automatically 扣活期 and records the 月結調整 −本金

### Requirement: Display formatting
Deposit display SHALL follow the app's conventions: money to 2 decimal places and rates shown as percentages; stored values are never rounded. A derived rate SHALL be displayed like an entered one and identified as derived by a hover hint; editing a deposit whose rate is derived SHALL leave the rate entry empty, showing the derived value only as a hint, so saving does not store it.

#### Scenario: Rate display
- **WHEN** a deposit stores rate `0.028162004662004664`
- **THEN** the view displays it as a percentage such as `2.82%` while the stored value is unchanged

#### Scenario: Derived rate hint
- **WHEN** a deposit's rate is derived
- **THEN** the 利率 column shows it as a percentage with a hover hint saying it was calculated from the interest and dates

#### Scenario: Saving an edit keeps the rate derived
- **WHEN** the user edits a deposit whose rate is derived and saves without typing a rate
- **THEN** the stored rate stays empty and the rate continues to be derived

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

### Requirement: Deposit 收訖 flag

Each deposit SHALL carry a `received_at` date (NULL while unreceived). Create SHALL record `received_at = end_date` when the entered `end_date` is already past.

#### Scenario: Past end_date auto-marks on create

- **WHEN** a deposit is created with `end_date` already past
- **THEN** it is stored received with `received_at = end_date`

### Requirement: Deposit 收訖

The system SHALL expose `POST /api/deposits/:id/receive`. It marks the deposit received at a date (default today) and MAY correct `interest` to the amount actually received.

It credits exactly `principal + interest` (not user-supplied) under the cash-accounts receipt crediting rule, with receipt currency `HKD`. The request carries `bank_in` (default true) and an optional `account_id` override; the 收訖 form preselects the account matching the deposit's bank when one exists.

#### Scenario: One-click maturity

- **WHEN** the user 收訖's a deposit (principal `110000`, interest `993`, ending `2026-10-12`) choosing cash account `渣打`
- **THEN** the deposit is stored received, `渣打` gains `110993`, the `2026-10` month gains a `dep-end` adjustment item of `110993`, and the deposit leaves 未到期定期

#### Scenario: Corrected interest flows into the credit

- **WHEN** the user 收訖's a deposit (principal `110000`, stored interest `993`) entering 實收利息 `1000` and choosing cash account `渣打`
- **THEN** the stored interest becomes `1000`, `渣打` gains `111000`, and the `dep-end` item is `111000`

#### Scenario: Default account when none chosen

- **WHEN** the user 收訖's a deposit with no `account_id` and bank-in on, and HS is the HKD default
- **THEN** HS gains `principal + interest`

#### Scenario: Receipt without a bank-in

- **WHEN** the user 收訖's a deposit with `不存入` selected (`bank_in` false)
- **THEN** only `received_at` and the dep-end item are recorded — no cash account changes

#### Scenario: dep-end item skipped when already stored or monthless

- **WHEN** the deposit's end month already stores its `dep-end:<id>` item or has no month row
- **THEN** the credit and flag apply; the stored item is updated to principal + interest, or none is recorded when the month has no row

### Requirement: Deposit receipt month item and finality

The receive endpoint also records the month's `dep-end:<id>` `adjustment` item for that amount. Re-receiving SHALL return 409. Receipt SHALL NOT be reversible: there is no unreceive endpoint or action.

#### Scenario: Receive is final

- **WHEN** a deposit has been received
- **THEN** a second receive call returns 409 and no unreceive action exists

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

### Requirement: Unreceived deposit list
The system SHALL present the **unreceived** deposits ordered by end date — a matured-but-unreceived deposit is included and flagged 已到期未收 — each showing label, principal, rate, interest, total, end_date, and end_month, with a 收訖 action per row. A deposit leaves the list once 收訖, including one 收訖'd before its `end_date`.

#### Scenario: Overdue deposit stays listed
- **WHEN** a deposit ended `2026-10-13`, today is `2026-10-13`, and it is not 收訖
- **THEN** it appears in 未到期定期 with a 已到期未收 marker

#### Scenario: Early receipt leaves the list
- **WHEN** a deposit whose `end_date` is still in the future is 收訖'd
- **THEN** it no longer appears in 未到期定期

### Requirement: Receipt-based deposit totals
The system SHALL derive the deposit totals over exactly the deposits the unreceived list shows, whatever their `end_date`: the total active principal (Σ principal); a month rollup by end year and month with Σ total, Σ interest, and Σ principal; and a bank rollup by `bank` with the same sums, deposits without a bank in their own bucket. The active principal SHALL feed Overview 已定期 and the live 總數/流動資產 at principal only.

#### Scenario: Active principal total
- **WHEN** the unreceived deposits hold principal 145,000, 140,000, 80,000, and 80,000 and every other deposit is received
- **THEN** the total active principal is 445,000

#### Scenario: Matured deposit keeps counting
- **WHEN** deposits of 110,000 and 35,000 ended `2026-10-13`, today is `2026-10-13`, and neither is 收訖
- **THEN** the total active principal and Overview 已定期 still include 145,000, and live 流動資產 is unchanged by the maturity

#### Scenario: 收訖 removes the deposit from the totals
- **WHEN** the user 收訖's an overdue deposit of 110,000 with interest 993, choosing 存入活期 on the HS cash row
- **THEN** the total active principal drops by 110,000, the HS cash row rises by 110,993, and live 流動資產 rises by only the 993 interest

#### Scenario: Month rollup
- **WHEN** active deposits end in October 2026 and November 2026
- **THEN** the rollup lists those months with their Σ total, Σ interest, and Σ principal

#### Scenario: Bank rollup by bank
- **WHEN** active deposits carry banks `SC` (four deposits) and `HS` (one deposit)
- **THEN** the bank rollup reports an `SC` group (Σ principal 365,000) and an `HS` group (Σ principal 80,000)

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
