# Design

## Context

See proposal.md (Why). Current state:

- `service::deposits::place(conn, &ValidatedDeposit, &NewDeposit)` is private. It inserts the row, applies the optional 活期 debit (HKD bank account, overdraft checked inside the transaction), and records the `dep-start:<id>` item.
- `create(pool, &ValidatedDeposit, &NewDeposit)` and `convert_plan(pool, item_id, &ValidatedDeposit, &NewDeposit)` wrap `place` in a transaction; `convert_plan` also deletes the plan with `store::forecast::delete_item_in_tx`.
- `routes::forecast::convert` loads the plan (404), rejects a non-deposit kind (409), defaults `bank` (`ForecastItemKind::default_bank`) and `principal` (`−amount`), then validates (400).
- ADR-0005 keeps shape checks (parsing, ranges, "at least one field") in routes and puts domain validation with the workflow. `validate_deposit` is domain validation: date order, required end date, numeric sanity.

## Goals / Non-Goals

**Goals:**
- The placement interface takes the raw `NewDeposit` and nothing else; no path into `place` skips validation.
- The 轉為定期 rules (plan lookup, kind check, defaults) live in `service::deposits`.
- Error precedence is identical to today's.

**Non-Goals:**
- `PATCH /api/deposits/{id}` (`routes::deposits::update`). It is a single-store write with merge rules; moving it would be the "service pass-through" ADR-0005 rejects. It stays in the route.
- 家人 deposits, bonds, and dividends, which follow the same validate-then-store pattern. They are single-store CRUD with no cross-store workflow, so ADR-0005 keeps them in routes.
- Changing `calc::validate_deposit` itself.

## Decisions

### D1. New interface

```rust
pub async fn create(pool: &SqlitePool, body: &NewDeposit) -> Result<i64, ServiceError>;
pub async fn convert_plan(pool: &SqlitePool, item_id: i64, body: NewDeposit) -> Result<i64, ServiceError>;
```

A private `fn validate(body: &NewDeposit) -> Result<ValidatedDeposit, ServiceError>` maps `body` into `DepositInput` and turns field errors into `ServiceError::Validation`. Both entry points call it before opening the transaction, so a rejected body never starts one. `place` keeps its current private signature.

`convert_plan` takes `body` by value because it fills in the defaults (`bank` when `None`, `principal` when `None`) before validating, as the route does today.

*Alternative: `place` validates inside the transaction.* Rejected: validation is pure and needs no connection, and a failure before `begin` is cheaper and leaves nothing to roll back.

### D2. Error precedence in `convert_plan`

The order stays the same:
1. `store::forecast::load_item` → `StoreError::NotFound` → 404
2. Non-deposit kind → `ServiceError::Conflict("forecast item {id} is not a deposit plan")` → 409
3. Defaults applied, then `validate` → 400 field errors
4. Inside the transaction: the debit-account checks → 400 on `debit_account_id`; an unknown account → 404

The plan is loaded outside the transaction, as today. A plan deleted between the load and the delete makes `delete_item_in_tx` fail the transaction, so nothing is left half-written.

### D3. Routes

- `routes::deposits::create` → `service::deposits::create(&pool, &body)`, then `store::load_one`.
- `routes::forecast::convert` → `service::deposits::convert_plan(&pool, id, body)`, then `store::deposits::load_one`.
- Neither route imports `validate_deposit` or `DepositInput` any more, though `routes::deposits::update` still does.

## Risks / Trade-offs

- [Error message or status drift] → The existing route tests (`routes::forecast` convert tests, `routes::deposits` tests, `backend/tests/api.rs` convert/create cases) stay unchanged and must pass. New service tests pin the precedence.
- [`update` remains asymmetric (route-validated)] → This is deliberate under ADR-0005 and recorded as a non-goal so a later review doesn't re-suggest it as an oversight.
