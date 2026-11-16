# Design

## Boundary

The component owns everything keyed to the forecast window:

- `KIND_INFO`/`PLAN_ROWS` row metadata, `cellOf`.
- Selection state: `selected`, `selectedMonth`, `selectedCell`,
  `selectedKind`, `selectedItems/Placements/Trades/IbkrLines`.
- Draft state: `drafts`, `newItem`, `selectCell`, `syncDrafts`.
- Mutations: `saveItem`/`addItem`/`removeItem`/`copyItem` and the
  `converting`/`startConvert`/`onConverted` flow (DepositForm).
- `sourceLabel` for the 利息 component list.

## Props / emits

- `forecast: ForecastResponse` — the view's `v-if="forecast"` guard stays,
  so the prop is non-null.
- `saved: [message: string]` — a write succeeded; the view sets
  `message`, clears `error`, and re-runs `load()` (which refetches both
  overview and forecast).
- `failed: [message: string]` — a write failed; the view sets `error`.

The view keeps the single top-of-page feedback line — the child never
renders its own `<p class="error|ok">`.

## syncDrafts

Before: `load()` in the view called `syncDrafts()` after refetch when a
cell was selected. After: the child `watch`es `props.forecast` and calls
`syncDrafts()` when a selection exists — the prop is replaced wholesale
on every reload, so the watcher fires exactly when the view's load did.

## Styles

Scoped styles can't cross components: the `.forecast`/`.forecast-grid`/
`.plan-*`/`.component-list` rules move; `.cell-input` is duplicated
(the view still uses it on its own edit inputs); `.deposit-form` keeps
working because a child's root element carries the parent's scope id.
