# Proposal

## Why

`openspec validate --specs --strict` fails on 13 of the 16 main specs. The only cause is 64 requirements whose descriptions exceed 500 characters. These long blocks each pack several behaviors into one paragraph, which makes them hard to test and to modify one at a time. Splitting them lets strict validation pass and gives each behavior its own requirement and scenarios.

## What Changes

- In each over-long requirement, trim the description down to its core behavior. The header and every existing scenario stay unchanged.
- Each behavior removed from a description becomes its own requirement, with at least one scenario. The original wording is kept, not paraphrased.
- No behavior changes. Every normative statement in today's specs appears exactly once after the split. No code, API, or data changes.
- Correct stale names that `consolidate-cash-fx-settings` left in `overview` requirements it did not modify. These requirements still say `overview.goal1_amount`, `overview.goal2_amount`, `overview.goal3_inflation`, `overview.goal3_years`, `overview.final_replacement`, `overview.salary`, and `overview.semi_liquid_target`. The keys the system actually uses are `goals.*`, `ledger.salary`, and `strategy.semi_liquid_target`. One requirement still says that cash rows are edited via "the manual-assets endpoints". This is a naming fix only.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

Requirement structure only; there are no behavior changes:

- `aia-policies`
- `app-navigation`
- `bonds`
- `family-deposits`
- `month-stat`
- `mpf-accounts`
- `overview`
- `stock-dividends`
- `stock-portfolio-summary`
- `stock-trades`
- `stock-yearly-summary`
- `time-deposits`
- `year-in-review`

## Impact

Spec files under `openspec/specs/` only. Afterwards, `openspec validate --specs --strict` passes for all 16 specs.
