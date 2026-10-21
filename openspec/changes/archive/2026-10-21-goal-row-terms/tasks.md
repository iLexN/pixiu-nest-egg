# Tasks

## 1. Backend echo

- [x] 1.1 Add `pub total_change_avg: Option<f64>` to `OverviewGoals` in `backend/src/models/overview.rs` (after `interest_avg`, doc-commented as the running `C8` average feeding the Final pace); populate `total_change_avg: input.total_change_avg` in `backend/src/calc/overview.rs` and update the `overview_goals` doc comment. Verify: `cargo build -p wealth-backend` compiles.
- [x] 1.2 In `overview_goals_reproduce_the_sheet_cells` (same file), assert `goals.total_change_avg == input.total_change_avg` beside the existing `saved_avg`/`interest_avg` echo assertions. Verify: `cargo test overview_goals` passes.

## 2. Frontend columns

- [x] 2.1 Add `total_change_avg: number | null` to `OverviewGoals` in `frontend/src/api.ts`. Verify: `pnpm exec vue-tsc --noEmit` typechecks.
- [x] 2.2 In `frontend/src/views/OverviewView.vue`, insert `基數` and `步伐` columns in the 目標 table: milestone rows show `現值`/`current_invest` and `存/年`/`saving_year`; the 3rd row shows `流動資產`/`liquid_assets` and `(歷年存+息)×12`/`combined12`; the Final row shows `總數`/`total_assets` and `歷年總變化×12`/`total_change_avg × 12` — each cell rendering the term name plus its live `fmtMoney` value (`—` while absent). Verify: `pnpm build` succeeds and the card shows the labels/values.
- [x] 2.3 Fix the three footer rows' `colspan="3"` → `colspan="5"` for the widened table. Verify: footer rows span the full width.

## 3. Docs and verification

- [x] 3.1 Update the 目標 card paragraph in `docs/overview.md` for the 基數/步伐 columns and the `total_change_avg` echo.
- [x] 3.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — all clean.
- [x] 3.3 Restart the backend (kill `:8787`, detached `nohup cargo run -p wealth-backend --bin wealth-backend`) and confirm `GET /api/overview` returns `goals.total_change_avg` and the card renders the new columns.
