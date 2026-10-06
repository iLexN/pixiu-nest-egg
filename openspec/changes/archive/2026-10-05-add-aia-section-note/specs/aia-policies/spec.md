# Spec Delta

## ADDED Requirements

### Requirement: AIA section note

The system SHALL keep a free-text note for the whole AIA section, stored under `aia.note` and returned by `GET /api/aia/summary`; `PATCH /api/aia/note` SHALL update it and SHALL remove the stored note when the submitted text is blank. The AIA view SHALL show it in a 備註 block editable in place.

#### Scenario: Section note

- **WHEN** the user saves a note on the AIA page
- **THEN** the note is persisted and shown on subsequent loads

#### Scenario: Clearing the note

- **WHEN** the user saves the note with empty text
- **THEN** the note is cleared and no longer reported by the summary
