# Proposal

## Why

Annual recurring expenses — tax, insurance premiums, fund contributions — live in the 預測 grid's Tax/基金/醫療保險 row and must be re-entered by hand every year. Re-typing the same amount and note is tedious and error-prone; a one-click copy to the same month next year matches how these items actually recur.

## What Changes

- The Tax/基金/醫療保險 plan editor gains a `複製` action per item that creates a new forecast item of the same kind in the same month of the following year, carrying the row's amount and note. The source item is unchanged.
- The copy lands outside the 7-month forecast window and stays stored but invisible until the window slides forward — already-covered behavior for out-of-window items. The UI reports the target month so the action is not silent.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: the forecast item editor gains a copy-to-next-year requirement for Tax items.

## Impact

- `frontend/src/views/OverviewView.vue` — new `copyItem` action and one button in the plan editor, calling the existing `POST /api/forecast/{ym}/items` endpoint. No backend, API, or schema changes.
- `docs/overview.md` — the editor flow block gains the 複製 step.
