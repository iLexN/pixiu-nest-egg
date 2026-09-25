## Why

The 開心Pool figure on the 總覽 page is the live chained pool balance, not an average — it sits in the 過去 12 個月平均 card only because the workbook parks it at `Overview!G10`. It is also part of the headline math: `流動資產` is defined as liquid minus the pool balance, so surfacing it next to that figure makes the subtraction legible.

## What Changes

- The live 開心Pool balance moves from the 過去 12 個月平均 card to the headline strip at the top of 總覽, rendered as a fourth summary tile alongside 總數, 流動資產, and 流動資產 ÷ 薪金×100.
- The 過去 12 個月平均 card keeps its six averaged/budget figures (總數增加, 支出, 生活支出, 生活預算, 存, 利息).
- The 半流動資金 total renders red while below 25% × 流動資產 (green above), with its `A13` share and `C14` difference moved onto a second footer row.
- No API change: `GET /api/overview` keeps returning `averages.pool_balance`; the frontend renders the same field in a different slot.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `overview`: the 總覽 page's headline strip gains the live 開心Pool balance tile; the 過去 12 個月平均 card no longer lists it.

## Impact

- `frontend/src/views/OverviewView.vue` only: remove the 開心 Pool row from the averages table and add a `.total-card` to the headline `totals-grid`.
- `docs/DATA_FLOW.md` description of the two cards updated to match.
- No backend, schema, or parity-check changes; `averages.pool_balance` stays in the response for the moved tile.
