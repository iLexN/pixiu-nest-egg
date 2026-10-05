# Proposal

## Why

The 預測 grid's `ref check` row already flags danger in red, but `ref check` measures
半流動 against a ratio of 流動資產 — it stays green even while the projected 活期
(operating cash) drops to a level that cannot cover a month's spending. A low 活期
column is the earliest sign that a planned 定期 lockup or a large item leaves too
little spendable cash; it needs its own visual warning.

## What Changes

- The `活期` cells of the 總覽 → 預測 grid render in the negative/red style while
  the month's `cash` figure is below `20000` (strictly below; `20000` exactly and
  `—` absent cells render normally). The floor is a fixed display literal, like
  `ref_check`'s `0` — not a stored setting.

## Capabilities

### New Capabilities

### Modified Capabilities

- `overview`: the `總覽 預測 grid card` requirement gains the 活期 red-floor display
  rule alongside the existing `ref_check` red-while-negative rule.

## Impact

- `frontend/src/views/OverviewView.vue` — one conditional class on the 活期 row's
  `<td>`; `cash` is `number | null`, so the check must guard `null` (an unguarded
  `month.cash < 20000` coerces `null` to `0` and would paint `—` cells red).
- `docs/overview.md` — the 預測 section documents the new display rule.
- No API, model, migration, or calculation changes — `cash` already flows through
  `GET /api/forecast`.
