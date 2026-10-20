# Spec Delta

## Purpose

Keeps the manually entered exchange rates that convert foreign-currency holdings (USD stocks, IBKR USD cash, AIA policies) into HKD, so every section derives its HKD figures from one shared, section-neutral rate.

## ADDED Requirements

### Requirement: Stored rate per currency

The system SHALL store at most one current rate per foreign currency as `rate_to_hkd` (HKD per one unit), with the time it was last updated. Only `USD` SHALL be accepted. The rate MUST be a finite number greater than zero. HKD itself SHALL have no stored rate. Rates SHALL NOT be rounded.

#### Scenario: Rate stored as entered

- **WHEN** the user sets the USD rate to `7.843045`
- **THEN** the stored USD rate is exactly `7.843045` with an updated timestamp

#### Scenario: Invalid rate rejected

- **WHEN** a USD rate of `0`, a negative number, or a non-number is submitted
- **THEN** the request is rejected with a field error and the stored rate is unchanged

#### Scenario: Unsupported currency rejected

- **WHEN** a rate is submitted for `HKD` or `EUR`
- **THEN** the request is rejected and nothing is stored

### Requirement: Rate API

`GET /api/fx-rates` SHALL list the stored rates, each with `currency`, `rate_to_hkd`, and `updated_at`. `PATCH /api/fx-rates/{currency}` SHALL accept `{ "rate": number | null }`: a number sets the rate, `null` clears it. The response SHALL be the currency's resulting entry, with `rate_to_hkd` `null` once cleared.

#### Scenario: Set then read

- **WHEN** the user patches `/api/fx-rates/USD` with rate `7.8`
- **THEN** `GET /api/fx-rates` lists USD at `7.8`

#### Scenario: Clearing the rate

- **WHEN** the user patches `/api/fx-rates/USD` with `null`
- **THEN** USD no longer appears in `GET /api/fx-rates`, and every figure that needs the USD rate is absent on the next read

### Requirement: Rate-dependent figures

Every HKD figure derived from a USD amount SHALL use the stored USD rate. This covers AIA HKD totals, the IBKR computed total, the 基金 and IBKR overview rows, live 總數/流動資產, the AIA premium suggestion, and the year-review blends. While no USD rate is stored, each such figure SHALL be absent, never computed as zero.

#### Scenario: One rate everywhere

- **WHEN** the USD rate changes from `7.8` to `7.85`
- **THEN** the AIA summary HKD totals, the IBKR `computed_total_hkd`, and the overview totals all use `7.85` on their next read

#### Scenario: Unset rate leaves figures absent

- **WHEN** no USD rate is stored
- **THEN** the AIA HKD totals and the IBKR `computed_total_hkd` are absent, and live totals omit the USD-dependent terms
