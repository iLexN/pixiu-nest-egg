# Design

## Context

`upsert` in `backend/src/routes/months.rs` already distinguishes absent vs explicit-null totals on create: `patch.total_assets` is `Option<Option<f64>>`, so `Some(None)` stores NULL (live) while `None` snapshots the live value. `MonthStatPatch` in `frontend/src/api.ts` already types both fields `number | null`. The 新增月份 form is the only caller that creates rows; import inserts `month_stats` directly with workbook values.

## Goals / Non-Goals

**Goals:**

- A month added from 新增月份 starts live (即時計算); no follow-up 改為即時 click.

**Non-Goals:**

- No change to the bare `PATCH` create contract (omitted totals still snapshot) or to `recapture`/`改為即時`.
- `start_cash` still stores NULL on create — it has no live mode (see archive `2026-09-22-recapture-start-cash`).

## Decisions

- **Frontend sends explicit nulls, backend untouched.** `addMonth` calls `api.patchMonth(ym, { total_assets: null, liquid_assets: null })`. Alternative considered: flip the create-branch default to NULL when totals are absent — rejected because it removes the create-and-freeze primitive (a bare PATCH would no longer be able to snapshot at creation) and rewrites more spec text for no caller benefit.

## Risks / Trade-offs

- A back-filled past month stores NULL totals and can never live-fill (`live_from()` gates live to ≥ the current month), so it shows `—` with no UI path to a value (recapture/改為即時 render only for current-or-later months). Accepted: a snapshot of today's portfolio in a past month is meaningless anyway, and history arrives via import, not this form.
