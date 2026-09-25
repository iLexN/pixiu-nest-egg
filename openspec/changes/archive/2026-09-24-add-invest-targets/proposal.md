# Proposal

## Why

The `Overview` sheet's `J22:N27` block tracks a per-year investment target — how much should have been invested versus what actually was — and it is still maintained by hand. Every input already derives inside the app (`invested`, 利息回報, 開心Pool支出, and per-month salary history); the sheet's own per-year formulas diverge (2024 is a literal, 2025 lacks the entertainment term), so a single canonical formula replaces them rather than migrating history verbatim.

## What Changes

- `GET /api/overview` gains an `invest_targets` block mirroring `J22:N27`: `avg_invested` = the mean of `invested` over the last three completed years (the sheet's `J22` = `AVERAGE(K23:K25)`), plus one row per year carrying `invested` (`K`), the effective `raise`, `target` (`L`), `remain` (`M`, current year only), and `growth` (`N`).
- One canonical target formula, retro-computed for every year with prior-year data (historical targets intentionally differ from the sheet's per-year variants): `target(Y) = invested(Y-1) − interest(Y-1) − pool_spend(Y-1)×0.7 + raise(Y)×0.5×12 + interest(Y) + pool_spend(Y)×0.7` — i.e. last year's organic invested, plus half the year's salary raise annualized, plus this year's returns reinvested, plus 70% of the YoY *increase* in entertainment spend.
- `year_review` gains a nullable `raise` column: a manual override editable via `PATCH /api/year-review/:year`; while NULL it derives live as `max(0, last salary(Y) − last salary(Y-1))` from stored month salaries — the existing history reproduces the sheet's hand-typed constants (2960, 1890) exactly, so no seeding is needed and changing `overview.salary` flows through automatically once new month rows record it.
- Import additionally seeds `invested_adjustment` from the Overview block's `K`-column invested cells for years without a `YearInReview` block — the 2023 row gains `+110000`, reproducing the sheet's `206523.15` and enabling 2024's `growth`.
- Parity compares the `K`-column invested cells, the `J22` average, and completed-year `growth` cells normally; `target`/`remain` and the current-year `growth` report informational — the sheet's per-year formulas legitimately differ.
- The 總覽 page gains an 投資目標 card (average header + per-year table); the 回顧 page exposes the effective `raise` per year with optional override editing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `overview`: `GET /api/overview` additionally returns the `invest_targets` block (rows, formula, average), and the 總覽 page shows the 投資目標 card.
- `year-in-review`: `raise` joins the stored per-year manual figures with a live derivation fallback; `PATCH /api/year-review/:year` and the 回顧 page accept/show it; import also seeds `invested_adjustment` from the Overview `J:K` year/invested cells for years the `YearInReview` sheet doesn't cover.

## Impact

- `backend/migrations/0023_year_review_raise.sql` — `year_review.raise` column.
- `backend/src/models.rs` — `raise` on `YearReviewPatch`/`YearReviewRecord`; `InvestTargetRow`/`InvestTargets` on `OverviewResponse`.
- `backend/src/calc.rs` — raise derivation + target formula over `year_review_rows` output.
- `backend/src/routes/overview.rs` — assemble `invest_targets` reusing the year-review build path.
- `backend/src/routes/year_review.rs` — accept `raise` in the PATCH.
- `backend/src/xlsx.rs` — read the `J22:N27` cells into `OverviewCached`.
- `backend/src/import.rs` — seed `invested_adjustment` for uncovered years (2023).
- `backend/src/parity.rs` — compare the block in `check_overview`.
- `frontend/src/api.ts`, `views/OverviewView.vue` (投資目標 card), `views/YearReviewView.vue` (raise display/override editing).
- `docs/DATA_FLOW.md`, `AGENTS.md`.
