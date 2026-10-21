# Tasks

## 1. Backend calc

- [x] 1.1 Add `us_stock_cost_hkd` and `us_stock_now_value_hkd` (`Option<f64>`) to `YearReviewAssets` in `backend/src/calc/year_review.rs` and derive them as the USD figure × `usd_hkd_rate`, absent without either input; extend the US test to cover the converted figures and the no-rate case; verify `cargo test` passes

## 2. Frontend

- [x] 2.1 Add the two fields to `YearReviewAssets` in `frontend/src/api.ts`; add a sub-line mechanism to `MetricRow` in `YearReviewView.vue` and render the HKD figure under the USD value on 美股 成本 and 美股 市值; verify `pnpm build && pnpm exec vue-tsc --noEmit`

## 3. Docs and spec sync

- [x] 3.1 Update the `docs/overview.md` asset-group description with the HKD sub-lines; verify `openspec validate add-us-stock-hkd-subvalues` passes; restart the backend and confirm the live 年結 page shows the HKD sub-lines

## Workflow follow-up

- Archive the change after review.
