# family-deposits Specification

## Purpose

Tracks time deposits (定期) the user holds on behalf of family members — mum, dad, Irene — as pure records with the same registry and 收訖 lifecycle as the user's own 定期, replacing the workbook's `Mum` and `Dad` sheets while keeping that money entirely outside the user's own totals, cash rows, and monthly ledger.

## Requirements

### Requirement: Family deposit registry

The system SHALL maintain a list of family deposit records separate from the user's own deposits, each carrying a required `holder` (free text naming the family member, e.g. `媽媽`, `爸爸`, `Irene`), an optional `label` (the bank reference, e.g. `SC-9179`), an optional `bank` code, optional `principal`, optional `interest` (利息 paid), an optional `start_date` (`YYYY-MM-DD`), a required `end_date` (`YYYY-MM-DD`), and an optional free-text `note` (where the bank's stepped-rate schedule is kept). There SHALL be no numeric rate field. The system SHALL support creating, listing (filterable by holder, by received status, and by end year, ordered by end date), editing, and deleting family deposit records, and SHALL preserve each record's position via a stable `sort_order`.

#### Scenario: Recording a family deposit

- **WHEN** the user records a family deposit with holder `爸爸`, label `SC-9024`, principal 300000, start_date `2026-07-20`, end_date `2026-08-31`, interest 920.29, and note `01 Jul 2026 to 31 Aug 2026: 2.60%`
- **THEN** the deposit is stored and appears in the family deposit list under 爸爸

#### Scenario: Filter by holder

- **WHEN** the user lists family deposits for holder `媽媽`
- **THEN** only 媽媽's deposits are returned, earliest end date first

#### Scenario: Editing a family deposit

- **WHEN** the user changes a family deposit's end_date, interest, or note
- **THEN** the stored record is updated and the holder's summary reflects the change on next read

#### Scenario: Deleting a family deposit

- **WHEN** the user deletes a family deposit
- **THEN** it no longer appears in any list or holder summary

### Requirement: Family deposit validation

The system SHALL validate family deposit input before storing it: `holder` MUST be non-empty after trimming; `end_date` MUST be a calendar date in `YYYY-MM-DD` form; `start_date` MUST be a calendar date in `YYYY-MM-DD` form when present; `principal` and `interest` MUST NOT be negative when present; and at least one of `label`, `principal`, or `interest` MUST be present.

#### Scenario: Missing holder

- **WHEN** the user submits a family deposit with a blank holder
- **THEN** the system rejects it with a validation error naming the holder field, and stores nothing

#### Scenario: Malformed end date

- **WHEN** the user submits a family deposit with end_date `2026/13/45`
- **THEN** the system rejects it with a validation error naming the date field, and stores nothing

#### Scenario: Negative amount

- **WHEN** the user submits a family deposit with principal `-5000`
- **THEN** the system rejects it with a validation error naming the principal field

### Requirement: Derived family deposit fields

The system SHALL compute, on read and never store: `total = COALESCE(principal, 0) + COALESCE(interest, 0)`; `status` = `End` once 收訖 (`received_at` set), otherwise active — a deposit stays active past its `end_date` until receipt is confirmed; and `end_month`/`end_year` taken from `end_date`.

#### Scenario: End date reached but unreceived

- **WHEN** a family deposit's end_date is today or in the past and it is not 收訖
- **THEN** its status is active and it is flagged 已到期未收 in the holder's upcoming list

#### Scenario: Received deposit

- **WHEN** a family deposit has `received_at` set
- **THEN** its status is `End` and it leaves the upcoming list

### Requirement: Family deposit 收訖 lifecycle

Each family deposit SHALL carry a `received_at` date (NULL while unreceived). The system SHALL expose a receive action that marks the deposit received at a given date (defaulting to today) and optionally corrects the stored `interest` to the amount actually paid; receiving an already-received deposit SHALL return 409. The system SHALL expose an unreceive action that clears the flag; unreceiving an unreceived deposit SHALL return 409. Neither action SHALL credit any `manual_assets` row, create or delete any Month Stat item, or produce a Month Stat suggestion. Create and edit SHALL NOT auto-mark past end dates as received — the user confirms receipt explicitly.

#### Scenario: Confirm receipt with corrected interest

