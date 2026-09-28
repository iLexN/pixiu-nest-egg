# Spec Delta

## MODIFIED Requirements

### Requirement: Manual balances and salary

The system SHALL store named manual balances in two kinds — `cash` accounts (e.g. HS, 渣打 — the 活期 behind 月初) and `asset` rows (e.g. Irene, HS人壽) — each with `label`, `amount`, `sort_order`, `updated_at`, and a `liquidity` classification of `short` or `long` (default `long`; meaningful for `asset` rows, which the Overview 策略 block places in 短期可取回 or 長期可取回 accordingly — `cash` rows carry it but it has no effect on them), exposed through `GET/POST /api/manual-assets` and `PATCH/DELETE /api/manual-assets/:id`; `POST` SHALL accept `liquidity` optionally and `PATCH` SHALL accept it as a field to change, rejecting any value other than `short`/`long`. The system SHALL also store a current salary in `app_meta` (`overview.salary`) exposed through `PATCH /api/months/settings`. Month rows SHALL snapshot the salary at creation so past months keep their era's figure. The 資產 editor SHALL offer a 流動性 choice (短期 / 長期) for `asset` rows.

#### Scenario: Update cash balances

- **WHEN** the user saves HS `30538.78` and 渣打 `1364.89`
- **THEN** later live totals use 活期 `31903.67`

#### Scenario: Salary snapshot per row

- **WHEN** the salary setting is `52700` and a new month row is created, then the setting is raised to `55000`
- **THEN** the existing row still reports `52700` while the next created row reports `55000`

#### Scenario: Liquidity defaults to long

- **WHEN** the user creates an `asset` row without specifying `liquidity`
- **THEN** the stored row reports `liquidity` `long`

#### Scenario: Liquidity reclassified

- **WHEN** the user patches an `asset` row's `liquidity` to `short`
- **THEN** the row reports `short` and the next `GET /api/overview` counts its amount in `short_term` instead of `long_term`

#### Scenario: Invalid liquidity rejected

- **WHEN** a request carries `liquidity` `medium`
- **THEN** the request is rejected with a validation error and nothing is stored
