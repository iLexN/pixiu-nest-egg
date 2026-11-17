# Proposal

## Why

`MonthStatView.vue` (901 lines) holds two concerns: the year/month list view
and the whole month-detail editor (figure editing, item CRUD, suggestion
accept/dismiss, auto-interest breakdown). `MonthSettingsPanel` already proved
the panel convention — props down, `@saved`/`@failed` up, the panel calls
`api` itself — and the view's message/error handlers are already named for it.

## What Changes

- New `frontend/src/components/MonthDetailPanel.vue` owns the detail card:
  the month figure editor, the per-category item tables, suggestion rows,
  the `interest_auto` breakdown, and the item-add form.
- Interface: one prop `:detail="detail"`; emits `saved`/`failed`/`deleted`.
  `selectedIsCurrentOrLater` derives inside the panel; `monthDraft` re-inits
  on `detail` change.
- `MonthStatView.vue` keeps the year bar, year summary, running averages,
  month table, add-month form, and `MonthSettingsPanel`; it owns
  `selectedMonth`/`detail`/`load` and clears selection on `@deleted`.
- No behavior change — same card, same actions.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None.

## Impact

- `frontend/src/components/MonthDetailPanel.vue` — new.
- `frontend/src/views/MonthStatView.vue` — ~901 → ~430 lines.
