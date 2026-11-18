# Design

## Context

See proposal.md — Why. Today's frontend has no shared owner of the page lifecycle: 22 view/form/panel modules hand-roll `error`/`message` refs (17 `load()` functions, 40 write actions repeating confirm → call → message → reload → catch, 19 `window.confirm` sites), in two error idioms and three parent↔child protocols. `api.ts` (`request`, `ApiError`, `toApiError`, `errorMessage`) is already a deep, stable seam and stays untouched. The GLOSSARY now defines **Page lifecycle** (frontend counterpart of the Page read model).

## Goals / Non-Goals

**Goals:**

- One module (`frontend/src/page.ts`) owns the load–mutate–report protocol; views/forms/panels keep only domain wiring.
- Every display outcome byte-identical: messages, error lines, field chips, disabled buttons, empty states, reload behavior.
- Adoption across all ~25 modules in this one change, pilot-first.

**Non-Goals:**

- No test runner (vue-tsc + `pnpm build` remain the checks); no new UX (no toasts, no spinners, no page-level loading state).
- No change to `api.ts`, to the HTTP surface, or to any backend behavior.
- No per-page specialization (e.g. a month-page context) — only the generic lifecycle.

## Decisions

### D1 — One composable, two scopes

`usePage()` creates an instance owning `{ data, message, error, busy }` state plus `reload()` and `run(action, opts)`. A page creates one instance for its loader and write actions; a form creates its own for `submit` — the form's field chips render from its instance's structured error, so a failed submit never surfaces on the page error line. Alternatives rejected: one page-wide instance shared into forms (a failed form submit would land on the page error line — changes display); panels keeping their own instances (their writes must reload the page, which only the page instance can do).

### D2 — Structured error, one storage, two projections

The instance stores the error as `ApiError` (via `toApiError` semantics: any thrown `Error` becomes status-0, no fields). The page error line renders `error.message` (today's `errorMessage(err)` output); form field chips render `error.fields` through the existing `FieldError` component. Alternatives rejected: string storage (loses field chips); parallel string + structured state (the drift re-enters inside the module).

### D3 — `run` owns the mutation protocol

`run(action, { message?, confirm?, reload? })` wraps a write action: optional `window.confirm(confirm)` first (declined → no-op), then the action; a local validation failure throws inside the action and lands in the error state like any failure; on success, set `message`, clear `error`, then reload the loader unless `reload: false`. Defaults: reload on (38 of 40 call sites reload today); opt-out for the two IbkrCard local-patch sites (SummaryView's `@saved="ibkr = $event"`, OverviewView keeps its reload). Alternatives rejected: never reload (protocol stays split); always reload, no opt-out (changes SummaryView's IbkrCard to an extra fetch).

One documented micro-deviation from byte-identical, surfaced during implementation: `run` clears the success message when an action starts (StocksView's idiom). Views that did not clear it (e.g. BondsView) previously let a stale success message linger beside a new error on failure — the same drift shape as the two error idioms, unified in the cleaner direction. Failure-path only; revert is one line if vetoed.

### D4 — `busy` flag is in, page loading is out

Five forms hand-roll `saving = ref(false)` gating `:disabled="saving"` around submit — that is rendered, existing state, so the instance exposes a `busy` flag set/cleared by `run` (try/finally) and forms render it on their submit buttons. Pages ignore it; no page-level loading state ships (nothing renders one today). Alternative rejected: forms keep private `saving` refs (the double-submit guard stays hand-rolled — the drift this change deletes).

### D5 — Panels take the page's runner via props

MonthDetailPanel, MonthSettingsPanel, ForecastGrid receive the page instance's `run` (or `reload`/`run` pair) as a prop and call it directly; their `saved: [text]` / `failed: [text]` emits are deleted (33 + 29 sites). Domain events that carry state, not display — the 月結 panel's `deleted` — remain emits. Forms keep `saved: [entity]` emits so the parent still chooses the message and owns reload-on-saved. Alternative rejected: `provide`/`inject` (each panel has exactly one consumer parent; a prop keeps the seam visible in the template).

### D6 — Adoption: pilot, then sweep, one change

Task 1 builds `page.ts` and adopts it in IbkrTransfersView (1 write action, 80 lines) — the smallest real page — to settle the shape. Then per-module tasks sweep views (small → large), then forms, then the three panels. Each task ends with `pnpm build` (vue-tsc is the drift check); `errorMessage` imports disappear from views as they adopt. Alternatives rejected: pilot-only change (half-adoption is the drift); pilot + separate sweep changes (the repo treats same-kind sweeps as one change, e.g. cash-movement-module).

## Risks / Trade-offs

- [Sweep touches ~25 files in one change] → per-module tasks that each keep `pnpm build` green; behavior parity by keeping every message string byte-identical (the pilot task fixes the mapping idiom once).
- [Deleting `saved`/`failed` text emits could orphan a parent handler] → each panel task updates its parent (MonthStatView, OverviewView) in the same task; `pnpm build` fails on any missed handler (typed emits).
- [`run` could hide a thrown error a caller needs to branch on] → no caller branches on write failures today (all 40 sites only set the error); if one appears, `run` returning the outcome is an internal addition that breaks no caller.
- [Vue reactivity nuance: replacing a form's local refs with instance state] → `pnpm build` + the live restart check (AGENTS.md rule) per module batch.

## Migration Plan

Pure frontend refactor, single change: build `page.ts` → pilot → sweep views → forms → panels → delete now-unused `errorMessage` imports from views. Rollback is reverting the change's commit. After the final task: restart the backend (it serves the built frontend) and spot-check 月結, 持倉總覽, and a form's field errors against the live app.

## Open Questions

None blocking. One design-level addition beyond the grilling rounds, flagged for veto: D4 (`busy` flag) — Q9 ruled out a *page-loading* flag as dead surface; D4 brings in the *form-saving* flag because it exists and renders today (5 forms, `:disabled="saving"`).
