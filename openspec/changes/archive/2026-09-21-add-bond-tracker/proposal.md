# Proposal

## Why

Bonds (債券) are still tracked entirely in `財富分析報告.xlsx`: a registry table (label / issue number / principal / maturity, with a `Total` row summing principal) plus a per-bond coupon schedule (付息日 / 利息釐定日 / 年息率 / 每1萬利息 / derived interest). Moving this section into the app continues the section-by-section migration and — unlike the sheet, where matured bonds are deleted — lets the app keep matured bonds and their coupon records as history going forward.

This change covers only the 債券 section. AIA, 香港年金, Month Stat and Overview remain in the workbook; they are different data shapes and are deferred to their own changes.

## What Changes

- New `bonds` table storing one row per bond: `label` (e.g. `silver bond`), `issue_no` (e.g. `03GB2710R`), `principal`, `maturity_date` (the sheet's `end` column), `note`, `sort_order`.
- New `bond_coupons` table storing one row per scheduled coupon: `bond_id`, `pay_date` (付息日), `fixing_date` (利息釐定日), `annual_rate` (年息率; `NULL` = 待定), `per_10k` (每1萬港元債券利息; `NULL` = 待定), `received_amount`, `note`.
- Derived on read, never stored (repo convention): bond `status` (active while `maturity_date` is in the future, matured otherwise — same rule as deposits); coupon `expected = per_10k × principal ÷ 10000` (the sheet's interest column); coupon `status` (`待定` while rate/per_10k are unset, `pending` once fixed, `received` once `received_amount` is set); coupon `variance` (received − expected).
- Bond CRUD API: `GET/POST /api/bonds`, `PATCH/DELETE /api/bonds/:id`, `GET /api/bonds/summary` (active bonds with totals and upcoming coupons, matured bonds for history). Coupon API: `GET/POST /api/coupons`, `PATCH/DELETE /api/coupons/:id` for entering the fixed rate and recording receipt. Deleting a bond deletes its coupons.
- New `債券` nav group in the frontend with a 總覽 view: totals strip, one section per active bond showing its coupon table (待定 / pending / received states, inline rate entry, 收訖 marking), and a matured-bonds history section.
- `import_xlsx` extended to read the 債券 sheet's registry rows and coupon blocks (blocks matched to their bond via the 發行編號 in the block label; 待定 cells imported as `NULL`), idempotently, preserving workbook order.
- `check_parity` extended with a bonds section comparing the active principal total against the sheet's `Total` cell and coupon expected amounts against the cached interest column, skipping 待定 cells.
- Not in this change: AIA, 香港年金, Month Stat/Overview, bond currencies other than HKD (the sheet tracks none), principal repayment as a transaction (maturity only flips status), and multi-currency totals.

## Capabilities

### New Capabilities
- `bonds`: recording, listing, editing and deleting bond records with their derived maturity status; per-bond coupon schedules with the rate-fixing (待定 → fixed) and receipt (pending → received) lifecycle; and the 總覽/history views with active-principal totals and upcoming coupons.

### Modified Capabilities
- `spreadsheet-trade-import`: the import command also imports the 債券 sheet's bond registry and coupon schedules idempotently, and the parity command also compares bond totals and coupon amounts against the workbook's cached figures.
- `app-navigation`: the first-level nav gains a 債券 group (after MPF) containing 總覽; the 港股/美股 market toggle stays hidden while it is active.

## Impact

- Backend: new migration `0012_bonds.sql`; new `routes/bonds.rs`; additions to `models.rs`, `calc.rs`, `xlsx.rs`, `import.rs`, `parity.rs`, `routes/mod.rs`, and the import/parity binaries' coverage.
- Frontend: new `債券` nav group and `BondsView.vue` (+ bond/coupon forms); `api.ts` types and endpoints; `App.vue` nav wiring.
- Docs: `docs/DATA_FLOW.md` gains the bond data flow; `AGENTS.md` roadmap marks 債券 migrated.
- No new dependencies; `財富分析報告.xlsx` stays read-only and remains source of truth for non-migrated sections.
- Bond status depends on the current date, so a bond that matured since the sheet last recalculated will surface as an expected parity difference (same caveat as deposits).
