# Proposal

## Why

The 總覽 page shows the IBKR account block read-only with a pointer to 股票 → 美股 → 總覽 for edits. The same figures and the same `PATCH /api/ibkr` endpoint back both places; duplicating the table while keeping editing on one page only is drift waiting to happen.

## What Changes

- Extract the 美股 summary's IBKR card (display table plus the 編輯 form: transfer delta/date, App 現值, HKD/USD 現金) into a shared `IbkrCard` component.
- 總覽 renders the same component in place of its read-only table and reloads after a save, since the asset-table IBKR row, 總數/流動資產, liquidity tiers, and goals all derive from `computed_total_hkd`.
- No backend work: `PATCH /api/ibkr` and its validation are unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: the 總覽 page's IBKR block becomes the same editable card as the 美股 summary — editable inputs saving through `PATCH /api/ibkr`, reloading the view on save.

## Impact

- **Frontend**: new `frontend/src/components/IbkrCard.vue`; `frontend/src/views/SummaryView.vue` and `frontend/src/views/OverviewView.vue` consume it.
- **Docs**: `docs/overview.md` — the IBKR edit flow lives on both pages.
