# Tasks

## 1. Split the loads

- [x] 1.1 `service::settings`: pure `semi_liquid_target_of(&Settings)` / `goal_constants_of(&Settings)`; async wrappers delegate
- [x] 1.2 `service::wealth`: `IbkrReads` + `ibkr_reads(pool)` + `totals_input_with(pool, &reads)`; `totals_input` delegates
- [x] 1.3 `service::wealth::LiveContext { input, totals, stored, book, reads }` + `open(pool)` + `semi_liquid_target()`

## 2. Rewire the page builds

- [x] 2.1 `overview`: `compose_ibkr_block` + `ibkr_block_from(pool, &ctx)`; `build` opens `LiveContext`, derives settings values from `stored`
- [x] 2.2 `forecast::build` opens `LiveContext`; drops its own `totals_input`/`Settings::load`/`semi_liquid_target`/`open_with_live`

## 3. Verify

- [x] 3.1 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- [x] 3.2 `pnpm gen:api && pnpm build && pnpm exec vue-tsc --noEmit` — expect no `api-types.ts` diff
- [x] 3.3 Restart backend; spot-check `GET /api/overview` and `GET /api/forecast`

## Workflow follow-up

- Archive with `openspec archive` (skip_specs — nothing to sync)
