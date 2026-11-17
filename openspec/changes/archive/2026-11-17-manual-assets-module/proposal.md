# Proposal

## Why

Manual assets (資產) are a glossary-level domain — they feed live totals and
the 策略 liquidity tiers — but they have no module of their own: their model,
SQL, and HTTP handlers all squat inside the 月結 (`months`) modules, and the
OpenAPI surface mislabels them (`tag = "months"`, `months_*_asset`
operationIds on `/api/manual-assets`). The architecture review flagged this
as the top deepening candidate.

## What Changes

- Extract `ManualAsset`/`NewManualAsset`/`ManualAssetPatch`/`ManualAssetLiquidity`
  from `models/months.rs` into `models/manual_assets.rs` (flat re-export, no
  import churn).
- Extract the asset SQL block from `store/months.rs` into
  `store/manual_assets.rs`, renaming on move to store conventions:
  `manual_asset_sum`→`sum`, `load_assets`→`load_all`, `load_asset`→`load_one`,
  `next_asset_sort_order`→`next_sort_order`, `insert_asset`/`update_asset`/
  `delete_asset`→`insert`/`update`/`delete`.
- New `service::manual_assets` mirroring `service::cash_accounts`: `validate`
  (collects all `FieldError`s — previously the route returned the first
  error only), `create`, `update`, `remove`.
- Move the four `/api/manual-assets` handlers from `routes/months.rs` to
  `routes/manual_assets.rs`, retagged `manual-assets` with
  `manual_assets_{list,create,update,remove}` operationIds; create/update/
  remove delegate to the service.
- Lift `routes/patch.rs` to crate-level `patch` so the service can use the
  PATCH-merge helpers (the layer rule forbids `service` importing `routes`);
  the private `merge_text` copies in `service::{months,mpf}` are deleted.

No endpoint path, method, or schema changes; the only observable deltas are
the OpenAPI tag/operationIds and collect-all validation errors on the manual
assets endpoints.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `api-docs`: the tag-grouping requirement gains `manual assets` as its own
  domain tag; manual-assets operations leave the `months` grouping.

## Impact

- `backend/src/patch.rs` (new), `backend/src/models/manual_assets.rs` (new),
  `backend/src/store/manual_assets.rs` (new),
  `backend/src/service/manual_assets.rs` (new),
  `backend/src/routes/manual_assets.rs` (new).
- `models/months.rs`, `store/months.rs`, `routes/months.rs`, `routes/mod.rs`,
  `service/{mod,months,mpf,wealth,overview}.rs`, `routes/patch.rs` (deleted),
  `routes/{aia,bonds,deposits}.rs` (import repoint).
- `openspec/specs/api-docs/spec.md` via delta.
