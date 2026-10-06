# Design

## Context

`saving_year` (`overview_goals` in `backend/src/calc.rs`) averages five yearly terms, skipping absent ones. Three are already on the response (`averages.saved`, `invest_targets.avg_invested`, `money_master.avg_per_month`); the other two are the running `saved_avg`/`interest_avg` from `month_running_averages`, currently internal to `OverviewGoalsInput`.

## Goals / Non-Goals

- Goal: the 目標 card footnote expands `存/年` into its five terms with live contributions, in the same `note-formula`/`note-line` layout as the 投資目標 footnote.
- Non-goal: expanding `passive_return_*` (`current_invest × N4` — already self-evident from the footer figures) or the `years`/`eta` paces.

## Decisions

- **Echo `saved_avg`/`interest_avg` as `Option<f64>` on `OverviewGoals`** rather than fetching the month summary or pre-computing the five terms server-side.
  - vs. a terms array: each term's contribution is trivially composable in the view (`×12`), and echoing raw inputs matches the pattern set by `interest`/`pool_spend` on `InvestTargetRow` and the constants on `OverviewGoals`.
  - `Option` (not `f64`) because the running averages genuinely absent while no qualifying month exists — unlike the constants, there is no default.
- **Footnote rendering**: a third muted line, `存/年 = AVERAGE(` label with the five contributions stacked via `.note-line`, `—` for absent terms. Labels reuse existing UI vocabulary: `近12月存` (過去 12 個月平均 card), `近3年invested` (投資目標 card), `MM月均` (Money Master card), `歷年存`/`歷年存+息` (月結 page's 歷年平均 grid).

## Risks / Trade-offs

- [Two of the five labels are new shorthand (`歷年存`, `MM月均`)] → kept terse to fit the muted line; each names its source card.

## Migration Plan

Additive response fields only — no migration, no client breakage.
