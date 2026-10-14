# Spec Delta

## REMOVED Requirements

### Requirement: Workbook seeding
**Reason**: The workbook importer is deleted; the migration is complete.
**Migration**: Values already seeded stay stored and editable in the app; no replacement seeding.

### Requirement: Parity check
**Reason**: The parity checker is deleted with the workbook import; the workbook is no longer maintained, so its cached cells are not a reference.
**Migration**: None; year-review figures are verified by the backend unit tests.
