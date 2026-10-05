# Design

## Context

`overview_goals` (`backend/src/calc.rs`) already receives the effective constants via `OverviewGoalsInput` (`goal3_inflation`/`goal3_years`/`final_replacement`, defaults applied by the caller in `routes/months.rs::goal_constants`), but `OverviewGoals` doesn't echo them. The remaining inputs — `averages.living_budget`, `salary`, `n4_rate` — are already on the response.

## Goals / Non-Goals

- Goal: the 目標 card footnote expands both formula targets (`O14`/`P14`) with live values, matching the 投資目標 footnote's style.
- Non-goal: expanding the `remain`/`years` denominators (pace terms) — the footnote covers the `target` formulas only, which is where the constants live.

## Decisions

- **Echo the three constants on `OverviewGoals` as plain `f64`** rather than having `OverviewView` fetch `GET /api/months/settings`.
  - vs. fetching settings: a second request and a month-scoped endpoint for three values already in scope during the overview derivation; the settings endpoint also returns far more than needed.
  - Plain `f64` (never absent) matches their semantics — the defaults always apply — same as `semi_liquid_target` on `OverviewResponse`, which already echoes an effective setting this way.
- **Labels follow the card/settings vocabulary**: `生活預算` (averages card), `薪金` (settings form), `通脹`/`年期` for the 3rd constants, `替代率` for `final_replacement`; `n4_rate` reuses the existing `N4` caption. `final_replacement` renders via `fmtPercentShort` (70%), `n4_rate` via `fmtPercent` (3.852%) — matching how the same values display in settings and the existing footnote.
- **One footnote line joining both formulas with `·`**, appended to the existing muted footnote paragraph block inside the card; always rendered (the constants never absent), `—` for absent inputs.

## Risks / Trade-offs

- [The footnote grows to a second line] → consistent with the 投資目標 card, which already wraps.
- [`替代率`/`通脹`/`年期` are new UI labels not used elsewhere] → kept parenthesized so the raw figures stay primary; the settings row already calls them `3rd ×1.07/30年` and `Final 70%`.

## Migration Plan

Additive response fields only — no migration, no client breakage.