- **WHEN** the user 收訖's 爸爸's SC-9024 with interest `920.29`
- **THEN** the deposit is stored received, its interest is `920.29`, it leaves 爸爸's upcoming list, and no cash row or month item changes

#### Scenario: Receiving twice

- **WHEN** the user 收訖's a family deposit that is already received
- **THEN** the system responds 409 and nothing changes

#### Scenario: Undo receipt

- **WHEN** the user 取消收訖's a received family deposit
- **THEN** `received_at` is cleared and the deposit returns to the holder's upcoming list

### Requirement: Per-holder note

The system SHALL keep one optional free-text note per holder (e.g. Mum's AIA 人壽保險 / 危疾保險 policy lines), updatable through a dedicated action where an empty or `null` value clears it. The note SHALL be returned with the holder's summary and SHALL persist even when the holder currently has no deposits.

#### Scenario: Saving a holder note

- **WHEN** the user sets 媽媽's note to `AIA 人壽保險 B027033487 / 危疾保險`
- **THEN** the summary for 媽媽 returns that note on the next read

#### Scenario: Note survives an empty registry

- **WHEN** 媽媽 has a note and the user deletes her last deposit
- **THEN** 媽媽 still appears in the summary with her note and no deposits

#### Scenario: Clearing a note

- **WHEN** the user submits `null` for 爸爸's note
- **THEN** the summary no longer returns a note for 爸爸

### Requirement: Per-holder summary and history

The system SHALL present a summary grouped by holder — every holder that has deposits or a note — each with its note, its unreceived deposits, and its active principal (Σ principal over deposits whose `end_date` is in the future). The page SHALL show one compact block per holder (name, active principal, note) and a **single** 未到期定期 table across all holders — every unreceived deposit ordered by end date (flagging 已到期未收 where past due) with a 持有人 column plus label, bank, principal, interest, total, start_date, end_date, and note; there SHALL NOT be a separate upcoming table per holder. No per-month rollup table is shown per holder. The page SHALL also offer a year filter, defaulting to the current year, listing deposits ending in that year.

#### Scenario: Active principal per holder

- **WHEN** 媽媽 has one unreceived deposit of 100000 ending in the future and five received ones
- **THEN** 媽媽's active principal is 100000 and only the unreceived deposit is in her upcoming list

#### Scenario: History by year

- **WHEN** the user selects year 2026
- **THEN** the family deposits ending in 2026 are listed with their holder, interest, and total

### Requirement: Isolation from the user's own figures

Family deposits SHALL NOT be counted in any figure derived from the user's own deposits: the 定期 summary (active principal, month/bank/year rollups), Overview 已定期 / 半流動資金 / 總數 / 流動資產, Month Stat 利息, `interest_auto`, and deposit suggestions, and the 年結 回顧 定期 figures. Workbook import SHALL NOT read the `Mum`/`Dad` sheets and the parity check SHALL NOT compare them.

#### Scenario: User figures unchanged

- **WHEN** a family deposit of 300000 with interest 920.29 is created and later 收訖'd
- **THEN** the 定期 summary active principal, the Overview 總數 and 流動資產, and the month's `interest_auto` are identical to their values before the family deposit existed

### Requirement: Display formatting and date entry

Family deposit display SHALL follow the app's conventions: money to 2 decimal places, stored values never rounded. Every `start_date` and `end_date` entry field SHALL present and accept `YYYY-MM-DD` with a calendar picker, and `start_date` SHALL default to today when creating a deposit. The `holder` field in the 新增/編輯 form SHALL be a selection list — the default family members 媽媽, 爸爸, Irene plus every holder already present in the data — with an 其他 choice that reveals a text field for a new name; the user SHALL NOT have to type an existing holder's name.

#### Scenario: ISO date entry

- **WHEN** the user opens the 新增 form for a family deposit
- **THEN** the date fields show and accept `YYYY-MM-DD` and start_date is prefilled with today

#### Scenario: Holder picked from a list

- **WHEN** the user opens the 新增 form while 爸爸 already has deposits
- **THEN** the 持有人 field is a dropdown listing 媽媽, 爸爸, Irene (and any other existing holder) plus 其他, and choosing 其他 reveals a text field for a new name
