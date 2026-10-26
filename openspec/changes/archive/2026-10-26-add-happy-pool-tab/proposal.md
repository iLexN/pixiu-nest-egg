# Proposal

## Why

The 開心Pool is shared between iLex and Irene. The app tracks its yearly chain (pool income, Irene's contributions, 娛樂支出, closing balance), but nothing shows how much each person has put in overall or how much Irene still owes to match iLex's half. Today that settlement is worked out by hand.

## What Changes

- `GET /api/months/summary` gains a `pool_totals` object over all stored years: `ilex_input` (Σ pool income = Σ interest × that year's pool rate), `irene_input` (Σ `pool_input`, the 月結 `Irene + 開心 Pool` column), and `irene_payable` (`ilex_input ÷ 2 − irene_input`).
- A new **HappyPool** sub-tab sits after 年結 in the 總覽 group. It shows:
  - Four headline cards: 開心 Pool (the live balance, the same figure as the 總覽 tile), iLex, Irene, and Irene payable.
  - A per-year table with the columns year, 結餘 (the chained pool balance), iLex input, Irene input, and 支出 (the year's 娛樂支出).
- The 總覽 group's sub-tabs become 總覽, 月結, 年結, HappyPool.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `month-stat`: the yearly summary adds the 開心Pool contribution totals and the Irene payable figure, and a HappyPool page renders them alongside the per-year pool table.
- `app-navigation`: the 總覽 group adds a HappyPool sub-tab after 年結.

## Impact

- Backend: `backend/src/calc/months.rs` (new pure totals calculation and its test) and `backend/src/routes/months.rs` (the summary response field). No migration and no new endpoint.
- Frontend: `frontend/src/api.ts`, `frontend/src/App.vue`, and a new `frontend/src/views/HappyPoolView.vue`.
- Docs: `docs/overview.md` and `docs/DATA_FLOW.md`.
