# Tasks

## 1. Baseline

- [x] 1.1 Before any code change, capture `GET /api/overview` from the running backend into `/tmp/footnote-baseline/overview.json` and verify it is valid JSON (`python3 -m json.tool`).

## 2. 投資目標 terms

- [x] 2.1 In `backend/src/models/overview.rs`, add `InvestTargetTerms` (`ToSchema`, field docs) and `InvestTargetRow.terms`. In `calc::invest_targets`, build the terms first and derive `target` from them, keeping the operand order (design D1). Verify the existing `invest_targets` tests still pass (`cargo test calc::year_review`).
- [x] 2.2 Add calc tests: the spec's 2025/2026 scenario (terms ≈ `428635.41`, `51900.65`, `27110.65`, `11340`, `62753.15`, `6715.1` and they sum to `target`); the earliest row (prior terms absent, `raise_term` `0`, `target` absent). Verify with `cargo test calc::year_review`.

## 3. Goal terms and pace

- [x] 3.1 In `backend/src/models/overview.rs`, add `SavingYearTerms`, `OverviewGoals.saving_terms` and `FormulaGoal.pace`. In `calc::overview_goals`, build `SavingYearTerms` first and average its present fields in the existing order; have `formula_goal` store `pace` (design D2). Verify the existing `overview_goals` tests still pass (`cargo test calc::overview`).
- [x] 3.2 Add calc tests: the spec's saving-terms scenario (five terms ≈ `360159.24`, `326841.22`, `364801.69`, `316354.11`, `370124.90`; `saving_year` their mean); absent money-master → `mm_year` absent; `third.pace` == `running_combined_year`; `final_goal.pace` ≈ `719017.86`. Verify with `cargo test calc::overview`, then run `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`.

## 4. Frontend

- [x] 4.1 Run `cd frontend && pnpm gen:api`; add `api.ts` aliases only if the view names the new types (existing alias pattern). Verify `api-types.ts` has `InvestTargetTerms`, `SavingYearTerms`, and `pace` on `FormulaGoal`.
- [x] 4.2 In `OverviewView.vue`, read `terms`, `saving_terms`, and `pace` per design D3; delete `targetNotePrior` and every `× 0.7`/`× 6`/`× 12` in `<script>`. Verify with `pnpm build && pnpm exec vue-tsc --noEmit`, and confirm `grep -nE "\* ?(0\.7|6|12)\b" frontend/src/views/OverviewView.vue` finds no script matches.

## 5. Docs

- [x] 5.1 Update `docs/overview.md` (the 投資目標 and 目標 footnote descriptions: the terms and paces come from the response). Verify that the doc's footnote paragraphs name `terms`, `saving_terms` and `pace`.

## 6. Integration check

- [x] 6.1 Restart the backend per AGENTS.md, re-capture `/api/overview` to `/tmp/footnote-after/overview.json`, and verify every pre-existing field is identical to the baseline (only `terms`, `saving_terms` and `pace` added). Confirm the current-year `terms` sum to that row's `target` and that each formula goal's `pace` matches the old frontend formula on the captured data.

## Workflow follow-up

- Run `trail check` before committing.
- Archive after review (`openspec archive overview-footnote-terms-from-backend`), syncing the `overview` delta.
