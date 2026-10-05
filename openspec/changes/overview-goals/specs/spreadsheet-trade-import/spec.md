# Spec Delta

## ADDED Requirements

### Requirement: Goal amount seeding

Import SHALL seed the milestone target settings once — `overview.goal1_amount` from `Overview!K12` and `overview.goal2_amount` from `Overview!K18` — like the other `app_meta` settings, and SHALL NOT overwrite either key on re-import.

#### Scenario: Fresh import seeds the amounts

- **WHEN** import runs against a database with no `overview.goal*_amount` keys and the workbook caches `K12` `1550000` and `K18` `2500000`
- **THEN** the stored amounts are `1550000`/`2500000` and the overview's milestones derive against them

#### Scenario: Re-import preserves edits

- **WHEN** the user has patched `overview.goal1_amount` and import runs again
- **THEN** the stored amount keeps the user's value

#### Scenario: Blank sheet cell seeds nothing

- **WHEN** `Overview!K12` has no cached number
- **THEN** `overview.goal1_amount` stays unset and the 1st milestone reports absent figures

### Requirement: Goals parity comparisons

The parity command SHALL read the workbook's goals cells — `K12`/`K18` milestone amounts, `K13`/`L12`/`L13` shared figures, `K14`/`L14`/`K20`/`L20` milestone derivations, and `O14`/`O15`/`O16`/`P14`/`P15`/`P16` formula-goal cells — and compare them against the app's `goals` block:

- `K12`/`K18` SHALL be deterministic comparisons: import seeds the settings from those literals, so a mismatch is a failure.
- The derived cells SHALL be informational only: they drift with live prices, today's date, the app's fresher `N4` (its own `income_value_rate` rather than the sheet's cached `YearInReview!L13`), and any user-edited saving inputs, so differences are expected rather than failures.

#### Scenario: Seeded amounts match

- **WHEN** import seeded `goal1_amount`/`goal2_amount` from the same workbook
- **THEN** the `K12`/`K18` parity rows report matches

#### Scenario: Derived cells reported as informational

- **WHEN** the app's `L12`/`P14` differ from the cached cells because prices, the date, or `N4` moved since the workbook last recalculated
- **THEN** the parity report lists those mismatches as informational, not failures
