# Tasks

## 1. Backend — workbook cells and live totals

- [x] 1.1 In `xlsx.rs`, add `UsAccountCached` (美股 `B1`/`B2`/`B4`/`B5`/`B7` cached values) onto `MarketSheets`/`WorkbookData` and extend `OverviewCached` with the `A3:C18` block cells (`B3:B10`, `C3:C9`, `A13`, `B14:B18`, `C14`, `B1`, `H1`, `J1`); verify by unit test parsing the real workbook fixtures used by existing xlsx tests
- [x] 1.2 In `calc.rs`, rename `LiveTotalsInput.deposits_active_total` → `deposits_active_principal` and add `ibkr_hkd_cash`/`ibkr_usd_cash`; `live_totals` uses principal and adds `usd_cash × rate + hkd_cash` to both totals; update the existing live-totals tests to assert principal-only deposits and the IBKR cash terms
- [x] 1.3 In `routes/months.rs`, update `live_totals_input` to pass `active_totals(..).principal` and read the two IBKR cash `app_meta` keys; `cargo test` still passes

## 2. Backend — overview endpoints

- [x] 2.1 In `models.rs`, add `OverviewResponse`, `OverviewAssetRow`, `SemiLiquid`, `IbkrBlock`, `IbkrPatch` (absent-vs-null fields); verify `cargo check`
- [x] 2.2 Create `routes/overview.rs`: `GET /api/overview` (headline totals, ordered asset rows incl. manual `asset` rows and IBKR, shares, 半流動資金 block with `vs_quarter_liquid`/`share`, ibkr block) and `GET`/`PATCH /api/ibkr` (validate finite ≥ 0, `null` clears via `meta_put`); register both in `routes/mod.rs`; verify `cargo test` covers the derivation and validation paths
- [x] 2.3 In `import.rs`, seed `ibkr.*` `app_meta` keys from `UsAccountCached` only when unset and extend the import report; verify a second import leaves edited values untouched (existing idempotency test pattern)

## 3. Backend — parity

- [x] 3.1 In `parity.rs`, add `check_overview` comparing `B3:B10`, `C3:C9`, `A13`, `B14/B15/B18`, `C14`, `B1`/`H1`/`J1`, and 美股 `B1`/`B2`/`B4`/`B5`/`B7` against computed/stored values with informational outcomes for user-edited manual cells; verify `check_parity` runs and reports the new section

## 4. Frontend

- [x] 4.1 In `api.ts`, add `OverviewResponse`/`OverviewAssetRow`/`SemiLiquid`/`IbkrBlock` types and `overview()`, `getIbkr()`, `updateIbkr()`; verify `vue-tsc --noEmit`
- [x] 4.2 Create `views/OverviewView.vue` (headline strip, asset table with inline amount edit for manual rows, 半流動資金 block, IBKR readout) and wire the new first `總覽` group/tab in `App.vue`; verify the page renders and inline edits persist on reload
- [x] 4.3 In `SummaryView.vue`, add the US-only IBKR card (four inputs + derived net/net%/computed total/diff); verify it loads and saves via `PATCH /api/ibkr` and is absent for HK

## 5. Docs and verification

- [x] 5.1 Update `docs/DATA_FLOW.md` (Overview section + live-totals correction note), `AGENTS.md` roadmap (Overview A3:C18 + IBKR migrated; remaining Overview pieces listed), and the implemented-scope note in `docs/MONTH_STAT_OVERVIEW.md`
- [x] 5.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, then `import_xlsx` and `check_parity` against `財富分析報告.xlsx`; all pass with the Overview parity section reporting only expected informational drift
