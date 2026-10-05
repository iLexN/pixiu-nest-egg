# Tasks

## 1. Derivation (calc.rs)

- [x] 1.1 Add `bank: Option<String>` to `SuggestionDeposit`, load it from `deposits.bank` in `months::load_interest_events`, and update every constructor; verify `cargo test` still passes
- [x] 1.2 Add `forecast_locked_anchor(deposits, first_month)` (`start_date` none or `< first_month`, `end_date >= first_month`) with tests covering: NULL start included, prior-month start included, start on the 1st excluded, start later in the month excluded, `end_date` inside the first month included, end before the first month excluded, and the spec's `300000` scenario; verify the tests pass
- [x] 1.3 In `forecast_months`, add the placement flow `deposit_start` (Σ principal of deposits with `start_date` in the month, `+ 0.0`): cash −, locked +. Expose it on `ForecastMonthDerived` and update the doc comments for `locked_base`/`cash`/`locked`. Tests: a `55000` deposit starting `2026-09-30` lowers Sep `cash` by `55000` and raises `locked` by `55000`, and ending `2027-01-05` gives Jan `deposit_finish` `55000`. An equivalence test compares plan `sc_deposit -55000` in Sep (return `2027-01`) plus anchor `445000` against the converted deposit (start `2026-09-30`, end `2027-01-05`, interest `0`) plus anchor `500000 − 55000 = 445000` with no plan: `cash`/`locked`/`semi_liquid`/`ref_check` are equal in every column. Existing forecast tests pass unchanged

## 2. API (routes/forecast.rs, models.rs)

- [x] 2.1 Pass `calc::forecast_locked_anchor(&events.deposits, first)` as `locked_base` instead of `input.deposits_active_principal`; verify via a test or a scratch-DB GET that the overview `半流動資金` block is unchanged
- [x] 2.2 Add `ForecastPlacement { deposit_id, label, bank, principal, start_date }` (`ToSchema`) and `ForecastMonth.deposit_placements`, filled per month from `events.deposits` whose `start_date` is in the month; update the `ForecastMonth` doc comments. Verify with `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`

## 3. Frontend (api.ts, OverviewView.vue)

- [x] 3.1 Add the `ForecastPlacement` type and the `deposit_placements` field in `api.ts`
- [x] 3.2 In `OverviewView.vue`, map placements to rows (HS → `hs_deposit`, else → `sc_deposit`), include `−principal` in the cell sum (null only when there are no items and no placements), and list placements read-only (label, −principal, start_date, `已定期`) in the cell editor with no actions. Verify with `pnpm build && pnpm exec vue-tsc --noEmit`

## 4. Docs

- [x] 4.1 Update the 預測 section of `docs/overview.md`: the month-start anchor rule, the placement flow and dating rule ("start_date = the day money left 活期"), the bank-row display, 轉為定期 leaving totals unchanged, and the fallback-start limitation; verify the text matches the delta spec

## 5. Integration and data fix

- [x] 5.1 Restart the backend on :8787 and confirm `GET /api/forecast` answers
- [x] 5.2 With the user's confirmation, `PATCH /api/deposits/23 {"start_date":"2026-08-31"}`, then verify `GET /api/forecast`: 2026-09 `locked` `500000`, `cash` ≈ `20072.34`, and `deposit_placements` lists `9077`; the UI's 2026-09 `SC高息馬拉松` cell shows `-55000`
