# Design

## Context

`frontend/src/components/MonthSettingsPanel.vue` (~550 lines) is a collapsible 設定 card inside `MonthStatView.vue`. It hosts 活期帳戶 and 資產 CRUD plus one `InlineForm` covering every `AppSettingsPatch` field. `MonthStatView` fetches `settings`/`assets`/`accounts` solely to feed this panel; the panel's `summary`/`year` props exist only to label and save the per-year `pool_rate`. `PATCH /api/settings` already accepts partial patches (every field `Option`), so the backend needs no change. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**

- A standalone 設定 page reachable from a top-level nav group, with content identical in capability to today's card.
- Per-section editing so a save touches only that section's fields.
- The 開心Pool section shows all stored years' rates — `GET /api/settings` gains a `pool_rates` list on `AppSettings` (read side only; writes still go through the existing PATCH fields).

**Non-Goals:**
- No redesign of the editors themselves — tables, forms, validation messages move as-is.
- No relocation of API-level settings requirements across specs (salary/pool-rate/Money Master stay in `month-stat`; goals/`bill_amount`/`semi_liquid_target` stay in `overview`; `calendar_private_id` storage stays in `time-deposits`). Only the UI-location statements move.

## Decisions

### One `SettingsView.vue`, not per-section components

The panel's logic (draft refs, `numOrNull` coercion, `props.run` wrapper) moves into a single `frontend/src/views/SettingsView.vue` using `usePage(load)`. The single `editingSettings` flag becomes `editingSection: 'general' | 'mm' | 'goals' | 'calendar' | null`; `startSettingsEdit`/`saveSettings` split into per-section `startXxxEdit`/`saveXxx` pairs reusing today's draft init and validation per field. Alternative considered: four section components — rejected; the shared draft/`run` plumbing is small and splitting would add prop drilling for no behavioral gain. The file lands ~600 lines; splitting later is cheap if it grows.

### 開心Pool becomes a rates table; `GET /api/settings` exposes `pool_rates`

The rate field leaves the 一般 section entirely. Its own section mirrors the 活期帳戶/資產 pattern: one row per stored year （年份 / 利率） with 編輯 and 刪除, plus a 新增 action and the current year's effective rate (`settings.pool_rate`) shown beside it. `store::months::pool_rates()` already returns the whole `pool_rates` table as a `BTreeMap<i32, f64>`; `service::settings::get` maps it onto a new `AppSettings.pool_rates: Vec<PoolRate>` (`{year, rate}` — an array, not a map, so openapi-typescript emits a typed row and `v-for` iterates it directly). Writes reuse `PATCH /api/settings`: edit/add sends `pool_rate` + `pool_rate_year`; delete sends `pool_rate: null` for that year — `set_pool_rate(year, None)` already deletes the row and restores the earlier-year fallback. Alternative considered: a dedicated `GET /api/pool-rates` endpoint — rejected; the settings page already fetches `GET /api/settings`, and one field keeps the OpenAPI surface unchanged in shape.

### Summary line + 編輯 → form per section

Each section keeps today's interaction: a muted read-only summary line showing the effective values, an 編輯 button opening an `InlineForm` for just that section. Alternative considered: always-visible forms — rejected; long page of inputs and no at-a-glance summary.

### Settings API requirements stay in their consumer specs

Requirements describing the `GET/PATCH /api/settings` fields and `app_meta` storage live with the capabilities that consume them (`month-stat`, `overview`, `time-deposits`). The new `app-settings` spec covers only what the 設定 page does. Alternative considered: move all settings requirements into `app-settings` — rejected; it would churn four specs for no behavior change and separate each field's rules from the feature that uses them.

## Risks / Trade-offs

- **`pool_rates` rows without an effective-rate marker** → The table shows stored rows; years relying on the fallback simply aren't listed, and the current year's effective rate is shown separately so the effective figure is always visible.
- **`SettingsView.vue` ~600 lines** → Acceptable; the split-seam is obvious (per-section components) if it grows.
- **Eight nav groups** → 設定 is a leaf group appended last; no group-order semantics change.
- **`pool_rate_year` from a free year field in the row form** → A typo writes a `pool_rates` row for a wrong year; recoverable via the row's 刪除, which restores the earlier-year fallback.
