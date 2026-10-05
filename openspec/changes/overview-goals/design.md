# Design

## Context

The `Overview` sheet tracks four savings goals in two blocks: `J10:M20` (1st 目標 `K12` 1.55M, 2nd 目標 `K18` 2.5M — fixed amounts with derived remain/years/ETA) and `N13:P16` (3rd target `O14:O16`, Final Target `P14:P16` — pure formulas). Confirmed formulas:

- `K13` = `B3+B4+B9` (港股 + 債券 + IBKR)
- `L12` = `AVERAGE(G7*12, J22, M31*12, 'Month Stat'!L8*12, ('Month Stat'!L8+'Month Stat'!N8)*12)`; `M12` = `L12/12`
- `L13` = `K13*N4`; `N4` = `YearInReview!L13` — the latest completed year's `income_value_rate`
- `K14`/`K20` = target − current; `L14`/`L20` = remain ÷ `L12`; `J11`/`J17` = `TODAY() + 365*years`
- `O14` = `H6*1.07*12*(25+5)`; `O15` = `O14−H1`; `O16` = `O15/((L8+N8)*12)`
- `P14` = `E1*0.7*12/N4`; `P15` = `P14−B1`; `P16` = `P15/(C8*12)`

All inputs already exist in the app: `live_totals` (B1/H1), the asset rows (B3/B4/B9), `trailing_averages` (G7/H6), `invest_targets` (J22), `money_master` (M31), `month_running_averages` (MS row 8), and `year_review_rows` (the N4 rate).

## Goals / Non-Goals

**Goals:**
- The four goals derive live on every `GET /api/overview`; monthly upkeep is zero — the goals move with the stored data.
- The milestone amounts and the three formula constants stay editable, in the 月結 settings card alongside the other `overview.*` settings.
- Faithful sheet semantics: figures absent while inputs are missing or a divisor is zero; `AVERAGE` skips absent terms.

**Non-Goals:**
- The `K11`/`J10`-area text labels and the `P19:V36` long-term projection table (a separate unmigrated block).
- Storing goal history or auto-adjusting targets.
- Reproducing the sheet's negative-years display for reached goals — the app deliberately reports them as reached instead.

## Decisions

- **Pure `calc::overview_goals(&OverviewGoalsInput) -> OverviewGoals`.** Like `invest_targets`/`money_master`, the route resolves the inputs and the math is a pure function, so the sheet's cached values are unit-testable. `today` is an input so ETA tests are deterministic.
- **Five `app_meta` keys.** `overview.goal1_amount`/`overview.goal2_amount` hold the milestone targets (`null` blanks that milestone, matching "the setting is the cell"); `overview.goal3_inflation`/`overview.goal3_years`/`overview.final_replacement` carry defaults 1.07/30/0.7 so `null` resets to the sheet literal — the `semi_liquid_target` pattern (`MonthSettings` returns effective values, `MonthSettingsPatch` uses `Option<Option<f64>>`). No migration needed.
- **Seed `K12`/`K18` at import, insert-if-absent.** Same rule as `SALARY_KEY`: the settings exist so the user can move the goalposts; re-import must not clobber an edit.
- **`N4` = `income_value_rate` of `today.year() − 1`.** The sheet hardcodes `YearInReview!L13` (the latest completed year); the app resolves the same row dynamically. The app's rate drifts slightly from the sheet's cache (0.03866 vs 0.03852) — expected, informational in parity.
- **`remain ≤ 0` means reached.** The sheet would show negative years and a past ETA; the app reports `years`/`eta` absent and the card shows `已達成`. `remain` itself is still reported.
- **Parity: amounts deterministic, derivations informational.** `K12`/`K18` must match because import seeds from them; the rest legitimately diverge (live prices, date, fresher `N4`), so they report `Info` like the other live-linked overview cells.
- **Settings live in `PATCH /api/months/settings`.** Same card/pattern as the `money_master.*` fields; no new endpoints, so the OpenAPI pin test is untouched.

## Risks / Trade-offs

- [`(saved_avg + interest_avg)` when only one running average exists] → the combined term is absent unless both exist; documented in the delta spec's "absent terms skipped" rule. The sheet always has both once Month Stat has data.
- [`N4` absent early in a database's life] → `passive_return_*` and `final_goal.*` go absent; acceptable — the sheet errors the same way before a completed year exists.
- [Annual `N4` step] → every January the rate jumps to the newly completed year's; the card's caption names the year so the shift is explainable.
- [ETA is a rough `365 × years`] → faithful to the sheet's `TODAY()+365*L14`; leap-year drift is the sheet's own approximation.
