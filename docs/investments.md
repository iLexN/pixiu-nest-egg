# Investments flows — MPF, 債券, AIA

Part of the [data-flow guide](DATA_FLOW.md). Table definitions: [`mpf_accounts`, `mpf_history`, `bonds`, `bond_coupons`, `app_meta`](database.md).

## Update MPF figures in MPF → 總覽

```text
MpfView edit form (via 編輯 in the account row's ⋯ menu)
  → PATCH /api/mpf/accounts/:id
  → backend merges the patch and validates (label required, amounts ≥ 0)
  → UPDATE one mpf_accounts row
  → if contributions or balance changed, inside the same transaction:
      → backfill a synthetic month-end mpf_history row per empty elapsed
        month, carrying the pre-update values
      → upsert today's mpf_history row
        (ON CONFLICT account_id+recorded_on → replace)
  → GET /api/mpf reloads accounts, totals, note, and history
```

Editing only metadata (label, trustee, plan/contract/member numbers) writes **no** history row. Deleting a history row via 記錄 → 刪除 removes just that row; last-month and max figures recompute on the next read, since they are derived rather than stored.

Derived on every read:

```text
per-account rate      = (balance − contributions) ÷ contributions   (empty when 0)
per-account last month = latest history row in the previous month
per-account max        = MAX(seed_max_*, all history rows, current),
                          rate and gain tracked independently
portfolio buy/now/%/gain = Σ contributions, Σ balance, derived
portfolio last month     = the as-of portfolio at last month-end
portfolio max            = the largest rate / largest gain independently over
                          the as-of timeline and the app_meta seeds
```

The as-of merge: for every recorded history date (plus last month-end and today), each account contributes its latest row at or before that date — an account with no eligible row falls back to its current values once it existed (`created_at` ≤ date), or is excluded before that. This carries an untouched account forward through months it was never edited and yields a true portfolio max rather than summing per-account peaks that may never have co-occurred.

The 備註 block saves free text via `PATCH /api/mpf/note` into `app_meta`; clearing it removes the entry.

## Load the 債券 view

```text
BondsView (債券 → 總覽)
  → GET /api/bonds/summary
      → Σ active principal, active bonds with their coupon schedules,
        matured bonds for history, and the unpaid coupons ordered by pay date
  → 收訖 on an unreceived matured bond → POST /api/bonds/:id/receive
      → UPDATE bonds SET received_at (+ credited_asset_id/credited_amount
        when 存入活期 is picked)
      → UPDATE manual_assets amount += credited (cash rows only)
      → INSERT the bond-end:<id> adjustment month_item for the maturity
        month (skipped when already stored or the month has no row)
  → 取消收訖 → POST /api/bonds/:id/unreceive reverses the credit, deletes
      the bond-end item, and clears received_at
```

Coupon receipt is per-coupon (each 付息日's `received_amount`); bond 收訖 covers only the principal return. The Active/已到期 split stays maturity-date based — an unreceived matured bond sits in 已到期 flagged 本金未收 until confirmed, and 債券!B1 parity is untouched since the sheet drops matured rows outright.

## Add or edit a bond or coupon

```text
新增債券 / 編輯 in a bond header's ⋯ menu
  → POST /api/bonds or PATCH /api/bonds/:id
  → validate label, principal > 0, valid maturity date

新增付息 / 編輯 in a coupon row's ⋯ menu
  → POST /api/coupons or PATCH /api/coupons/:id
  → validate pay_date and non-negative amounts

刪除 → DELETE /api/bonds/:id (cascades its coupons) or /api/coupons/:id
```

## Fix a coupon's rate or mark it received

```text
釐定 on a 待定 row
  → PATCH /api/coupons/:id { fixing_date, annual_rate, per_10k }
  → status becomes PENDING; the expected amount now shows

收訖 on a pending row
  → PATCH /api/coupons/:id { received_amount, bank_in? }
  → status becomes RECEIVED; variance = received − expected shows
  → banks the amount into the HS cash manual asset (uncheck 存入活期 to skip)
    and records the coupon:<id> adjustment month_item for the pay month
```

Rate fields are entered as a percent (4); the API stores the fraction. Clearing `received_amount` flips the coupon back to pending, reverses the stored bank credit, and deletes the coupon item. A matured bond's coupons stay editable — the receipt history remains completable after maturity.

## Load the AIA view

```text
AiaView (AIA → 總覽)
  → GET /api/aia/summary
      → every policy row with its derived balance_pct and event history,
        totals over non-excluded rows, the in-account display value, HKD
        conversions via the stored rate, and the next premium-due date
```

Two flags reproduce the sheet's two sums: `excluded` rows sit in the AIA account but are not the user's money (the `irene 20%` share), so they drop out of the totals; `in_account` rows feed `display_value`, the figure that should match the AIA portal — rows held in another account (`irene 年金`) count in the totals but not in `display_value`.

## Add or edit a policy, or update the rate

```text
新增保單 / 編輯 in a policy row's ⋯ menu
  → POST /api/aia/policies or PATCH /api/aia/policies/:id
  → validate label and non-negative USD figures
  → changing value_usd refreshes value_updated_at

編輯 on the USD → HKD card
  → PATCH /api/aia/rate
  → writes app_meta key aia.usd_hkd_rate — a manual copy of the
    workbook's Overview!N3 GOOGLEFINANCE cell; all HKD figures derive
    from it and are absent while unset
```

## Record a premium payment or withdrawal

```text
繳費 on a policy row
  → POST /api/aia/events { kind: "payment", event_date, amount_usd, next_pay_date }
  → one transaction: premium_usd += amount, remaining_years −= 1 (when
    set, never below zero), next_pay_date = the submitted date (the form
    proposes current +1 year); the event row snapshots the previous
    next_pay_date/remaining_years

提取 on a policy row
  → POST /api/aia/events { kind: "withdrawal", ... }
  → withdrew_usd += amount

刪除 on an event row
  → DELETE /api/aia/events/:id reverses it: the amount leaves its
    cumulative field and the snapshotted fields restore — delete is the
    undo for a misrecorded event
```

Recording a payment replaces the workbook's three manual edits (buy usd, remaining years, next pay) with one action. Amounts are entered in USD; the `irene 年金` premium paid in HKD is converted before entry.
