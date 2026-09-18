# Tasks

## 1. Schema and models

- [x] 1.1 Add `backend/migrations/0012_bonds.sql` creating `bonds` and `bond_coupons` (per design.md; `bond_id` FK, nullable `annual_rate`/`per_10k`/`received_amount`/`fixing_date`); verify `cargo test` migration tests pass and `sqlite3 data/wealth.db ".schema bonds"` shows the tables after running the backend once
- [x] 1.2 Add `Bond`, `BondCoupon`, `NewBond`, `BondPatch`, `NewBondCoupon`, `BondCouponPatch` (nullable-field serde pattern from `DividendPatch`) plus `BondStatus`/`CouponStatus` enums to `backend/src/models.rs`; verify `cargo build` compiles

## 2. Derivation

- [x] 2.1 In `backend/src/calc.rs`, implement `validate_bond`, `validate_coupon`, `bond_active` (maturity > today), `coupon_expected` (`per_10k × principal ÷ 10000`), `coupon_status` (received → 待定 → pending precedence), `coupon_variance`, and `next_pay_date`; verify unit tests cover: 待定 coupon with null rate/per_10k, expected `199.45 × 5 = 997.25`, received overrides fixed status, variance sign, maturity today counts as matured

## 3. API

- [x] 3.1 Add `backend/src/routes/bonds.rs` with `GET /api/bonds?status=`, `POST /api/bonds`, `PATCH /api/bonds/:id`, `DELETE /api/bonds/:id` (cascade coupons), `GET /api/bonds/summary` (`today`, `totals.active_principal`, `active` + `next_pay_date` + coupons, `matured`, `upcoming_coupons`), and `GET/POST /api/coupons`, `PATCH/DELETE /api/coupons/:id`; wire in `routes/mod.rs`; verify with curl that PATCH sets a coupon's rate (待定→pending) and `received_amount` (→received), and DELETE on a bond removes its coupons

## 4. Workbook import and parity

- [x] 4.1 Extend `backend/src/xlsx.rs` to parse the `債券` sheet: registry rows under the `end` header (label/issue/principal/serial maturity) and coupon blocks (label row's `發行編號…` matched to the bond, then header + rows until blank; `待定` cells → NULL); capture cached `B1` and per-coupon interest cells; verify parse output against `財富分析報告.xlsx` yields one bond and six coupons (3 fixed, 3 unfixed)
- [x] 4.2 Extend `backend/src/import.rs` to insert bonds (key `issue_no`, fallback label+principal+maturity) and coupons (key bond+pay_date) idempotently, importing past-pay-date coupons as received with `received_amount` = sheet interest; verify a second `import_xlsx` run reports all skipped and adds no rows
- [x] 4.3 Extend `backend/src/parity.rs` with a bonds section comparing active principal total vs cached `B1` and per-coupon `expected` vs cached interest (skipping 待定); verify `check_parity` reports matches against the workbook's cached figures

## 5. Frontend

- [x] 5.1 Add `債券` nav group (after MPF) with a single `總覽` sub-tab in `App.vue` (market toggle hidden while active); verify nav shows 股票, 定期, MPF, 債券 in order and the toggle hides on 債券
- [x] 5.2 Add `api.ts` types/endpoints and `frontend/src/views/BondsView.vue`: totals strip, per-active-bond coupon table (待定/pending/received chips, inline rate entry, 收訖 action, variance display), matured-bonds section, add/edit bond + coupon forms; verify `pnpm exec vue-tsc --noEmit` and a manual UI pass: fix a 待定 coupon's rate, mark it received, see variance and totals update
- [x] 5.3 Show the workbook reminder hint (Overview/Month Stat unmigrated) on the 債券 page, matching the MPF page's pattern

## 6. Docs and verification

- [x] 6.1 Update `docs/DATA_FLOW.md` with the bond data flow (registry, coupon lifecycle, summary derivation, import/parity) and `AGENTS.md` roadmap to mark 債券 migrated; verify docs match implemented behavior
- [x] 6.2 Run full verification: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, plus a real `import_xlsx` (twice — second run all-skipped) + `check_parity` against `財富分析報告.xlsx`
