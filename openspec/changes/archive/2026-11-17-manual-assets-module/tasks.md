# Tasks

## 1. Lift PATCH-merge helpers to `crate::patch`

- [x] 1.1 Create `backend/src/patch.rs` with `merge`/`merge_opt`/`merge_num`/`merge_text` moved verbatim; declare `mod patch` in the crate root
- [x] 1.2 Delete `backend/src/routes/patch.rs` and `pub mod patch` in `routes/mod.rs`; repoint every `super::patch` import in `routes/*` to `crate::patch`
- [x] 1.3 Delete the private `merge_text` in `service/months.rs` and `service/mpf.rs`; import `crate::patch::merge_text`

## 2. Move the 資產 modules

- [x] 2.1 Move `ManualAsset`, `NewManualAsset`, `ManualAssetPatch`, `ManualAssetLiquidity` from `models/months.rs` to `models/manual_assets.rs`; add `mod manual_assets; pub use manual_assets::*;` to `models/mod.rs`
- [x] 2.2 Move the asset SQL block from `store/months.rs` to `store/manual_assets.rs` with renames (`sum`, `load_all`, `load_one`, `next_sort_order`, `insert`, `update`, `delete`); `pub mod manual_assets` in `store/mod.rs`
- [x] 2.3 Repoint `service::wealth::totals_input` → `store::manual_assets::sum` and `service::overview::build` → `store::manual_assets::load_all`

## 3. Service + routes

- [x] 3.1 Add `service/manual_assets.rs` (`validate`, `create`, `update`, `remove`) mirroring `service::cash_accounts`; `pub mod manual_assets` in `service/mod.rs`
- [x] 3.2 Move the four `/api/manual-assets` handlers to `routes/manual_assets.rs`, retagged `manual-assets`, operationIds `manual_assets_{list,create,update,remove}`; create/update/remove delegate to the service
- [x] 3.3 Drop the asset handlers/`validate_asset`/unused imports from `routes/months.rs`; `pub mod manual_assets` + repoint `routes!()` in `routes/mod.rs`

## 4. Tests + verification

- [x] 4.1 Move workflow tests to `service::manual_assets` tests (connect_memory); move `unknown_liquidity` to `routes::manual_assets` tests
- [x] 4.2 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- [x] 4.3 `pnpm gen:api && pnpm build && pnpm exec vue-tsc --noEmit`
- [x] 4.4 Restart the backend; confirm `GET /api/manual-assets` answers and the `manual-assets` tag shows in `/scalar`

## Workflow follow-up

- Archive with `openspec archive` and sync the api-docs delta into `openspec/specs/`
- Update `docs/` if wiring references changed
