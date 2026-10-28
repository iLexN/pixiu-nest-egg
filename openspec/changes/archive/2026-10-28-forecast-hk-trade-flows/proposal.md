# Proposal

## Why

The 總覽 → 預測 grid's 股票 row only sums hand-entered `stock` forecast items, so a real HK trade never reaches the forecast. On 2026-10-27 a 匯豐 BUY of 54063.17 left 活期, but the October column still counts only the −50000 "HSBC" plan, and every chained month's 活期/半流動/ref check reads high. 月結 already picks the trade up as a `trade:` suggestion; the forecast should agree.

## What Changes

- Real HK trades dated inside the forecast window become cash flows in their month's 活期: BUY → −`total`, SELL → +`total`. US trades stay out (they settle inside IBKR, matching the 月結 `trade:` suggestion rule).
- Each forecast month exposes the trades as `trade_lines` (read-only), and the 股票 cell shows Σ stock plan items + Σ trade amounts, listing the trades read-only (marked `已交易`) in its editor.
- Stock plan items are left untouched: a plan is an estimate that may cover several buys of unknown final amount, so the user trims or deletes it by hand once the real trades land. No convert action and no automatic suppression.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `overview`: the forecast month gains `trade_lines`; the 活期 `cash` formula adds Σ trade amounts; the 股票 grid cell shows trade lines read-only alongside plan items.

## Impact

- Backend: `calc/forecast.rs` (input + derived figure + tests), `routes/forecast.rs` (load HK trades in the window via the existing `store::trades::hk_suggestion_trades`), `models/forecast.rs` (new `ForecastTrade` schema, `trade_lines` field). No schema migration.
- Frontend: `api.ts` types, `OverviewView.vue` 股票 cell figure and editor.
- Docs: `docs/overview.md` 預測 section.
- API: `GET /api/forecast` response gains a field (additive, non-breaking).
