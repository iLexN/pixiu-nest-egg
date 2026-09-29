# Tasks

## 1. Backend setting + consumers

- [x] 1.1 In `backend/src/routes/months.rs` add the `overview.semi_liquid_target` `app_meta` key constant and a `0.25` default; extend `MonthSettings` (`models.rs`) with the effective `semi_liquid_target: f64` and `MonthSettingsPatch` with a nullable `semi_liquid_target`; validate finite `[0, 1)` in `update_settings` and `meta_put`/`null`-clear like `bill_amount`. Verify: inline test — patch `0.3` → settings returns `0.3`; patch `null` → `0.25`; patch `1.5` → 400 field error on `semi_liquid_target`.
- [x] 1.2 In `routes/overview.rs` read the setting and compute `vs_quarter_liquid = semi_total − ratio × liquid_assets`; expose the effective ratio on the overview response (for the footer label). Verify: inline test — meta set to `0.3` gives `total − 0.3 × liquid_assets`; unset behaves exactly as the current `0.25` code.
- [x] 1.3 In `calc.rs` add the ratio to `ForecastInput` and change `ref_check` to `semi_liquid − ratio × liquid_assets` (replacing `/ 4.0`); in `routes/forecast.rs` read the meta key and pass it in. Verify: existing ref-check test passes unchanged at the default; a new test with ratio `0.3` asserts the scaled check.

## 2. Parity

- [x] 2.1 In `parity.rs` change the `C14` comparison to recompute `response.semi_liquid.total − 0.25 × response.liquid_assets` and compare that against the workbook's cached `semi_liquid_vs_quarter`, independent of the stored setting. Verify: a parity test with `overview.semi_liquid_target` set to `0.3` still reports a match while the sheet's `C14` formula holds.

## 3. Frontend

- [x] 3.1 `frontend/src/api.ts`: add `semi_liquid_target` to `MonthSettings` and `MonthSettingsPatch`, and the effective-ratio field to the overview response type. Verify: `pnpm exec vue-tsc --noEmit` clean.
- [x] 3.2 `MonthStatView.vue` settings form: add a percent input for the ratio (draft `×100`, patch `÷100`, blank → `null` reset), mirroring the pool-rate field. Verify: `pnpm exec vue-tsc --noEmit` clean and the form loads/saves the value against a running backend.
- [x] 3.3 `OverviewView.vue`: render the footer label `與{configured %}流動相差` from the response's effective ratio instead of the literal `25%`. Verify: `pnpm build` succeeds.

## 4. Docs

- [x] 4.1 Update `docs/overview.md`: `C14` described as `total − configured ratio × 流動資產` (default `25%`), the `ref check` line updated to the same ratio, and the setting (key, default, `null` reset) documented. Verify: text matches the implemented behavior.

## 5. Verification

- [x] 5.1 `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` all pass.
