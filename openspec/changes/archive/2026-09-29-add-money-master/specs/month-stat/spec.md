# Spec Delta

## ADDED Requirements

### Requirement: Money Master settings

`PATCH /api/months/settings` SHALL also accept the Money Master challenge fields — `start_date` (`YYYY-MM-DD`), `saved`, `target_months`, `target_amount`, and the optional `month_now`/`coming_save` overrides — stored as `money_master.*` keys in `app_meta`; `GET /api/months/settings` SHALL return them. `start_date` SHALL be a valid `YYYY-MM-DD` date or `null`; `month_now` and `target_months` SHALL be finite positive integers; `target_amount` SHALL be finite and positive; `saved` and `coming_save` SHALL be finite (negative allowed — the bank's figure goes negative while ahead of target). `null` clears a stored value; clearing `month_now` or `coming_save` returns that figure to its derivation. These figures are copied from the bank app — the challenge's own data — so they are user inputs, not derivations of tracked data.

#### Scenario: Settings updated

- **WHEN** `saved` is patched to `1094405.06` and `target_amount` to `1000000`
- **THEN** the settings response reports both and the overview block's `saved_progress`/`coming_save`/`can_use` follow

#### Scenario: Invalid field rejected

- **WHEN** `target_months` is patched to `0` or `start_date` to `not-a-date`
- **THEN** the request fails with 400 and field errors on the offending fields

#### Scenario: Override cleared restores derivation

- **WHEN** `month_now` is patched to `null` while `start_date` is `2023-10-27`
- **THEN** the overview block derives `month_now` from `start_date` again

#### Scenario: New challenge reconfigured

- **WHEN** the user patches `start_date`, `saved`, and the targets for a new challenge with no overrides stored
- **THEN** `month_now` restarts from `1`-indexed derivation and `coming_save` re-derives against the new targets
