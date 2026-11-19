# Spec Delta

## MODIFIED Requirements

### Requirement: Private calendar setting

`GET/PATCH /api/settings` SHALL expose `calendar_private_id`, the ID of the user's Private Google calendar, stored as the `calendar.private_id` key in `app_meta`. A patched value SHALL be stored trimmed. A blank string or `null` SHALL clear it, and an unset value reads as `null`. No ID format SHALL be enforced. The 設定 page's 日曆 section SHALL let the user view and edit it.

#### Scenario: Setting round-trips

- **WHEN** `calendar_private_id` is patched to ` 7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com `
- **THEN** `GET /api/settings` reports `7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com`

#### Scenario: Blank clears the setting

- **WHEN** `calendar_private_id` is patched to `""` or `null`
- **THEN** `GET /api/settings` reports `calendar_private_id` as `null`

#### Scenario: Edited in the 設定 form

- **WHEN** the user opens the 設定 page's 日曆 editor
- **THEN** it offers a `Private 日曆 ID` field prefilled with the stored value, and saving an empty field clears the setting
