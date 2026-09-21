# Proposal

## Why

AIA insurance/annuity policies are still tracked entirely in `財富分析報告.xlsx`: a policy table (label / next pay / policy number / buy usd / now usd / derived balance %% / remaining years / withdrew / remark) plus a summary block that converts the USD totals to HKD via the Overview sheet's exchange rate and computes an overall return. Moving this section into the app continues the section-by-section migration; the policies are long-lived (multi-year premium terms and drawdown schedules), so the app becomes the natural place to keep values current.

This change covers only the `AIA` sheet. 香港年金, Month Stat and Overview remain in the workbook; they are different data shapes and are deferred to their own changes.

## What Changes

- New `aia_policies` table storing one row per sheet policy line: `label` (plan/group name, e.g. `年金 - 2024 - 2029`), `policy_no` (e.g. `B632611401`), `next_pay_date` (the next premium-due date), `premium_usd` (buy usd), `value_usd` (now usd) with `value_updated_at`, `remaining_years`, `withdrew_usd`, `note`, `link` (the remark column's AIA PDF hyperlink target — manual entry only, calamine does not read hyperlinks), `excluded` (row not counted in totals — the sheet's `irene 20%` share row), `in_account` (row counted in the sheet's `AIA display value` figure), `sort_order`.
- New `aia_events` table logging each premium payment and withdrawal: policy, `kind` (`payment`/`withdrawal`), date, USD amount, note, and the pre-update `next_pay_date`/`remaining_years` so a misrecorded event can be deleted cleanly. `premium_usd`/`withdrew_usd` stay stored cumulative figures — events drive their updates, they do not replace them.
- Derived on read, never stored (repo convention): per-policy `balance_pct = (value_usd + withdrew_usd − premium_usd) ÷ premium_usd` (the sheet's `balance %%` column); totals `premium`, `value`, `withdrew`, `balance_pct` over non-excluded rows (the sheet's `buy usd` / `now usd` / `balance %%` cells) and `display_value` over `in_account` rows (the sheet's `AIA display value` cell); HKD conversions of the totals via a stored rate.
- The workbook's USD→HKD rate cannot be fetched (it is a GOOGLEFINANCE cell), so a manual rate is kept in `app_meta` under `aia.usd_hkd_rate`, seeded from the workbook's cached `Overview!N3` at import and editable in the UI — same facility the MPF note uses.
- AIA API: `GET/POST /api/aia/policies`, `PATCH/DELETE /api/aia/policies/:id`, `GET /api/aia/summary` (policies with derived returns plus USD/HKD totals, the rate, and the next upcoming premium-due date), `PATCH /api/aia/rate` for the exchange rate, and `GET/POST/DELETE /api/aia/events`. Recording a `payment` event atomically adds to `premium_usd`, decrements `remaining_years` and advances `next_pay_date` a year — the sheet's three manual edits collapsed into one action so nothing is forgotten; a `withdrawal` event adds to `withdrew_usd`.
- New `AIA` nav group in the frontend (after 債券) with a single 總覽 view: totals strip (USD totals + HKD conversions + overall return + next premium due), editable exchange rate, the policy table with per-row balance %, withdrew and remaining-years columns plus 繳費/提取 actions, and a per-policy payment/withdrawal history.
- `import_xlsx` extended to read the `AIA` sheet's policy rows (numeric buy/now cells in the policy block, carrying the plan label down to continuation rows), the cached summary cells, and the cached `Overview!N3` rate — idempotently, preserving workbook order.
- `check_parity` extended with an AIA section comparing per-policy premium/value and the USD/HKD totals against the sheet's cached figures.
- Not in this change: 香港年金, Month Stat/Overview (the workbook keeps the live GOOGLEFINANCE rate; the app keeps a manual copy), value-update history (the sheet stores a single current value, like `manual_price` on stocks), HKD premium entry (payments are logged in USD; the sheet's one HKD-priced policy stays hand-converted) (the sheet's one HKD-priced row is stored already converted to USD), and the scratch `GG` cells below the table.

## Capabilities

### New Capabilities

- `aia-policies`: recording, listing, editing and deleting AIA policy rows with premium paid, current value, withdrawals, remaining years and remarks; per-policy and portfolio-level derived returns; the excluded/in-account flags that reproduce the sheet's two totals figures; the premium-payment and withdrawal event log with its one-click record actions; and the manual USD→HKD rate with HKD conversions.

### Modified Capabilities

- `spreadsheet-trade-import`: the import command also imports the `AIA` sheet's policy rows and seeds the exchange rate idempotently, and the parity command also compares AIA per-policy figures and totals against the workbook's cached values.
- `app-navigation`: the first-level nav gains an `AIA` group (after 債券) containing 總覽; the 港股/美股 market toggle stays hidden while it is active.

## Impact

- Backend: new migration `0013_aia.sql`; new `routes/aia.rs`; additions to `models.rs`, `calc.rs`, `xlsx.rs`, `import.rs`, `parity.rs`, `routes/mod.rs`, and the import/parity binaries' coverage.
- Frontend: new `AIA` nav group and `AiaView.vue` (+ policy form); `api.ts` types and endpoints; `App.vue` nav wiring.
- Docs: `docs/DATA_FLOW.md` gains the AIA data flow; `AGENTS.md` roadmap marks AIA migrated.
- No new dependencies; `財富分析報告.xlsx` stays read-only and remains source of truth for non-migrated sections.
- The HKD figures depend on the manually kept rate, so they drift from the workbook's live GOOGLEFINANCE rate between edits — an accepted approximation documented in DATA_FLOW.md, and harmless to parity (parity uses the same seeded rate).
