# Tasks

## 1. Module + pilot

- [x] 1.1 Create `frontend/src/page.ts` — the Page lifecycle composable (`usePage()`: `data` / `message` / `error: ApiError | null` / `busy` state, `reload()`, `run(action, { message?, confirm?, reload? })` per design D1–D4) — verify `cd frontend && pnpm build` compiles it (vue-tsc)
- [x] 1.2 Pilot in `IbkrTransfersView.vue` (its `load`, and `remove` with confirm + `已刪除 …` message) — verify `pnpm build` and that the 刪除 confirm text, success message, and error line are byte-identical

## 2. Standalone views

- [x] 2.1 Adopt in `HappyPoolView`, `StocksView`, `MpfView` (load + write actions onto the page instance) — verify `pnpm build` per view
- [x] 2.2 Adopt in `DepositHistoryView`, `YearReviewView`, `DepositsView`, `DividendsView`, `TradesView` — verify `pnpm build` per view
- [x] 2.3 Adopt in `FamilyDepositsView` (load + `loadHistory`), `AiaView`, `BondsView` (7 actions) — verify `pnpm build` per view
- [x] 2.4 Adopt in `SummaryView` (price/note/year edits, reorder, freeze; upload preview state stays view-local; IbkrCard handler keeps `@saved="ibkr = $event"` with `reload: false` pattern) — verify `pnpm build`

## 3. Forms (own instances)

- [x] 3.1 Adopt in `DepositForm` + `DepositReceiveForm`: submit runs through the form's instance, `busy` gates `:disabled`, field chips render from the structured error, `saved: [entity]` emit unchanged — verify `pnpm build` and that a validation failure and a 400 both render exactly as before
- [x] 3.2 Adopt in `DividendForm`, `DividendReceiveForm`, `StockForm`, `TradeForm` — verify `pnpm build` per form
- [x] 3.3 Adopt in `IbkrCard` (form-scoped instance for save; keep `saved: [IbkrBlock]` emit; `OverviewView`'s `@saved="load"` unchanged) — verify `pnpm build`

## 4. Panels + their pages (same task, per design D5)

- [x] 4.1 `MonthStatView` + `MonthDetailPanel` + `MonthSettingsPanel`: page instance created in the view, `run` passed down as a prop; delete the `saved`/`failed` text emits (keep `deleted`); parent handlers removed — verify `pnpm build` (typed emits catch any missed handler)
- [x] 4.2 `OverviewView` + `ForecastGrid`: same handoff — verify `pnpm build`

## 5. Cleanup + live check

- [x] 5.1 Remove now-unused `errorMessage` from `api.ts` — the module landed on `toApiError` (design D2), leaving `errorMessage` with zero callers — verify `pnpm build` and `pnpm exec vue-tsc --noEmit`
- [x] 5.2 Restart the backend (it serves the built frontend) per AGENTS.md and spot-check against the live app: 月結 (accept/dismiss a Candidate, save 設定), 持倉總覽 (price edit + a form's field error), IBKR card save — every message, error line, and disabled-button state byte-identical

## Workflow follow-up

- Run `trail check` before committing; commit `GLOSSARY.md` and `.trail/learnings/2026-11-18-questions-go-one-at-a-time-ilexn.md` with this change
- Archive with `openspec archive` (skip_specs — nothing to sync)
