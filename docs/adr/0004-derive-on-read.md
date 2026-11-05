# Derive on read, store inputs only

All computed money figures — per-trade 平均單價, holdings, cost, weighted average, market value, month figures, year review rows — are computed on read from stored inputs, never recomputed and stored. The one nuance: month `total_assets`/`liquid_assets` store NULL meaning "live" — every read derives them from the current portfolio until 重新擷取 freezes a snapshot.

### Considered options
- Storing aggregates on parent rows — rejected: a deleted or edited trade/deposit silently corrupts stored totals; derivation-from-source can never go stale.
- Flip create-default to live-NULL for bare PATCH — rejected in `add-month-live-totals`: it would remove the create-and-freeze primitive; the 新增月份 form sends explicit nulls instead.

Source: `openspec/changes/archive/2026-09-15-add-stock-trade-tracker`, `2026-09-30-add-month-live-totals`.
