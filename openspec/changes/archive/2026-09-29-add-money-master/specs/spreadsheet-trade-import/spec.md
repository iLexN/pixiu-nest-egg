# Spec Delta

## ADDED Requirements

### Requirement: Money Master seeding

Import SHALL seed the Money Master settings once — `money_master.start_date` `2023-10-27`, `money_master.saved` `1094405.06`, `money_master.target_months` `36`, `money_master.target_amount` `1000000` — from the user's bank-app figures (the workbook's `K31`/`L31`/`K34` literals are stale and SHALL NOT be seeded), leaving the `month_now`/`coming_save` overrides unset so derivation applies. Re-import SHALL NOT overwrite values the user has since changed.

#### Scenario: Fresh import seeds the challenge

- **WHEN** import runs against a database with no `money_master.*` keys
- **THEN** the overview block derives `month_now` from `2023-10-27`, `saved_progress` from `1094405.06 ÷ 1000000`, and `coming_save` from the seeded targets

#### Scenario: Re-import preserves edits

- **WHEN** the user has patched `money_master.saved` and import runs again
- **THEN** the stored `saved` keeps the user's value

### Requirement: Money Master parity comparisons

The parity command SHALL read the workbook's `J29:N35` cells — `K31`/`L31`/`K32`/`L32`/`K34` inputs and `M31`/`N31`/`K33`/`L33`/`M33`/`K35` derived cells — and compare them against the app's block as informational rows only: the app legitimately carries newer bank figures than the frozen workbook, so differences are expected rather than failures.

#### Scenario: Stale workbook reported as informational

- **WHEN** the workbook caches `month` `35`/`saved` `1042052.93` while the app derives `month_now` `36`/`saved` `1094405.06`
- **THEN** the parity report lists the mismatches as informational, not failures
