# Proposal

## Why

Adding a month in 月結 creates the row with `total_assets`/`liquid_assets` frozen at that moment (the create branch of `PATCH /api/months/:ym` snapshots live values), so every new month must immediately be unfrozen by hand with 改為即時. The new row should start live, matching the sheet where a fresh row reads `Overview!B1`/`H1` until it is paste-valued at month close.

## What Changes

- The 新增月份 form creates the row with `total_assets`/`liquid_assets` NULL — live-derived (即時計算) from the start — by sending both fields explicitly `null` instead of omitting them.
- The bare `PATCH` create contract is unchanged: omitted totals still snapshot live values, `recapture` still re-snapshots, explicit `null` still means live.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `month-stat`: the "Frozen and live asset totals" requirement — the 新增月份 form SHALL create rows live; snapshot-on-create applies only to a create patch that omits the totals fields.

## Impact

- `frontend/src/views/MonthStatView.vue` — `addMonth` sends `{ total_assets: null, liquid_assets: null }`.
- `openspec/specs/month-stat/spec.md` — delta on "Frozen and live asset totals" (wording + scenarios).
- `docs/overview.md` — payday-entry data-flow wording.
- No backend, migration, import, or parity changes; `MonthStatPatch` already accepts `number | null`.
