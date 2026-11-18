# Tasks

## 1. Create the directory module

- [x] 1.1 `service/months/book.rs` — `live_from` + `MonthBook` + assembly/present/live-tail internals + all event loaders, moved verbatim; `pub(crate) mod tests` with the seed helpers + `open_*`/`tail`/`pool_balance`/`months_and_month` tests
- [x] 1.2 `service/months/read.rs` — `MonthSummaryResponse`/`MonthDetailResponse` + `list`/`summary`/`detail`
- [x] 1.3 `service/months/write.rs` — `upsert`/`create_item`/`update_item` + `mod tests` (`upsert_create_*`, `create_item_*`, `update_item_*` + `empty_patch`/`item` helpers importing book's test helpers)
- [x] 1.4 `service/months.rs` shrinks to the module root: `mod` declarations + `pub use` re-exports of the full previous surface

## 2. Verify

- [x] 2.1 `cargo build` — zero call-site changes compile; `cargo test` — same test count, all pass; `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` clean
