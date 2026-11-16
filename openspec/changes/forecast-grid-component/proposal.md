# Proposal

## Why

`OverviewView.vue` is 1223 lines; ~440 of them are the 預測 grid — a
self-contained feature (sheet Overview!A20:H36) with its own state
(selection, drafts, convert flow) that only happens to render inside the
overview page. Pulling it into its own component makes both halves
readable and keeps plan-item editing from entangling the overview's
asset-edit state.

## What Changes

- New `frontend/src/components/ForecastGrid.vue`: the 預測 card — the
  three-section month grid, the selected-cell plan editor, and the
  轉為定期 `DepositForm`. Props `forecast: ForecastResponse`; emits
  `saved(message)` / `failed(message)` so the view keeps owning the one
  feedback line and the reload.
- `OverviewView.vue`: deletes the moved script/template/styles and renders
  `<ForecastGrid>` with `v-if="forecast"`.
- The plan editor's `syncDrafts` becomes a `watch` on the `forecast` prop
  inside the child (it ran inside the parent's `load()` before).

No behavior change: `skip_specs: true`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor; `skip_specs: true` is set.

## Impact

- `frontend/src/components/ForecastGrid.vue` — new.
- `frontend/src/views/OverviewView.vue` — ~440 lines out.
