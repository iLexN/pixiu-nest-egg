# Design

## Decision: kind becomes a filter, not a field

`RigidExpense` drops `category`. The Tax/Bill/Other kind check moves fully into the collection filter — it defines *which* plan items are rigid expenses, and is no longer emitted on the entries.

## Consequence: stored-Bill detection

Today `bill_from_items` is detected by matching `category == Bill` on the collected `RigidExpense`s. Without the field, the check scans the month's `cells` directly: a stored Bill item exists when a Bill-kind cell has non-empty `plan_items`. Same semantics — a stored Bill item suppresses the synthesized `差餉`.

## Serialized shape

```json
"rigid_expenses": [
  { "item": "AIA 醫療", "amount_hkd": -40452.24 },
  { "item": "差餉", "amount_hkd": -2158.0 }
]
```
