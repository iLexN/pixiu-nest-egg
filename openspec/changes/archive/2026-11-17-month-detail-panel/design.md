# Design

## Decisions

### D1: One prop, three emits — the `MonthSettingsPanel` shape

`MonthDetailPanel` takes `:detail: MonthDetail` (the whole
`GET /api/months/{ym}` response) and emits `saved`/`failed`/`deleted`. The
panel calls `api` itself for every mutation; the view only listens, toasts,
and reloads — the convention `MonthSettingsPanel` set, with `deleted` added
because removing a month must clear the view's selection.

### D2: The panel derives what it can

`selectedIsCurrentOrLater` computes from `detail.month.month` (history rows
never re-snapshot); `monthDraft` re-inits in a `watch(props.detail)` — the
view stops owning `startEdit`.

### D3: The view keeps coordination state

`selectedMonth`, `detail`, `load`, the year/month tables, the add-month form,
and the settings-panel wiring stay in the view: they span the whole page, not
the card.
