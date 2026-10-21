## Why

A pending bond coupon or HK dividend dated in the month previews in the 利息 breakdown but never becomes a 調整 suggestion, while a 定期 ending in the month suggests `dep-end` before 收訖 — the same receipt lifecycle handled two ways. Suggesting pending receipts lets the user accept them like any other event instead of waiting for 收訖.

## What Changes

- Bond coupons and HK dividends dated in the month are suggested as `adjustment` items whether or not they are 收訖: received ones carry `received_amount`, pending ones the expected (`per_10k`-derived) or estimated figure. Receipts with no known amount (待定 coupon, estimate-less dividend) are not suggested.
- Accepting a pending suggestion stores the item with its `auto_key`; a later 收訖 credits the cash and skips the item write (already stored) — the same accept-then-收訖 order `dep-end` already supports.
- `interest_auto`/`auto_interest` unchanged: pending receipts still preview muted and count only once 收訖.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `month-stat`: "Remaining suggested candidates" — the coupon and HK dividend clauses no longer require 收訖; pending receipts suggest their expected/estimated figure and amount-less receipts are not suggested.

## Impact

- Backend: `calc::build_suggestions` drops the `received` check for dividends/coupons and skips receipts with no amount; comment updates. No API shape, schema, or endpoint changes.
- Docs: `docs/overview.md` suggestions paragraph.
