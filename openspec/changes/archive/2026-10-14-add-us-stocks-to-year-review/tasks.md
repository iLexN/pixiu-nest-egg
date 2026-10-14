# Tasks

## 1. Backend calc

- [x] 1.1 Extend `YearReviewInputs` (`us_years`, `usd_hkd_rate`), `YearReviewAssets` (five `us_stock_*` fields), `YearReviewInvestment` (`us_sold_pl`) in `backend/src/calc/year_review.rs`, and derive them in `year_review_rows` — blends add US figures × rate, absent when a year has US figures but no rate; verify with new/extended inline tests covering US derivation, converted blends, and the absent-rate case
- [x] 1.2 Fetch `us_years` (`service::yearly::build` `Market::Us`) and `usd_hkd_rate` (`meta_f64` `aia::RATE_KEY`) in both `service/year_review.rs::build` and `routes/year_review.rs::build`; verify `cargo test` passes

## 2. Frontend

- [x] 2.1 Add the new fields to `YearReviewAssets`/`YearReviewInvestment` in `api.ts`; add five `美股 …` rows to 資產 and `美股P/L` to 投資 in `YearReviewView.vue` with USD marking and a footnote about blends converting at the stored rate; verify `pnpm build && pnpm exec vue-tsc --noEmit`

## 3. Docs and spec sync

- [x] 3.1 Update `docs/overview.md` asset/investment group descriptions and the `year-in-review` main spec Purpose line (add 美股); verify wording matches the delta
- [x] 3.2 Run `openspec validate add-us-stocks-to-year-review`; restart the backend and confirm 年結 → 資產 shows 美股 rows on the live DB

## Workflow follow-up

- Archive the change after review.
