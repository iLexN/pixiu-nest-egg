# Proposal

## Why

Two muted helper texts went stale. The 投資目標 card still carries a `月薪增幅於 年結 → 投資 覆寫` hint that was never specified and is no longer wanted — the 年結 page already documents the override itself. And the dedicated-settings-page move (`2026-11-18-dedicated-settings-page`) took every settings editor off the 月結 page, but two texts still send users to "月結設定".

## What Changes

- Remove the `月薪增幅於 年結 → 投資 覆寫` muted line under the 投資目標 formula footnote on the 總覽 page (`OverviewView.vue`).
- 年結 year-end guide: `薪金變了才去月結設定更新` → `薪金變了才去設定更新` (`YearReviewView.vue`).
- 定期 checklist note: `money master 在月結設定更新` → `money master 在設定更新` (`DepositsView.vue`).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — display text only, no requirement changes (`skip_specs: true`). The 投資目標 hint and the 年結 guide note are unspec'd; the deposits note stays conformant with `time-deposits`' "Reminder checklists", which names only "the app's settings".

## Impact

- `frontend/src/views/OverviewView.vue`, `frontend/src/views/YearReviewView.vue`, `frontend/src/views/DepositsView.vue` — text-only edits.
- No API, store, spec, or doc changes.
