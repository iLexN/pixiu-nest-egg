# Design

## Context

Manual assets (資產) squat inside the 月結 modules across all three layers:
`models/months.rs` holds the types, `store/months.rs` holds the SQL,
`routes/months.rs` holds the four `/api/manual-assets` handlers (tagged
`months`, `months_*_asset` operationIds), and `service::{wealth,overview}`
read them through `store::months`. No `service::manual_assets` exists. The
asset PATCH flow needs `merge_text`, which lives in `routes/patch.rs` — a
module the layer rule forbids `service/` from importing, which is why
`service::{months,mpf}` already carry private copies of it.

## Goals / Non-Goals

**Goals:**
- 資產 gets an addressable home in every layer it touches: `models/`,
  `store/`, `routes/`, and `service/` — named `manual_assets`, matching the
  glossary term.
- The OpenAPI surface stops lying: tag `manual-assets`, operationIds
  `manual_assets_*`.
- One definition of PATCH-merge semantics, importable by routes and service
  alike.
- No behavior change beyond retagging/renames and collect-all validation.

**Non-Goals:**
- No schema/table changes; `manual_assets` the table is untouched.
- No changes to 月結 behavior or to other domains' handlers.
- Frontend untouched (paths and schema shapes are identical; `api.ts` is
  hand-written).

## Decisions

### D1: `service::manual_assets` mirrors `service::cash_accounts`

`validate` collects all `FieldError`s (ServiceError::Validation);
`create` = validate → `next_sort_order` → insert → reload;
`update` = load → merge → validate → update → reload;
`remove` = delete (NotFound on missing id). `list` stays route→store.

- Chosen because `cash_accounts` is the sibling settings-domain row and
  already sets this exact shape; the user picked this over route→store.
- The route keeps only shape concerns (deserialize) and delegates.
- Observable nuance: create/update can now return multiple field errors at
  once; today the route returns the first.

### D2: PATCH-merge helpers move to crate-level `patch`

`routes/patch.rs` → `backend/src/patch.rs`; routes repoint
`super::patch` → `crate::patch`; the private `merge_text` copies in
`service::{months,mpf}` are deleted and import `crate::patch`.

- Chosen because `service::manual_assets::update` needs `merge_text`
  (`ManualAssetPatch.note` is `Option<Option<String>>`) — the third
  callsite the 2026-11-05 learning predicted would force this lift.
- Alternatives: a fourth private copy (rejected — the lift was already
  inevitable); `models::patch` (rejected — helpers aren't model types;
  a crate-level util sits beside `error.rs`/`db.rs`).

### D3: Renames on move

Inside the new `store::manual_assets`: `manual_asset_sum`→`sum`,
`load_assets`→`load_all`, `load_asset`→`load_one`,
`next_asset_sort_order`→`next_sort_order`, `insert_asset`/`update_asset`/
`delete_asset`→`insert`/`update`/`delete` — matching
`cash_accounts::{load_all,load_one,next_sort_order}` conventions.
`models/mod.rs` flat re-export keeps `crate::models::ManualAsset` working;
no import churn.

### D4: Test placement follows the seam

Workflow tests (`create` defaults liquidity to long, PATCH reclassifies
liquidity, note set/kept/cleared) move to `service::manual_assets` tests
against `connect_memory`. The `unknown_liquidity` rejection is a
deserialize-level check and moves with the handlers to
`routes::manual_assets` tests.

## Risks / Trade-offs

- OperationId rename changes generated `operationId`s → the hand-written
  `api.ts` and schema-based `api-types.ts` are unaffected; `pnpm gen:api`
  is run anyway per convention.
- The `manual-assets` OpenAPI tag appears in the doc — the api-docs spec
  delta covers it (see specs/api-docs/spec.md).
