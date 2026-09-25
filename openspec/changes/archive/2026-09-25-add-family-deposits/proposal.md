# Proposal

## Why

The workbook's `Mum` and `Dad` sheets record time deposits the user places on behalf of family members (mum, dad, and Irene) — one rolling principal per person with the bank reference, end date, stepped-rate schedule, interest paid, and an `end` marker per rollover, plus a note block (Mum's AIA policy numbers). That money is outside the user's own finances, so it cannot live in the existing `deposits` table without risking contamination of 定期!B1, Overview 已定期, live 總數/流動資產, Month Stat 利息, and the 回顧 figures. Migrating these sheets needs a registry with the same record-keeping as 定期 but hard-isolated from every "my money" aggregate.

## What Changes

- Add a standalone **family 定期 registry**: deposits keyed by a required `holder` (free text such as 媽媽 / 爸爸 / Irene) with `label` (bank ref), `bank`, `principal`, `interest`, optional `start_date`, required `end_date`, and a free-text `note` for the bank's stepped-rate schedule. There is **no numeric rate** field.
- Same **收訖 lifecycle as 定期**: a deposit stays listed (flagged 已到期未收 once past its end date) until the user confirms receipt, optionally correcting the interest actually paid; 取消收訖 reverts. Unlike 定期, receipt never credits a cash `manual_assets` row and never records a Month Stat item or suggestion — family money never enters the user's ledger.
- A **per-holder free-text note** (e.g. Mum's AIA 人壽保險 / 危疾保險 policy lines), editable inline, so `Mum!A1:B3` has a home.
- A per-holder **summary**: unreceived deposits earliest-maturity first, active principal (end date in the future), and a year-filtered history.
- New top-level **家人** navigation group (after 年結) with one sub-tab **定期** hosting the page.
- **No workbook import and no parity check** for these sheets — the user enters the rows by hand; `Mum`/`Dad` stay in the workbook until then.

## Capabilities

### New Capabilities

- `family-deposits`: registry, validation, derived fields, 收訖/取消收訖 lifecycle (without bank-in or ledger side effects), per-holder note, per-holder summary and history, and the guarantee that none of it feeds the user's own totals.

### Modified Capabilities

- `app-navigation`: the group list gains 家人 (after 年結) containing a 定期 sub-tab; the market toggle stays hidden for it.

## Impact

- **Backend**: new migration creating `family_deposits`; new models; new `routes/family.rs` mounted under `/api/family/...`; per-holder notes stored in `app_meta` via the existing meta helpers. No change to `deposits`, `months`, `overview`, `year_review`, `import`, or `parity` code paths.
- **Frontend**: new `FamilyDepositsView.vue`, API client functions/types, and a 家人 group in `App.vue`.
- **Specs / docs**: new `family-deposits` spec, `app-navigation` delta, `AGENTS.md` roadmap/data-flow and `docs/DATA_FLOW.md` updates.
- **Data**: additive only — a new table and new `app_meta` keys; existing rows and figures are untouched.
