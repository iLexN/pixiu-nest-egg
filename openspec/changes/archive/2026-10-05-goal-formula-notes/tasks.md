# Tasks

## 1. Backend: echo the formula constants

- [x] 1.1 Add `goal3_inflation: f64`, `goal3_years: f64`, `final_replacement: f64` to `OverviewGoals` in `backend/src/models.rs` with doc comments (effective constants, defaults while unset); verify `cargo build` compiles and the scalar OpenAPI docs show all three.
- [x] 1.2 Populate them in `calc.rs::overview_goals` from `input.goal3_*`/`input.final_replacement`; verify with `cargo test overview_goals`.
- [x] 1.3 Extend an `overview_goals` test (e.g. `overview_goals_constants_override_the_defaults`) asserting the response echoes the effective constants; verify `cargo test overview_goals` passes.

## 2. Frontend: formula footnote on the 目標 card

- [x] 2.1 Add `goal3_inflation: number`, `goal3_years: number`, `final_replacement: number` to `OverviewGoals` in `frontend/src/api.ts`; verify `pnpm exec vue-tsc --noEmit` still passes.
- [x] 2.2 Add a second `<p class="muted">` after the existing N4 footnote in the 目標 card rendering `3rd = 生活預算 (…) × 通脹 (…) × 12 × 年期 (…) · Final = 薪金 (…) × 替代率 (…) × 12 ÷ N4 (…)` — `living_budget`/`salary` via `fmtMoney`, `final_replacement` via `fmtPercentShort`, `n4_rate` via `fmtPercent`, constants raw; `—` for absent inputs; verify in the browser that the shown inputs produce the card's 3rd/Final 目標金額.
- [x] 2.3 Update `docs/overview.md`'s 目標 paragraph to mention the footnote expands both formula targets with live values; verify it matches the rendered note.

## 3. Verify

- [x] 3.1 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` all pass.
- [x] 3.2 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` passes.
- [x] 3.3 Restart the backend (`lsof -ti :8787 | xargs kill`, `cargo run -p wealth-backend --bin wealth-backend` in background) and confirm http://127.0.0.1:8787 shows the footnote whose inputs reproduce the 3rd/Final 目標金額.
