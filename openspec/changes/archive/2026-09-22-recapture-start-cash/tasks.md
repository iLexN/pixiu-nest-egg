# Tasks

## 1. Backend

- [x] 1.1 `routes/months.rs` `upsert`: in the update branch keep the `LiveTotalsInput` when `recapture` is set and resolve `start_cash` as `patch.start_cash` → recapture's `input.cash_sum` → stored value; leave the create branch storing the patch verbatim. Update the `upsert` doc comment. Verify: `cargo test` passes.

## 2. Frontend

- [x] 2.1 `MonthStatView.vue`: remove `newStartCash` and the 月初 input from the 新增月份 form; `addMonth` sends `api.patchMonth(ym, {})`; rename the recapture button to 重新擷取月初/總數/流動 and its message to 已重新擷取月初及即時總數. Verify: `pnpm exec vue-tsc --noEmit` clean.

## 3. Tests

- [x] 3.1 `backend/tests/api.rs`: add `month_recapture_snapshots_start_cash_from_the_cash_sum` — create with `{}` stores NULL; `recapture` fills `start_cash` from Σ cash assets; a cash-row edit + recapture refreshes it; explicit `start_cash` wins over recapture. Verify: `cargo test -p wealth-backend --test api month_recapture` passes.

## 4. Docs

- [x] 4.1 `docs/DATA_FLOW.md` and `AGENTS.md`: 重新擷取 re-snapshots 總數/流動資產/月初; 月初 = the 活期 cash sum; 新增月份 takes no 月初 input. Verify: wording matches implemented behavior.

## 5. Full verification

- [x] 5.1 `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`. Manual smoke: 新增月份 → 月初 `—`; 重新擷取月初/總數/流動 → 月初 equals the Overview 活期 sum.
