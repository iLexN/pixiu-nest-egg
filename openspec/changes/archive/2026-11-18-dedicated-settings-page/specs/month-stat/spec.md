# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: 月結 item editor and settings card`
- TO: `### Requirement: 月結 item editor`

## MODIFIED Requirements

### Requirement: 月結 item editor

The view SHALL also show an item editor per month listing stored items and auto-suggestions — pending ones first with accept/dismiss actions, dismissed ones after them marked `忽略（不計）` with accept/undismiss actions — plus manual add (娛樂 items offering a 不計入生活支出 flag, 利息 among the categories). For months before the previous month the suggestion rows SHALL show no actions.

#### Scenario: Suggestions offer accept and dismiss

- **WHEN** an actionable month has a pending auto-suggestion
- **THEN** the item editor offers accept and dismiss actions on it

#### Scenario: Dismissed suggestion stays visible

- **WHEN** the user dismisses a suggestion
- **THEN** the row remains listed marked `忽略（不計）` and offers accept and undismiss actions

#### Scenario: Old month shows read-only suggestions

- **WHEN** the user opens a month before the previous month that has suggestions
- **THEN** the rows show with no actions

### Requirement: Asset editor note field

The 設定 資產 editor SHALL offer a 備註 field when adding or editing an asset row, and the 資產 table SHALL show a row's note under its name.

#### Scenario: Note shown in settings

- **WHEN** the user saves the HS人壽 row with a 備註
- **THEN** the 資產 table shows the note beneath HS人壽
