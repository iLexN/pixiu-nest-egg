# Tasks

## 1. Store-owned transactions + typed record

- [x] 1.1 `store::aia::delete_with_events` and `store::bonds::delete_with_coupons` gain pool-taking variants that own their tx; `store::stocks::reorder(pool, market, ids)` owns the sort-order loop; routes call them without `pool.begin()`; verify `cargo check` passes.
- [x] 1.2 `store::year_review::record_row` (or a sibling) returns a typed record of `Option<f64>` fields instead of `SqliteRow`; verify no `try_get` remains in `routes/`.

## 2. service::aia (new module)

- [x] 2.1 Create `service/aia.rs` with `create_event` (snapshot prev → insert → apply payment) and `remove_event` (undo derived effects), each in one tx; register in `service/mod.rs`; migrate `routes/aia.rs`; verify `cargo test` passes.
- [x] 2.2 Inline `connect_memory()` tests: payment applies premium/remaining-years/next_pay_date; removal undoes them; verify `cargo test service::aia` passes.

## 3. service write fns for months, mpf, year_review

- [x] 3.1 `service::months::{create_item, update_item, upsert}`: dismissal-clear on accept, entertainment-flag merge rules, recapture/snapshot precedence; upsert re-presents via `MonthBook`; entertainment flag errors as `ServiceError::Validation`; migrate `routes/months.rs`; verify `cargo test` passes.
- [x] 3.2 `service::mpf::update`: `update_account` + `record_history` iff values changed, in one tx; migrate `routes/mpf.rs`; verify `cargo test` passes.
- [x] 3.3 `service::year_review::update`: merge over the typed record, delete-if-all-cleared, `upsert_hk_sold_pl` + `delete_empty_hk_snapshot`; migrate `routes/year_review.rs`; verify `cargo test` passes.
- [x] 3.4 Inline `connect_memory()` tests for each rule moved; verify `cargo test` passes.

## 4. Dedup + sweep

- [x] 4.1 Delete `active_principal`/`active_principal_of` from `routes/bonds.rs`; use `service::bonds`; verify `cargo check` passes.
- [x] 4.2 Grep confirms zero `pool.begin()` and zero `sqlx::`/`try_get` in non-test `routes/` code; verify the layer rule holds.

## 5. Verification

- [x] 5.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`; all pass.
- [x] 5.2 Restart the backend per AGENTS.md and confirm `PATCH /api/year-review/{y}`, `POST /api/aia/events`, `PATCH /api/months/{ym}`, `PATCH /api/mpf/{id}` behave identically.

## Workflow follow-up

- Archive the change after review (`openspec archive service-seam-writes`), nothing to sync (`skip_specs: true`).
