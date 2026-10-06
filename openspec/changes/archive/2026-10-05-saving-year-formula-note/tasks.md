# Tasks

## 1. Backend: echo the running averages

- [x] 1.1 Add `saved_avg: Option<f64>` and `interest_avg: Option<f64>` to `OverviewGoals` in `backend/src/models.rs` with doc comments ('Month Stat' L8/N8 running averages); verify `cargo build` compiles and the scalar OpenAPI docs show both.
- [x] 1.2 Populate them in `calc.rs::overview_goals` from `input.saved_avg`/`input.interest_avg`; verify with `cargo test overview_goals`.
- [x] 1.3 Extend an `overview_goals` test asserting the response echoes `saved_avg`/`interest_avg` (present and absent cases); verify `cargo test overview_goals` passes.

## 2. Frontend: 存/年 footnote on the 目標 card

- [x] 2.1 Add `saved_avg: number | null` and `interest_avg: number | null` to `OverviewGoals` in `frontend/src/api.ts`; verify `pnpm exec vue-tsc --noEmit` still passes.
- [x] 2.2 Add a `savingYearTerms` computed in `OverviewView.vue` producing the five yearly contributions (`averages.saved × 12`, `invest_targets.avg_invested`, `money_master.avg_per_month × 12`, `saved_avg × 12`, `(saved_avg + interest_avg) × 12`), `null` per absent term; verify it compiles.
- [x] 2.3 Add a third muted footnote line using the `note-formula`/`note-line` layout: `存/年 = AVERAGE(` with terms `近12月存×12`, `近3年invested`, `MM月均×12`, `歷年存×12`, `(歷年存+息)×12`, each contribution `fmtMoney`-formatted, `—` while absent; verify in the browser that `AVERAGE` of the shown values equals the 存/年 footer figure.
- [x] 2.4 Update `docs/overview.md`'s 目標 paragraph to mention the footnote also expands 存/年 into its five terms; verify it matches the rendered note.

## 3. Verify

- [x] 3.1 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` all pass.
- [x] 3.2 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` passes.
- [x] 3.3 Restart the backend (`lsof -ti :8787 | xargs kill`, `cargo run -p wealth-backend --bin wealth-backend` in background) and confirm http://127.0.0.1:8787 shows the 存/年 expansion whose mean equals the footer figure.
