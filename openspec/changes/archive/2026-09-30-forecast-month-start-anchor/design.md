# Design

## Context

`forecast_months` (calc.rs) receives `start` (a month-start snapshot) and `locked_base` (currently `deposits_active_principal`, meaning `end_date > today` regardless of `start_date`). It chains both through the window using plan items, returns, and `deposit_finish`. `input.deposits` (`SuggestionDeposit`, loaded by `months::load_interest_events`) already carries every deposit with `start_date`, `end_date`, `principal`, `interest`, and `label`, but no `bank`. See proposal.md for the failure this causes.

## Goals / Non-Goals

**Goals:**
- Both the cash anchor and the locked anchor represent the first day of the current month. Everything that happens after that is a flow in the window.
- `轉為定期` of a plan whose deposit dates match the plan leaves the totals unchanged (spec scenario "Conversion keeps the grid totals").

**Non-Goals:**
- Changing the overview `半流動資金` block, live totals, or `deposits_active_principal`.
- Fixing the fallback `start` (live 活期 when `start_cash` is unset). It is a "now" figure. Finishes already carry the same limitation; it is documented, not solved.
- Auto-detecting stale plans that a real deposit made without conversion now duplicates. Stale-plan handling stays as specified.

## Decisions

- **Anchor helper in calc, input field kept.** Add `pub fn forecast_locked_anchor(deposits: &[SuggestionDeposit], first_month: NaiveDate) -> f64` (principal where `start_date.map_or(true, |d| d < first_month) && end_date >= first_month`). The route passes its result as `ForecastInput.locked_base`. Keeping the field (rather than deriving the anchor inside `forecast_months`) leaves the existing tests' `445000` fixture valid and keeps the anchor unit-testable on its own. Alternative considered: derive the anchor inside `forecast_months` from `deposits`. Rejected because it rewrites every forecast test fixture for no behavioral gain.
- **`end_date >= first_month`, not `> today`.** A deposit maturing between the 1st and today stays in the anchor, so the existing `deposit_finish` subtraction removes it exactly once.
- **Every `start_date` in the month is a placement, including the 1st.** The rule is "start_date = the day the money left 活期". Money that left before the 月初 figure is dated in the prior month (hence the SC-3615 data fix to 2026-08-31).
- **Placement flow in `forecast_months`.** Per window month: `deposit_start = Σ principal of deposits with start_date in m` (`+ 0.0` to avoid `-0.0`). `cash -= deposit_start`, `locked += deposit_start`. Add `deposit_start: f64` to `ForecastMonthDerived`. It is internal. The API exposes the list rather than the sum, and the frontend folds it into the bank rows.
- **Bank on `SuggestionDeposit`.** Add `bank: Option<String>`, loaded from `deposits.bank`, and update every constructor (loader + tests). Used only for the placement list.
- **API shape.** `ForecastMonth.deposit_placements: Vec<ForecastPlacement>` with `deposit_id: i64`, `label: Option<String>`, `bank: Option<String>`, `principal: f64`, `start_date: String`, derived in the route from `events.deposits` for each month. `ForecastPlacement` derives `ToSchema`. This is additive, so the endpoint list in the openapi test is unchanged.
- **Frontend row mapping.** `bank === 'HS'` → `hs_deposit` row, everything else → `sc_deposit` row. The cell sum becomes `Σ plan items + Σ −principal of mapped placements`. It is `null` only when both lists are empty. The editor shows placements above the plan items as read-only rows tagged `已定期`, with no actions.

## Risks / Trade-offs

- [If `start_cash` is entered after a deposit dated inside the month, that deposit is subtracted twice] → Document the dating rule in docs/overview.md. The user fixes the date on the deposit.
- [A real deposit created without converting its plan now counts both the plan and the placement in the same month (previously it was double-counted via `locked` instead)] → Unchanged guidance: create deposits via `轉為定期`. The plan stays visible in the same cell next to the placement, so the duplicate is easy to spot.
- [Deposits with no `start_date` and `end_date` in the window count in the anchor even if they were actually placed mid-month] → Acceptable. Without a date there is no flow to place, and the behavior matches today's.

## Migration Plan

No schema change. After deploying: `PATCH /api/deposits/23 {"start_date":"2026-08-31"}` on the live DB (with the user's confirmation). Rollback = revert the commit. The data fix is harmless under the old code (the old code ignores `start_date` for the anchor).
