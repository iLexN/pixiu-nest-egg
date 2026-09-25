# Design

## Context

`Overview` `J22:N27` is a per-year invested-vs-target table. `K` links to `YearInReview`'s `invested` cells; `L` (target) is a different formula per year — 2024 is a literal, 2025 omits the entertainment term, 2026 reads `=(K25-F10)+(1890*0.5*12)+F19+(C25*70%)`. `M` is target-minus-invested on the current year only; `N` is invested YoY for completed years and target-vs-last-invested for the current year; `J22` averages the three completed years' `K`. The `J27` note states the intended formula. The app already derives every input: `year_review_rows` produces `invested`, `interest`, `pool_spend` per year, and `month_stats.salary` records the actual salary history (April step-ups: 45500→47850→50810→52700).

## Goals / Non-Goals

**Goals:**
- One canonical target formula, retro-computed uniformly; the sheet's per-year variants are deliberately superseded.
- `raise` auto-derived from stored month salaries, with a manual override escape hatch.
- Reuse `year_review_rows` output; no parallel aggregation.

**Non-Goals:**
- Reproducing the sheet's historical targets (2024's literal, 2025's variant) — history is recomputed under the unified formula and parity reports those cells informational.
- Salary history management — the existing month-row `salary` values are the input; no new ledger.
- Compounding/exactness in the "embedded match" strip — `invested` is actual net trades, so `−0.7×pool_spend(Y-1)` is a heuristic correction, as intended.

## Decisions

**Target formula:** `target(Y) = invested(Y-1) − interest(Y-1) − pool_spend(Y-1)×0.7 + raise(Y)×6 + interest(Y) + pool_spend(Y)×0.7`. The two pool terms net to `0.7×(pool(Y) − pool(Y-1))` — the fun tax charges only the YoY *increase* in entertainment spending, because last year's match is already embedded in `invested(Y-1)` and stripping it stops the charge compounding in the baseline. Chosen over the sheet's same-year `pool_spend(Y)×0.7` (target inflates mid-year with each fun purchase and double-hits the year's savings) and over a fully lagged `pool_spend(Y-1)` term (which the user rejected as losing the same-year feedback). A negative delta is allowed — spending less on fun lowers the target.

**`raise` = stored override else salary-derived.** `year_review.raise` (nullable REAL) is the manual override, patched like `income`. While NULL, `raise(Y) = max(0, lastSalary(Y) − lastSalary(Y-1))` where `lastSalary(y)` is the salary on the year's latest month row carrying one. Verified against the real DB: it reproduces the sheet's hand-typed 2960 (2025) and 1890 (2026) exactly, so no seeding is needed and an `overview.salary` change propagates automatically once new payday rows record it. Alternatives considered: storing raise% (needs the same salary history anyway); always-manual (forgets to update → silent understatement — rejected after the user asked for the auto behavior). Derivation clamps at 0; the override accepts negative values deliberately.

**Block assembly reuses the year-review build.** `overview.rs` calls `year_review::build(&pool)` and maps its rows into `invest_targets` — the year set, `invested`, `interest`, `pool_spend`, and `raise` all come from the same source `GET /api/year-review` exposes. `YearReviewRow` gains `raise` (effective value; NULL-able distinction between stored/derived is not needed downstream — the PATCH's null-clear semantics already handle it). `year_review_rows` gains a `last_salaries: &BTreeMap<i32, f64>` input loaded in `build` from `month_stats` (latest month per year with non-NULL salary). Alternative — computing targets inside `year_review_rows`: rejected; the block belongs to the Overview sheet/endpoint, and keeping it a pure projection of the year rows keeps each endpoint's spec focused.

**2023 seed via the Overview `J:K` cells.** Import parses `J23:J26`/`K23:K26` and seeds `invested_adjustment = K − hk_net_invested(Y) − Σ transfers(Y)` for each listed year — identical formula to the `YearInReview` seeding, so covered years compute the same value and the `ON CONFLICT … COALESCE(existing, …)` no-overwrite convention makes re-imports idempotent. Only 2023 gains anything new (`+110000`), which both fills the `K23` row and enables 2024's `growth`. Alternative — a hardcoded 110000 seed: rejected; parsing the cells is the same code path and stays correct if the sheet values change.

**Remain/growth asymmetry matches the sheet.** `remain` exists only on the current-year row (the sheet only fills `M26`); `growth` is invested-YoY for completed years (`N24`/`N25`) but target-vs-prior-invested for the current year (`N26`). `J22` → `avg_invested` is the mean over the last three *completed* years, skipping absent `invested` — matching `AVERAGE` semantics.

## Risks / Trade-offs

- Early-year `remain` is optimistic: `pool_spend(Y)` is YTD, so the delta term starts near `−0.7×pool(Y-1)` and the target looks easy until entertainment spending accumulates → Documented behavior; the figure self-corrects through the year.
- `raise` derivation trusts month-row salaries; a mistyped month salary skews the target → the manual override covers corrections; salary edits are already part of the normal payday flow.
- Record-only years (no month rows) derive `interest`/`pool_spend` as 0 → retro targets for sparse years are approximations — accepted by design.
- Existing databases only gain the 2023 seed on the next `import_xlsx` run → noted in tasks/docs; without it, 2023 shows no `invested` and 2024 shows no `growth`.

## Migration Plan

- `0023_year_review_raise.sql`: `ALTER TABLE year_review ADD COLUMN raise REAL` — additive, no backfill needed (NULL = derive).
- Re-run `import_xlsx` to seed the 2023 `invested_adjustment`; idempotent.
- Rollback: drop the column and revert the response fields; no stored data is irreversibly transformed.
