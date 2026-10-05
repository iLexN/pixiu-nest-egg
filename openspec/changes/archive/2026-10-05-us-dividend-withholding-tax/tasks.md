# Tasks

## 1. Schema and model

- [x] 1.1 Add `backend/migrations/0027_dividend_withholding_tax.sql` (`ALTER TABLE dividends ADD COLUMN withholding_tax REAL;`) and document the column in `docs/database.md` (`dividends` table, next to `received_price`); verify `cargo test` applies the migration on a fresh test database.
- [x] 1.2 In `backend/src/models.rs`, add `withholding_tax: Option<f64>` and derived `gross_amount: Option<f64>` to `Dividend` (doc comments), and add `gross_amount: Option<f64>` (input-only) and nullable `withholding_tax: Option<Option<f64>>` to `DividendPatch`. In `backend/src/routes/mod.rs`, select `d.withholding_tax` in `DIVIDEND_SELECT` and fill both fields in `row_to_dividend` (gross = received + tax when both are present). Verify with `cargo build` and an existing dividend API test whose response shows `withholding_tax: null, gross_amount: null`.

## 2. Validation and receipt logic

- [x] 2.1 In `backend/src/calc.rs`, add `withholding_tax` to `DividendInput`/`ValidatedDividend`, plus `gross_amount: Option<f64>` on the input. Validate finite and non-negative values, tax ≤ gross, no tax without a resulting `received_amount`, and field errors named `withholding_tax`/`gross_amount`. Add a pure `net_from_gross(gross, tax) -> f64`. Verify with new unit tests in the inline `#[cfg(test)]` module: 12.76 − 3.83 ≈ 8.93; tax > gross is rejected; tax on pending is rejected; negative tax is rejected.
- [x] 2.2 In `backend/src/routes/dividends.rs::update`:
  - Reject `gross_amount` together with an explicit `received_amount` (400 on `gross_amount`).
  - When `gross_amount` is present, resolve tax (patch, else stored, else 0) and set `received_amount = gross − tax` before the existing manual_price default and validation.
  - Merge `withholding_tax` like the other nullable fields and write it in the UPDATE.
  - In the `(Some, None)` un-receive branch, also set `withholding_tax = NULL`.
  - Keep banking on the net `received_amount`.

  Verify with the API tests in 2.3.
- [x] 2.3 Add tests to `backend/tests/api.rs`, then verify with `cargo test`:
  - (a) A US 收訖 with `gross_amount` 12.76 and `withholding_tax` 3.83 stores received ≈ 8.93 and tax 3.83, reads back gross 12.76, and `ibkr.usd_cash` gains ≈ 8.93.
  - (b) Gross only stores received = gross and tax null.
  - (c) Tax > gross gives 400 on `withholding_tax`, and nothing changes.
  - (d) Gross + received together gives 400 on `gross_amount`.
  - (e) Tax on a pending dividend gives 400.
  - (f) Clearing `received_amount` reverses the IBKR credit and nulls the tax.
  - (g) Editing an already-received record with gross + tax recomputes received.
- [x] 2.4 Update `docs/stocks.md` ("Mark a dividend received": the PATCH body gains `gross_amount`/`withholding_tax` and received = gross − tax; the derived-fields line gains `gross_amount`) and `docs/DATA_FLOW.md` if its dividend receipt summary lists the PATCH fields. Verify the docs match the spec scenarios.

## 3. Frontend (US only)

- [x] 3.1 In `frontend/src/api.ts`, add `withholding_tax`/`gross_amount` to `Dividend` and `gross_amount`/`withholding_tax` to `DividendPatch`; verify `pnpm exec vue-tsc --noEmit`.
- [x] 3.2 In `frontend/src/components/DividendReceiveForm.vue`, for `market === 'US'` replace 實收派息 with 派息 (prefilled from `estimated_amount`) and Tax inputs, add a muted display-only line 「實收派息 ≈ 派息 − Tax」 formatted with `fmtMoney`, and submit `{ gross_amount, withholding_tax, received_price, bank_in }`. HK is unchanged. Coerce `type="number"` values with `Number`/`String`, not `.trim()`. Verify with `pnpm build` and by receiving a US dividend on a scratch backend: the stored net, tax, and IBKR credit all match.
- [x] 3.3 In `frontend/src/components/DividendForm.vue`, when editing a received US dividend, show 派息 (prefilled `gross_amount ?? received_amount`) and Tax instead of 實收派息 and send `gross_amount`/`withholding_tax`. Verify by editing a received US record on the scratch backend: the recomputed received amount matches.
- [x] 3.4 In `frontend/src/components/DividendTable.vue`, add a Tax column (`fmtMoney(withholding_tax)`) to the received (non-receivable) table and adjust the empty-row `colspan`; verify `pnpm build` and visually check the 已收記錄 table.

## 4. Integration

- [x] 4.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, and `openspec validate us-dividend-withholding-tax --strict`; verify all pass.
- [x] 4.2 Restart the backend on :8787 (`lsof -ti :8787 | xargs kill`, then `cargo run -p wealth-backend --bin wealth-backend` in the background) and verify `GET /api/dividends?market=US` returns `withholding_tax`/`gross_amount`.
