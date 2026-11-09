# Proposal

## Why

The 定期 placement module (`service::deposits::{create, convert_plan}`) trusts its callers to validate first. Both `routes::deposits::create` and `routes::forecast::convert` call `calc::validate_deposit` themselves, then pass both the validated deposit and the raw body, because `place` needs `debit_account_id`/notes from the body. The 轉為定期 rules also sit in the route: loading the plan, the "not a deposit plan" 409, and defaulting `bank` from the plan's kind and `principal` from `−amount`. So the module's interface is two parameters wide for one input, and any new caller can reach `place` without the validation step. ADR-0005 says domain validation travels with the workflow; this placement workflow is where that rule is not yet followed.

## What Changes

- `service::deposits::create(pool, &NewDeposit)` validates inside. The `ValidatedDeposit` parameter is removed from its interface.
- `service::deposits::convert_plan(pool, item_id, NewDeposit)` owns the whole 轉為定期 step: loading the plan (missing → `NotFound`), rejecting a non-deposit plan (`Conflict`), the bank/principal defaults, validation, and the existing placement-plus-delete transaction.
- `routes::deposits::create` and `routes::forecast::convert` become one-call adapters and no longer import `validate_deposit`.
- Service tests exercise validation, the plan defaults, and error precedence through the new interface.

No externally observable behavior changes: the same endpoints, the same status codes in the same precedence (404 → 409 → 400 field errors → 400 debit errors), and the same rows written.

## Capabilities

### New Capabilities

None. This is a pure refactor, so `skip_specs: true` is set.

### Modified Capabilities

None. No requirement changes, so `skip_specs: true` is set.

## Impact

- `backend/src/service/deposits.rs`, `backend/src/routes/deposits.rs`, `backend/src/routes/forecast.rs`.
- No API, schema, or frontend change.
