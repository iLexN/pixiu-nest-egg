# Tasks

## 1. Placement interface

- [x] 1.1 In `backend/src/service/deposits.rs`, add the private `validate(&NewDeposit) -> Result<ValidatedDeposit, ServiceError>`. Change `create` to `create(pool, &NewDeposit)`. Change `convert_plan` to `convert_plan(pool, item_id, NewDeposit)`: load the plan (404), reject a non-deposit kind (409 `forecast item {id} is not a deposit plan`), default `bank`/`principal` from the plan, validate, then run the existing place-and-delete transaction (design D1/D2). Update the module doc comment. Verify `cargo check` reports call-site errors only in the two routes.
- [x] 1.2 Reduce `routes::deposits::create` and `routes::forecast::convert` to one service call each plus `load_one`; drop the now-unused imports (`calc::validate_deposit`/`DepositInput` in `routes/forecast.rs`, keeping them in `routes/deposits.rs` for `update`). Verify `cargo check` is clean and `grep -n "validate_deposit" backend/src/routes/forecast.rs` finds nothing.

## 2. Tests

- [x] 2.1 Migrate the existing `service::deposits` tests to the new interface (drop the `validated()`/`input()` helpers where unused). Add service tests:
  - `create` with an invalid `end_date` returns `Validation` and inserts no row.
  - `convert_plan` with `bank`/`principal` unset defaults them from an `sc_deposit` plan (`SC`, `−amount`).
  - `convert_plan` on a `tax` item returns `Conflict` and leaves the item.
  - `convert_plan` on a missing id returns `NotFound`.
  - `convert_plan` with an invalid `end_date` returns `Validation` and leaves the plan.

  Verify `cargo test service::deposits` passes.
- [x] 2.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`; all pass with the route tests and `backend/tests/api.rs` unchanged.

## 3. Integration check

- [x] 3.1 Restart the backend per AGENTS.md. Against a scratch DB (`WEALTH_DB=/tmp/placement-test.db WEALTH_ADDR=127.0.0.1:8790`), confirm `POST /api/deposits` with a bad `end_date` returns 400 with an `end_date` field error and that `POST /api/forecast-items/999999/convert` returns 404.

## Workflow follow-up

- Run `trail check` before committing.
- Archive after review (`openspec archive deposit-placement-owns-validation`); nothing to sync since `skip_specs: true`.
