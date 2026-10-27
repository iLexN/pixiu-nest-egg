# Spec Delta

## ADDED Requirements

### Requirement: Stock notes in the summary
The 持倉總覽 table SHALL let the user expand a stock's row to show its full 備註 beneath it, preserving line breaks, and edit it in place. Saving SHALL go through the stock update endpoint (`PATCH /api/stocks/{id}`); saving blank text SHALL clear the note. A summary-level control SHALL expand or collapse every visible stock's note row at once. When a market's summary is first shown, note rows SHALL start expanded for stocks with a non-empty 備註 and collapsed for the rest; reloading after an edit SHALL keep the user's current expand state.

#### Scenario: Notes expanded by default
- **WHEN** the user opens a market's 總覽 where `中國銀行` has a 備註 and `港燈` has none
- **THEN** `中國銀行`'s note row is shown and `港燈`'s is not

#### Scenario: Expanding a stock shows its full note
- **WHEN** the user expands the note row of a stock that has a 備註 containing line breaks
- **THEN** the full note appears beneath that stock's row with line breaks preserved

#### Scenario: Expand and collapse all notes
- **WHEN** the user activates 全部展開備註 then 全部收起備註
- **THEN** every visible stock's note row is shown, then all of them are hidden

#### Scenario: Editing a note from the summary
- **WHEN** the user edits a stock's note in its expanded row and saves
- **THEN** the note is persisted through the stock update endpoint and the reloaded table shows the new note

#### Scenario: Clearing a note
- **WHEN** the user saves an empty note text for a stock that has a 備註
- **THEN** the stock's note is cleared

#### Scenario: Adding a note where none exists
- **WHEN** the user expands a stock that has no 備註 and enters and saves text
- **THEN** the stock gains that note

#### Scenario: Note edit records no history
- **WHEN** the user edits only a stock's note from the summary
- **THEN** no price-history row is added or changed, per the unchanged-totals rule
