# Proposal

## Why

The `Overview` sheet's `J29:N35` block tracks the bank app's "Money Master" savings challenge (3 years / HK$1M, running 2023-10-27 → 2026-10-26). It is still maintained by hand: each month the user copies `month`, `saved`, and `comming save per month` from the bank app into the sheet, and `K35 can use` plus the progress ratios fall out of formulas. Two of the three copied figures turn out to be derivable — `month` follows from the challenge start date, and `comming save` from a formula verified against the bank's live figure — so the block migrates with only `saved` and the challenge parameters as real inputs.

## What Changes

- `GET /api/overview` gains a `money_master` block mirroring `J29:N35`: the challenge inputs (`start_date`, `saved`, `target_months`, `target_amount`), the effective `month_now`, `months_left`, effective `coming_save`, and the derived `avg_per_month` (`M31`), `yearly_rate` (`N31`), `time_progress` (`K33`), `saved_progress` (`L33`), `progress_gap` (`M33`), and `can_use` (`K35`).
- Six `app_meta` keys under `money_master.*` store the state: `start_date`, `saved`, `target_months`, `target_amount`, plus optional `month_now`/`coming_save` overrides (the `end_cash_override` pattern — set to pin a figure, clear to return to derivation). All editable through `PATCH /api/months/settings`; no new endpoints.
- Derivations verified against the bank app's live figures (2026-09-29): `month_now = full months elapsed since start_date + 1` → 36; `months_left = max(1, target_months − month_now + 1)` → 1; `coming_save = salary + (target_amount − saved)/months_left` → −41,705.06; `can_use = salary − coming_save` → 94,405.06.
- Import seeds `start_date` `2023-10-27`, `saved` `1094405.06`, `target_months` `36`, `target_amount` `1000000` once from the user's current bank figures (the workbook's `K31`/`L31`/`K34` literals are stale and intentionally not seeded); overrides stay unset.
- Parity reports the block's cells as informational — the app legitimately carries newer figures than the frozen workbook.
- The 總覽 page gains a `Money Master` card; the 月結 settings card gains the Money Master fields; the deposits view's manual-workbook note drops "money master" (the checklist step stays — it is still a user action, now done in-app).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: `GET /api/overview` additionally returns the `money_master` block (inputs, effective month/coming-save, progress and pace derivations), and the 總覽 page shows the `Money Master` card.
- `month-stat`: `PATCH /api/months/settings` accepts the six `money_master.*` fields with validation; `GET /api/months/settings` returns them.
- `time-deposits`: the reminder requirement's list of still-manual workbook steps loses "money master" — that step now happens in the app.
- `spreadsheet-trade-import`: import additionally seeds the `money_master.*` settings once; the Overview parity report gains informational comparisons for the `J29:N35` cells.

## Impact

- **Backend**: `backend/src/models.rs` (`MoneyMaster`, `MonthSettings`/`MonthSettingsPatch` fields, `OverviewResponse.money_master`), `backend/src/routes/months.rs` (key constants, settings GET/PATCH), `backend/src/routes/overview.rs` (block assembly + derivations), `backend/src/xlsx.rs` (`OverviewCached.money_master` + `parse_overview`), `backend/src/import.rs` (seed-once), `backend/src/parity.rs` (informational `J29:N35` comparisons). No migration — `app_meta` is generic key-value; no new endpoints, so the OpenAPI pin test is untouched.
- **Frontend**: `frontend/src/api.ts` types, `frontend/src/views/OverviewView.vue` (Money Master card), `frontend/src/views/MonthStatView.vue` (settings fields), `frontend/src/views/DepositsView.vue` (muted note rewording).
- **Docs**: `docs/overview.md`, `docs/MONTH_STAT_OVERVIEW.md`, `docs/DATA_FLOW.md`.
- **Reconfiguration**: a new challenge after 2026-10-26 is pure settings change (`start_date`, `saved`, optionally the targets); overrides must be cleared when switching or they suppress derivation.
