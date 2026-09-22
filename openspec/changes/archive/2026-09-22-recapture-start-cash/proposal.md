# Proposal

## Why

Adding a month asks the user to type 月初(出糧後) by hand, but that figure is just the 活期 bank total the app already tracks in the `cash` manual assets (the same sum shown in Overview's 半流動資金 block). Re-typing it duplicates data and invites typos. The user wants the add-month step to take no 月初 input, and the existing 重新擷取 action — which already re-snapshots 總數/流動資產 — to also capture 月初 from the live 活期 sum.

## What Changes

- The 新增月份 form drops its 月初 input; creating a month stores no `start_cash` (the column shows `—` until captured or edited).
- `PATCH /api/months/:ym` with `recapture: true` additionally re-snapshots `start_cash` to the live 活期 sum (Σ `manual_assets` where `kind='cash'`). An explicit `start_cash` in the same patch still wins over the snapshot, matching the totals' precedence.
- The month editor keeps its manual 月初 field for correcting historical rows; `改為即時` is unchanged (月初 has no live mode — it is always a stored snapshot).
- The recapture button is relabeled 重新擷取月初/總數/流動.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `month-stat`: `start_cash` is no longer collected at month creation; `recapture` re-snapshots it from the live 活期 cash sum alongside the totals.

## Impact

- `backend/src/routes/months.rs` — `upsert` update branch resolves `start_cash` as patch > recapture's `cash_sum` > stored; create branch unchanged.
- `frontend/src/views/MonthStatView.vue` — 月初 input removed from 新增月份 (`addMonth` sends `{}`); recapture button/message renamed.
- `backend/tests/api.rs` — new test covering create-without-月初, recapture fill, refresh after a cash-row edit, and explicit-wins.
- `docs/DATA_FLOW.md`, `AGENTS.md` — Month Stat data-flow wording.
- No migration, no import/parity changes — the workbook's F column still imports verbatim.
