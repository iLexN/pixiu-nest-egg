# Design

## Context

The 總覽 → 預測 plan editor (`OverviewView.vue`) lists a (month, kind) cell's items with per-row amount/note inputs and 儲存/刪除 actions. Items persist via `POST /api/forecast/{ym}/items`, which accepts any `YYYY-MM` — items ahead of the 7-month window are stored and slide into view later (existing "Out-of-window plan items" requirement). See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- One-click duplication of a Tax item into the same month next year, no manual re-typing.

**Non-Goals:**
- No new endpoint — the existing create endpoint already accepts out-of-window months.
- No copy action on other kinds (interest/stock/bill/other) or deposit rows — deposit `return_month` shifting is out of scope by decision.
- No recurrence engine — a copy is a one-off duplication, not a linked/repeating rule.

## Decisions

- **Frontend-only**: `copyItem` computes the target `ym` as `item.month` year+1 and calls `createForecastItem` with `{kind, amount, note}`. A dedicated backend `copy` endpoint would add an API surface for what is a plain create.
- **Copy the row's current input values, not the stored values**: the button copies what the user sees; an unsaved edit is part of "the same value". Validation matches 儲存 (`金額必須是數字` on a non-numeric draft) and the source item is never modified. Alternative — copying stored values — was rejected as less intuitive.
- **Confirm with the target month**: the copy is always outside the window (window is 7 months, target is +12) so nothing visibly appears; `已複製到 YYYY-MM` is the feedback. Alternative — switching the selection to the target cell — is impossible since the column does not exist.

## Risks / Trade-offs

- **Duplicate copies** → double-clicking 複製 inserts multiple items that cannot be deleted until the window reaches them (~5+ months). Accepted: identical exposure already exists via the 新增 row for far-future months; the API keeps the rows deletable later.
- **Silent far-future state** → the user cannot see or edit the copy until the window slides. Mitigated by the confirmation message naming the target month.
