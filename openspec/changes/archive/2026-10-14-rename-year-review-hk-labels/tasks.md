# Tasks

## 1. Relabel 年結 資產 stock rows

- [x] 1.1 Rename the five asset-row labels in `frontend/src/views/YearReviewView.vue` (股票 成本/派息/率/市值/市值率 → 港股 …); verify `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` passes and the live page shows 港股 rows
- [x] 1.2 Update the `YearReviewAssets` doc comment in `backend/src/calc/year_review.rs` and the asset-group wording in `docs/overview.md` to say 港股; verify no stray `股票` remains in the 資產 section wording

## 2. Spec wording sync

- [x] 2.1 Update the `year-in-review` main spec Purpose line (債券/股票/定期 → 債券/港股/定期) — Purpose edits are made directly on `openspec/specs/year-in-review/spec.md`
- [x] 2.2 Delta spec restates "Per-asset-class returns" and "Year review page" with the 港股 naming; verify `openspec validate rename-year-review-hk-labels` passes

## Workflow follow-up

- Archive the change so the MODIFIED requirements merge into `openspec/specs/year-in-review/spec.md`.
