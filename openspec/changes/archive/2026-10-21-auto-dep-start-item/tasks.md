# Tasks

## 1. Backend

- [x] 1.1 Add `service::deposits::create` — one transaction wrapping `deposits::insert_deposit` + `months::record_receipt_item(&mut tx, &format!("{}-01", &start_date[..7]), &format!("dep-start:{id}"), label, -principal)` when `start_date` is `Some` and `principal` is `Some` and nonzero; label falls back to `定期` as dep-end does. Verify with inline tests: deposit with `start_date` in a month that has a row → `dep-start` item of −principal stored; no month row → no item; no `start_date` → no item.
- [x] 1.2 Make `routes::deposits::create` call the service (move `next_sort_order` inside it). Verify: `cargo test` passes including the new cases; a regression check that `build_suggestions` still emits `dep-start` when the item is not stored (existing suggestion tests unchanged).
- [x] 1.3 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`. Verify: all three pass.

## 2. Docs

- [x] 2.1 Update `docs/deposits.md` — creation records the `dep-start` month item like 收訖 records `dep-end`. Verify: the doc no longer describes dep-start as suggestion-only.

## 3. Integration check

- [x] 3.1 Restart the real backend per AGENTS.md (`lsof -ti :8787 | xargs kill`, `nohup cargo run -p wealth-backend --bin wealth-backend … & disown`). Verify: `POST /api/deposits` with a `start_date` in the current month leaves a `dep-start:<id>` adjustment item in `GET /api/months/<ym>` without accepting a suggestion.
