# Spec Delta

## MODIFIED Requirements

### Requirement: Goal settings

The month-stat settings SHALL carry the five goal inputs as `app_meta` keys: `overview.goal1_amount` and `overview.goal2_amount` (the milestone targets — the sheet's `K12`/`K18` literals), `overview.goal3_inflation` (the sheet's `1.07`), `overview.goal3_years` (the sheet's `25+5`), and `overview.final_replacement` (the sheet's `0.7`).

- `GET /api/months/settings` SHALL return the two amounts as stored (`null` while unset) and the three constants as their effective values (the defaults while unset).
- `PATCH /api/months/settings` SHALL accept each field: the amounts must be finite and ≥ 0, the constants finite and > 0; invalid values are rejected with a field error. `null` clears an amount (blanking that milestone's figures) or resets a constant to its default; an absent field leaves the stored value untouched.

#### Scenario: Constants round-trip through defaults

- **WHEN** no constant is stored
- **THEN** settings report `goal3_inflation` `1.07`, `goal3_years` `30`, `final_replacement` `0.7`; patching `goal3_inflation` `1.05` returns `1.05`, and patching `null` returns `1.07` again

#### Scenario: Validation

- **WHEN** the patch carries `goal1_amount` `-1` or `goal3_years` `0`
- **THEN** the request is rejected with a field error and nothing is written

#### Scenario: Clearing a milestone amount

- **WHEN** `goal1_amount` was `1550000` and the patch carries `null`
- **THEN** settings report `goal1_amount` `null` and the overview's 1st milestone is blank
