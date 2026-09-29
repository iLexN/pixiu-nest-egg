# Spec Delta

## MODIFIED Requirements

### Requirement: Reminder checklists
The deposits view SHALL display the workbook's "定期 start step" and "定期 end step" checklists as static reminder text; the checklist still lists "money master" as a step — the user performs it in the app's settings — while 回報率 and Overview 預測 still require manual workbook edits until those sections are migrated.

#### Scenario: Checklist visible
- **WHEN** the user opens the deposits view
- **THEN** the start-step and end-step reminders are visible without editing capability, and the muted note no longer lists money master among the workbook-manual steps
