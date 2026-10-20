# Spec Delta

## MODIFIED Requirements

### Requirement: Family deposit registry

The system SHALL maintain a list of family deposit records separate from the user's own deposits, each carrying a required `holder` (free text naming the family member, e.g. `媽媽`, `爸爸`, `Irene`), an optional `label` (the bank reference, e.g. `SC-9179`), an optional `bank` code, and optional `principal`.

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

## ADDED Requirements

### Requirement: Family deposit date, interest and note fields

Each family deposit record SHALL also carry optional `interest` (利息 paid), an optional `start_date` (`YYYY-MM-DD`), a required `end_date` (`YYYY-MM-DD`), and an optional free-text `note` (where the bank's stepped-rate schedule is kept). There SHALL be no numeric rate field.

#### Scenario: Stepped-rate schedule lives in the note

- **WHEN** the user records a family deposit with note `01 Jul 2026 to 31 Aug 2026: 2.60%`
- **THEN** the record stores `start_date`, `end_date`, `interest` and the note, with no numeric rate field

## ADDED Requirements

### Requirement: Family deposit list operations

The system SHALL support creating, listing (filterable by holder, by received status, and by end year, ordered by end date), editing, and deleting family deposit records, and SHALL preserve each record's position via a stable `sort_order`.

#### Scenario: Listing filters

- **WHEN** family deposits exist under several holders
- **THEN** the list can be filtered by holder, by received status, and by end year, ordered by end date

## MODIFIED Requirements

### Requirement: Per-holder summary and history

The system SHALL present a summary grouped by holder — every holder that has deposits or a note — each with its note, its unreceived deposits, and its active principal (Σ principal over the holder's deposits that are not 收訖, whatever their `end_date` — a matured-but-unreceived deposit keeps counting until receipt is confirmed).

#### Scenario: Active principal per holder

- **WHEN** 媽媽 has one unreceived deposit of 100000 ending in the future and five received ones
- **THEN** 媽媽's active principal is 100000 and only the unreceived deposit is in her upcoming list

#### Scenario: Matured deposit counts until 收訖

- **WHEN** 媽媽's deposit SC-9179 of 100000 ended `2026-10-13`, today is `2026-10-13`, and it is not 收訖
- **THEN** 媽媽's active principal still includes 100000 and the deposit is flagged 已到期未收; after 收訖 her active principal drops by 100000

#### Scenario: History by year

- **WHEN** the user selects year 2026
- **THEN** the family deposits ending in 2026 are listed with their holder, interest, and total

## ADDED Requirements

### Requirement: Holder block and single upcoming table

The page SHALL show one compact block per holder (name, active principal, note) and a **single** 未到期定期 table across all holders — every unreceived deposit ordered by end date (flagging 已到期未收 where past due) with a 持有人 column plus label, bank, principal, interest, total, start_date, end_date, and note; there SHALL NOT be a separate upcoming table per holder. No per-month rollup table is shown per holder.

#### Scenario: One upcoming table for all holders

- **WHEN** two holders each have an unreceived deposit
- **THEN** the page shows one 未到期定期 table listing both deposits with a 持有人 column, and no per-holder table

## ADDED Requirements

### Requirement: Family deposit year filter

The page SHALL also offer a year filter, defaulting to the current year, listing deposits ending in that year.

#### Scenario: Year filter

- **WHEN** the page opens with deposits ending in the current and the next year
- **THEN** the year filter defaults to the current year and lists only that year's deposits

## MODIFIED Requirements

### Requirement: Display formatting and date entry

Family deposit display SHALL follow the app's conventions: money to 2 decimal places, stored values never rounded. Every `start_date` and `end_date` entry field SHALL present and accept `YYYY-MM-DD` with a calendar picker, and `start_date` SHALL default to today when creating a deposit.

#### Scenario: ISO date entry

- **WHEN** the user opens the 新增 form for a family deposit
- **THEN** the date fields show and accept `YYYY-MM-DD` and start_date is prefilled with today

#### Scenario: Holder picked from a list

- **WHEN** the user opens the 新增 form while 爸爸 already has deposits
- **THEN** the 持有人 field is a dropdown listing 媽媽, 爸爸, Irene (and any other existing holder) plus 其他, and choosing 其他 reveals a text field for a new name

## ADDED Requirements

### Requirement: Holder selection list

The `holder` field in the 新增/編輯 form SHALL be a selection list — the default family members 媽媽, 爸爸, Irene plus every holder already present in the data — with an 其他 choice that reveals a text field for a new name; the user SHALL NOT have to type an existing holder's name.

#### Scenario: Holder options offered

- **WHEN** the user opens the 新增 form for a family deposit
- **THEN** the holder field offers 媽媽, 爸爸, Irene and every stored holder, plus an 其他 choice revealing a text field

