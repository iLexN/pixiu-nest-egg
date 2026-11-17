# Tasks

## 1. Move the read models

- [x] 1.1 `service::months::list(year: Option<i32>)` — open book, filter, return `Vec<MonthStat>`
- [x] 1.2 `service::months::summary()` — `MonthSummaryResponse` (moved verbatim) + `pool_totals`/`pool_rate`/`pool_rate_year`
- [x] 1.3 `service::months::detail(month: NaiveDate)` — book row + items + events + dismissed + suggestions + `interest_auto` + `suggestions_actionable`; `StoreError::NotFound` for unknown month
- [x] 1.4 Routes delegate: `list`/`summary`/`show` thin; `pub use` re-export both response structs; drop unused `calc`/`store` imports

## 2. Verify

- [x] 2.1 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- [x] 2.2 `pnpm gen:api && pnpm build && pnpm exec vue-tsc --noEmit` — expect no `api-types.ts` diff
- [x] 2.3 Restart backend; spot-check `GET /api/months/summary` and `GET /api/months/{ym}`

## Workflow follow-up

- Archive with `openspec archive` (skip_specs — nothing to sync)
