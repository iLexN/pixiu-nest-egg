# Tasks

## 1. Shared helpers

- [x] 1.1 Create `frontend/src/parse.ts` exporting `numOrNull`; replace the copies in `MonthStatView`, `OverviewView`, `BondsView`, `AiaView`, `FamilyDepositsView`, `DepositReceiveForm` and the inlined parses in `YearReviewView`/`OverviewView`.
- [x] 1.2 Add `errorMessage(err)` and `toApiError(err)` to `frontend/src/api.ts`; rewrite every `err instanceof ApiError ? err.message : String(err)` catch body and the 3 re-wrap sites in `DepositForm`/`DividendForm`.
- [x] 1.3 Add `fmtMonth` and `shiftYearIso` to `frontend/src/format.ts`; update `MonthStatView`/`AiaView` (incl. `defaultNextPay`).
- [x] 1.4 Create `components/FieldError.vue` (props `error`, `field`); swap into the ~45 `<small v-if="error?.fieldMessage(...)">` sites in `DepositForm`, `DividendForm`, `TradeForm`, `DividendReceiveForm`, `StockForm`.

## 2. Verification

- [x] 2.1 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — clean.
- [x] 2.2 `cargo test` — all pass (backend untouched, sanity check).

## Workflow follow-up

- Archive the change (`openspec archive`) and sync delta specs into `openspec/specs/`.
