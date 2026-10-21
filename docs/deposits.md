# 定期 flows — deposits and family deposits

Part of the [data-flow guide](DATA_FLOW.md). Table definitions: [`deposits`, `family_deposits`, `cash_accounts`, `manual_assets`, `month_items`](database.md).

## Add a deposit in 定期 → 記錄

The 新增定期 button reveals DepositForm, which collapses after a successful save.

```text
DepositForm
  → POST /api/deposits
  → backend validates end_date and non-negative amounts
  → INSERT one row into deposits, sort_order appended
  → INSERT the dep-start:<id> adjustment month_item of −principal for the
    start month (skipped when the deposit has no start_date or the start
    month has no row — the dep-start suggestion still surfaces then; an
    already-stored keyed item is refreshed to the new amount)
  → GET /api/deposits?year=... reloads the history list
```

The form accepts rate as a percent; the API stores the fraction. `bank` is free text with 渣打 (SC) / 恒生 (HS) presets.

A blank rate is derived on read as 利息 ÷ 本金 × 365 ÷ (end_date − start_date) calendar days, only when principal > 0, interest and start_date are present, and end_date is after start_date. The API flags such a rate `rate_derived: true`; it is never stored, so a PATCH that leaves the rate field absent keeps the column NULL rather than freezing the derived value. An entered rate always wins (`rate_derived: false`).

## Edit or delete a deposit

```text
DepositForm edit mode (via 編輯 in a DepositTable row's ⋯ menu)
  → PATCH /api/deposits/:id
  → backend merges the patch, validates, UPDATEs one row

刪除 in the row's ⋯ menu → DELETE /api/deposits/:id
```

Derived fields (`total`, `status`, end month/year) change automatically on the next read.

## Load the 定期 views

```text
DepositsView (定期 → 總覽)
  → GET /api/deposits/summary
      → upcoming list (unreceived deposits, earliest end first — an
        end_date in the past without 收訖 shows 已到期未收)
      → active totals, month buckets, bank rollups (all receipt-based —
        the same unreceived deposits the list shows)
      → year tables for every end year present, plus the current year
  → 收訖 on an upcoming row → POST /api/deposits/:id/receive
      → UPDATE deposits SET received_at (+ interest correction and
        credited_account_id/credited_amount when 存入活期 is picked)
      → UPDATE cash_accounts amount += principal + (corrected) interest —
        the chosen account_id or the HKD default receipt account (不存入
        sends bank_in: false); the amount is fixed, not user-entered
      → INSERT the dep-end:<id> adjustment month_item for the end month
        for the same amount (an already-stored keyed item is refreshed to
        the receipt amount; skipped when the month has no row)

DepositHistoryView (定期 → 記錄)
  → GET /api/deposits/summary
      → history_years for the year selector
  → GET /api/deposits?year=YYYY&order=desc
      → history rows for the selected year
```

The list and every total/rollup are receipt-based: a deposit counts while it is not 收訖, whatever its `end_date`, and leaves on 收訖 (a stored `received_at`) — including a deposit 收訖'd early. An overdue-but-unreceived deposit shows 已到期未收 in the list and still counts in active principal, 已定期, and live 流動資產 until the user runs 收訖 → 存入活期, which moves the money into a cash_accounts row.

The 手動步驟提醒 checklists (定期 start step / 定期 end step) are static hints for the `Month Stat`, `回報率`, `Overview`, and money-master bookkeeping that used to live in the workbook.

## Load the 家人 → 定期 view

```text
FamilyDepositsView (家人 → 定期)
  → GET /api/family/deposits/summary
      → one section per holder (anyone with deposits or a stored note):
        their note, the unreceived 未到期 list ordered by end date
        (flagging 已到期未收 where past due), 活躍本金 = Σ principal over
        the holder's unreceived deposits (a matured one keeps counting
        until 收訖)
      → history_years for the year selector
  → GET /api/family/deposits?year=YYYY&order=desc
      → 記錄 rows for the selected year
  → holder chips (全部 + one per holder) filter the sections
```

```text
新增定期 / 編輯 in a row's ⋯ menu
  → POST /api/family/deposits or PATCH /api/family/deposits/:id
  → validates holder (required, trimmed), end_date, non-negative amounts —
    the same rules as 定期 minus the rate (there is no rate field; the
    bank's stepped-rate schedule goes in note)

收訖 on an upcoming row
  → POST /api/family/deposits/:id/receive { received_at?, interest? }
  → marks the deposit received (409 if already received) and optionally
    corrects the interest actually paid — nothing else happens: no cash
    manual_assets credit, no month item, no suggestion, and a past
    end_date is never auto-received on create/edit

刪除 → DELETE /api/family/deposits/:id
```

```text
編輯 on a holder's note block
  → PUT /api/family/holders/:holder/note { note }
  → writes the family.note.<holder> app_meta key; empty/null deletes it
  → the note survives deleting the holder's last deposit
```

Isolation: `family_deposits` rows are records only — they never enter 定期!B1, Overview 已定期/半流動資金/總數/流動資產, Month Stat 利息/`interest_auto`/suggestions, or the 回顧 figures.
