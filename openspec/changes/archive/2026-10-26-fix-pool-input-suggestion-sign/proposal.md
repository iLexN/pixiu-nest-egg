# Proposal

## Why

In Month Stat (`月結`), the auto-suggested `pool-input` (`Irene + 開心 Pool`) adjustment item is currently produced with a negative sign (`-pool_input`), under the mistaken assumption that `pool_input` is money leaving the bank. In reality, `pool_input` is an inflow into the bank account (and in every historical workbook `Month Stat` formula imported into `month_items`, `pool_input` is added as `+pool_input` in column G `調整` so `month_spend = start_cash + Σadjustment − end_cash` remains neutral).

## What Changes

- Change the auto-suggested `pool-input` candidate in `GET /api/months/:ym` from `-pool_input` to `+pool_input` when `pool_input > 0`.
- Update the `month-stat` spec and `docs/overview.md` to reflect that `pool-input` suggests `+pool_input`.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `month-stat`: The `Remaining suggested candidates` requirement changes the `pool-input` candidate amount from `− pool_input` to `+ pool_input`.

## Impact

- Backend: `backend/src/calc/suggestions.rs` (`build_suggestions` and unit test `suggestions_cover_each_event_kind`).
- Docs: `docs/overview.md` (`Month Stat (總覽 → 月結)` suggestions note).
