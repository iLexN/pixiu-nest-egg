# Spec Delta

## ADDED Requirements

### Requirement: Asset row note

Each manual asset row SHALL carry an optional free-text `note`. `POST /api/manual-assets` SHALL accept `note` optionally. `PATCH` SHALL accept `note`: a string sets it, `null` clears it, and an absent field leaves it unchanged. Surrounding whitespace SHALL be trimmed, and a blank note SHALL be stored as absent. The note SHALL NOT affect any total.

#### Scenario: Note set and kept

- **WHEN** the user patches the HS人壽 row with `note` `保單 123, 2026-10 估值`, then patches only its `amount`
- **THEN** the row still reports the note `保單 123, 2026-10 估值`

#### Scenario: Note cleared

- **WHEN** the user patches a row's `note` to `null`, or to a whitespace-only string
- **THEN** the row reports no note

### Requirement: Asset editor note field

The 月結 設定 資產 editor SHALL offer a 備註 field when adding or editing an asset row, and the 資產 table SHALL show a row's note under its name.

#### Scenario: Note shown in settings

- **WHEN** the user saves the HS人壽 row with a 備註
- **THEN** the 資產 table shows the note beneath HS人壽
