# Tasks

## 1. Backend: expose the formula inputs

- [x] 1.1 Add `interest: f64` and `pool_spend: f64` to `InvestTargetRow` in `backend/src/models.rs` with doc comments (year-review 利息回報 / 開心Pool支出, `0` when absent); verify `cargo build` compiles and the scalar OpenAPI docs show both fields.
- [x] 1.2 Populate them in `calc.rs::invest_targets` from `row.investment.interest` / `row.ledger.pool_spend`; verify with `cargo test invest_targets` after extending the existing test (1.3).
- [x] 1.3 Extend `invest_targets_derive_target_remain_growth_and_average` (or add a test) asserting `interest`/`pool_spend` pass through per row, including `0` for a year with no months; verify `cargo test invest_targets` passes.

## 2. Frontend: formula footnote on the 投資目標 card

- [x] 2.1 Add `interest: number` and `pool_spend: number` to `InvestTargetRow` in `frontend/src/api.ts`; verify `pnpm exec vue-tsc --noEmit` still passes.
- [x] 2.2 In `OverviewView.vue` add computeds locating the current-calendar-year row and the `year − 1` row from `data.invest_targets.rows`; verify they return `undefined` cleanly when absent.
- [x] 2.3 Add the `<p class="muted">` footnote under the card's table rendering `目標 = 上年invested (…) − 上年利息回報 (…) − 上年Pool支出×0.7 (…) + 月薪增幅×50%*12 (…) + 今年利息回報 (…) + 今年Pool支出×0.7 (…)` — each paren the term's `fmtMoney` contribution (`(raise ?? 0) * 6` for the raise term, `× 0.7` applied to pool spends), `—` for absent inputs, hidden while no current-year row exists; verify in the browser against the card's 目標 figure.
- [x] 2.4 Add a sentence to `docs/overview.md`'s 投資目標 paragraph noting the footnote expands the current-year target's terms with live values; verify the sentence matches the rendered note.

## 3. Verify

- [x] 3.1 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` all pass.
- [x] 3.2 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` passes.
- [x] 3.3 Restart the backend (`lsof -ti :8787 | xargs kill`, `cargo run -p wealth-backend --bin wealth-backend` in background) and confirm http://127.0.0.1:8787 shows the footnote whose six values sum to the current row's 目標.
