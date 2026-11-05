# 收訖 is the receipt point of truth; no unreceive; unreceived stays active

收訖 (received) is the single event that flips a deposit/coupon/dividend/family record to terminal: it credits the cash account (`credited_account_id`/`credited_amount`), feeds month 利息, and ends activity. **There is no unreceive action** — the three `POST .../unreceive` endpoints were deliberately deleted; a mistaken 收訖 is corrected by editing the credited row/item manually or deleting and re-creating the record. Consequently a deposit's activity rule is `!received`, not date-based: an unreceived deposit past `end_date` stays active in aggregates as 已到期未收 until the user confirms receipt — matching the workflow 收訖 → 存入活期.

### Considered options
- Keep unreceive endpoints — rejected: three near-identical reversal handlers for a rare correction; field-edit reversal (clearing `received_amount`) remains for coupons/dividends.
- Activity = `end_date > today || !received` — rejected: double-checks the same thing and an early 收訖 must stop counting, which the OR form gets wrong.
- Auto-receive past-dated family deposits — rejected.

Source: `openspec/changes/archive/2026-10-15-remove-unreceive-actions`, `2026-10-13-count-unreceived-deposits-as-active`.
