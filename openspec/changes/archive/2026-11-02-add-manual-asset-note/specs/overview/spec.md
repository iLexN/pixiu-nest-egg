# Spec Delta

## ADDED Requirements

### Requirement: Asset row note tooltip

Each `assets` entry in `GET /api/overview` SHALL carry `note`: the manual asset row's note for a manual row, and absent for every module row (港股, 債券, 基金, MPF, IBKR) or a manual row without one. On 總覽, the asset table SHALL show a row's note as a tooltip when the mouse is over that row.

#### Scenario: Hover shows the note

- **WHEN** the HS人壽 manual row has a note and the user hovers its row in the 總覽 資產 table
- **THEN** the note appears as a tooltip

#### Scenario: Module rows carry no note

- **WHEN** the overview is requested
- **THEN** the 港股, 債券, 基金, MPF and IBKR rows carry no note
