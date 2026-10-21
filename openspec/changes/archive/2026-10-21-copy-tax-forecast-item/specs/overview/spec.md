# Spec Delta

## ADDED Requirements

### Requirement: Copy a Tax forecast item to next year

The Tax/基金/醫療保險 editor SHALL offer a `複製` action on each item that creates a new item of the same `kind` in the same month of the following year, carrying the amount and note currently entered in the item's row, while the source item is unchanged.

#### Scenario: Copy lands in the same month next year

- **WHEN** a `tax` item in `2026-10` with amount `-6000` and note `稅` is copied
- **THEN** a new `tax` item exists in `2027-10` with amount `-6000` and note `稅`, and the `2026-10` item is unchanged

#### Scenario: Copy stored outside the window

- **WHEN** a `tax` item is copied
- **THEN** the new item's month is outside the seven-month forecast window, stays stored, and enters the grid as the months slide forward

#### Scenario: Only the Tax row offers it

- **WHEN** the editor is open for a kind other than `tax`
- **THEN** no `複製` action is offered
