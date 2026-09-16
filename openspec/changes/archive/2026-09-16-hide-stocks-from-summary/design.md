## Context

`stocks.is_active` exists end-to-end but is unused: the column is written as `1` everywhere, `StockPatch.is_active` is applied by `PATCH /api/stocks/:id`, `Stock.is_active` is serialized, and `api.updateStock` accepts it. `RowActions.vue` already provides the ⋯ menu used by StockTable, TradeTable and DividendTable.

## Goals / Non-Goals

**Goals:**
- Hide/unhide a stock from 股票管理 with one action; hidden rows are muted but remain visible in the registry.
- 持倉總覽 hides the rows while keeping them in totals, rollups, ordering and the API payload.

**Non-Goals:**
- No backend or schema changes.
- Hidden stocks stay selectable in trade/dividend forms — hiding is a display concern, not a lifecycle state.

## Decisions

- **Use `is_active` rather than a new `hidden` column.** The flag already exists with exactly this semantic; reusing it avoids a migration and a parallel field. Alternative considered: a separate `hidden` column — rejected as redundant.
- **Filter in the frontend, not the API.** `GET /api/summary` keeps returning hidden stocks so totals/rollups/parity/reorder are computed over the complete set and no backend branch is needed. `SummaryView` renders `stocks.filter(s => s.is_active)` and shows a `已隱藏 N 支股票` note. Alternative considered: a `?include_hidden` query param — rejected because it would make totals semantics depend on a display flag and would break the reorder contract (which requires every stock id in the market).
- **Totals keep counting hidden stocks** (user decision) — matches the workbook, where every row feeds the sums.
- **RowActions gains an optional `hideable`/`hidden` prop pair and `hide` event** so TradeTable and DividendTable are unaffected.

## Risks / Trade-offs

- Drag-reorder with hidden stocks → the drop handler already maps over the full `summary.stocks` array, so the complete id list is sent and hidden rows keep their positions.
- A hidden stock receiving a new trade stays hidden until unhidden → acceptable; the registry still shows it muted.
