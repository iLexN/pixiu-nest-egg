# Proposal

## Why

The 年結 asset block only carries the HK stock figures; the US portfolio is invisible there except as a lump inside `invested`. The workbook also only had the HK column, but the app now tracks US cost, dividends, and year-end snapshots per market — surfacing them makes the review complete.

## What Changes

- Add a `美股` asset entry to each year row: `cost`, `dividends`, `rate`, `now_value`, `value_rate` derived from the US yearly table — reported in USD.
- Add `us_sold_pl` (美股P/L, USD, display-only) to the investment group; `sold_pl`/`net_investment` stay HK-only as before.
- The three blended `回報率` rates gain the US figures converted to HKD at the stored `aia.usd_hkd_rate`; a blend is absent when the year has contributing US figures but no rate is set.
- The 年結 page gains five `美股 …` rows in 資產 and a `美股P/L` row in 投資, marked USD.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `year-in-review`: adds the US asset entry and `us_sold_pl`, and blends US figures (HKD-converted) into the rates.

## Impact

- `backend/src/calc/year_review.rs` — new response fields, blend math, tests.
- `backend/src/service/year_review.rs`, `backend/src/routes/year_review.rs` — fetch US yearly rows + FX rate. Additive response fields only; no new endpoints.
- `frontend/src/api.ts`, `frontend/src/views/YearReviewView.vue` — new rows.
- `docs/overview.md` — asset/investment group descriptions.
- Behavior caveat: every year's blends convert US figures at the single stored FX rate (no per-year rate exists).
