## Why

The 新增交易, 新增股票, and 新增定期 forms are permanently rendered at the top of their views, pushing the history tables — which the owner checks far more often than they add records — below the fold. The add-forms should be on-demand, not always visible.

## What Changes

- In 交易記錄 (`TradesView.vue`), replace the always-visible `<TradeForm>` with a 新增交易 toggle button; the form renders only while the button's panel is open.
- In 股票管理 (`StocksView.vue`), do the same for the add-only stock form with a 新增港股股票 / 新增美股股票 button (label follows the selected market).
- In 定期記錄 (`DepositHistoryView.vue`), do the same for `<DepositForm>` with a 新增定期 button.
- Each form auto-collapses after a successful save; the new row appearing in the history table is the confirmation. Toggling the button closed also resets/cancels an in-progress form (Vue remounts `TradeForm`/`DepositForm` on each open, so they start empty; `StocksView` calls `resetForm()`).
- Update `docs/DATA_FLOW.md` flow headers so the add flows mention the form is revealed by the 新增 button.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — this is a frontend presentation change. Where the add-form renders on the page is not spec-level behavior; the create/validate/persist requirements in `stock-trades` and `time-deposits` are unchanged. `skip_specs: true` is set in `.openspec.yaml`, matching the precedent in the archived `inline-stock-row-editing` change.

## Impact

- `frontend/src/views/TradesView.vue` — toggle button + `v-if` on `<TradeForm>` + collapse on save.
- `frontend/src/views/StocksView.vue` — toggle button + `v-if` on the form + collapse on save and on market switch.
- `frontend/src/views/DepositHistoryView.vue` — toggle button + `v-if` on `<DepositForm>` + collapse on save.
- `docs/DATA_FLOW.md` — minor wording updates to the three add flows.
- No backend, API, or database impact.
