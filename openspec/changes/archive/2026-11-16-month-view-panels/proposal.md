# Proposal

## Why

`MonthStatView.vue` is 1399 lines; ~400 of them are the collapsible 設定
card — cash accounts, manual assets, and the settings display/edit form —
a self-contained area with 17 draft refs and its own edit state that the
view merely loads and re-renders. Extracting it mirrors the
ForecastGrid split and leaves the view to month data.

## What Changes

- New `frontend/src/components/MonthSettingsPanel.vue`: the whole `設定 ▸`
  card — 活期帳戶 table+form, 資產 table+form, the settings summary lines,
  and the settings inline form. Props `settings`, `summary`, `accounts`,
  `assets`, `year`; emits `saved(message)` / `failed(message)` so the view
  keeps owning the one feedback line and the reload (same convention as
  `ForecastGrid`).
- `MonthStatView.vue`: deletes the moved script/template/styles and renders
  `<MonthSettingsPanel>`.

No behavior change: `skip_specs: true`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor; `skip_specs: true` is set.

## Impact

- `frontend/src/components/MonthSettingsPanel.vue` — new.
- `frontend/src/views/MonthStatView.vue` — ~400 lines out.
