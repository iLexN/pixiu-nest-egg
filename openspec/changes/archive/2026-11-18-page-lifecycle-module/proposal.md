# Proposal

## Why

The frontend grew one view at a time, and each hand-rolled the same page lifecycle — `error`/`message` refs, a `load()` function, and the confirm → call → message → reload → catch dance around every write action. That protocol is now copied 40 times across 22 view/form/panel modules and has drifted into two error idioms (views: `error = ref('')` + `errorMessage(err)`; forms: `error = ref<ApiError | null>` + `toApiError(err)`) and three parent↔child protocols (panels emit display text on `saved`/`failed`; forms emit the saved entity; IbkrCard mixes both). Nothing owns the protocol, so changing it means editing 20+ files, and every new page copies it again.

## What Changes

- **New module `frontend/src/page.ts`** — the **Page lifecycle** (GLOSSARY term, frontend counterpart of the Page read model): a composable owning one scope's data, success message, structured error state (`ApiError`, one storage), reload-after-write, and confirm-before-delete. One instance per page, one per form.
- **Views adopt it** — each view's `load()`, `error`/`message` refs, and every write action's try/catch/confirm/reload collapse onto the module; views keep only domain wiring (drafts, selection, computed state).
- **Panels call the page's runner** — MonthDetailPanel, MonthSettingsPanel, ForecastGrid stop emitting `saved`/`failed` display text (33 + 29 emit sites); domain events like the 月結 panel's `deleted` remain.
- **Forms keep `saved`-with-entity emits** — a form's submit runs through its own instance; field chips render from the same structured storage; parents still choose the success message.
- **Display is byte-identical** — same messages, same error lines, same field chips, same empty states. Reload is the default after a successful write, with an opt-out for the two IbkrCard local-patch sites (e.g. SummaryView's `@saved="ibkr = $event"`). No loading flag ships — nothing renders one today.
- **Local validation throws inside the wrapped action** (`金額必須是數字`, `名稱及金額必填`, …) — any throw lands in the error state via today's `toApiError` semantics; the module is the single writer of its own state.
- **No test runner** — vue-tsc and `pnpm build` remain the only frontend checks; a later runner would test the module through the same interface.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — no requirement changes; every display outcome is byte-identical; `skip_specs: true` is set.

## Impact

- New `frontend/src/page.ts`
- All 15 views in `frontend/src/views/`
- `frontend/src/components/`: DepositForm, DepositReceiveForm, DividendForm, DividendReceiveForm, StockForm, TradeForm, IbkrCard, MonthDetailPanel, MonthSettingsPanel, ForecastGrid
- `frontend/src/api.ts` unchanged (`api`, `ApiError`, `toApiError`, `errorMessage` stay; views stop importing `errorMessage` directly)
- `GLOSSARY.md` — Frontend structure: Page lifecycle (staged in the working tree with this change)
