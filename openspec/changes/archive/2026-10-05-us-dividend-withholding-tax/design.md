# Design

## Context

See proposal.md (Why). `received_amount` already holds the **net** amount: the live US rows show it at about 70% of `estimated_amount` (VOO 12.7582 → 8.93, GOOG 4.84 → 3.39). The IBKR USD cash credit, yields, yearly rollups, parity, and 利息 all read `received_amount`. The receipt flow is `DividendReceiveForm.vue` → `PATCH /api/dividends/:id` → `routes/dividends.rs::update`, which merges the patch onto the stored row, validates through `calc.rs::validate_dividend`, writes the row, and then handles the NULL↔value receipt transition (bank-in / reversal) in the same transaction. The workbook importer only inserts new rows, deduped by key, so a new column stays NULL and is never overwritten.

## Goals / Non-Goals

**Goals:**
- The backend derives the net amount from gross and tax, and keeps the tax.
- Every existing consumer of `received_amount` stays untouched.

**Non-Goals:**
- No backfill of tax on historical rows (estimated − received mixes estimate error into the tax).
- No change to `variance`, to the yield definitions, or to which amount is banked.
- No re-banking when a received record's amount is edited afterwards. That is existing behavior: only the NULL↔value transition banks or reverses.
- No HK UI change, although the backend accepts tax for HK too.
- No yearly tax totals yet. Storing the column makes them possible later.

## Decisions

1. **Store `withholding_tax`; derive `gross_amount` on read.** Two of {gross, tax, net} must be stored. `received_amount` (net) is already stored and widely read, so the second stored value is tax, which the user asked to keep. *Alternative:* store gross and derive tax. Rejected because it gives the same information, and tax is the value the user reasons about.
2. **`gross_amount` is an input-only patch field.** It is resolved into `received_amount` before the merge/validate/transition logic, so the banking code sees a normal net `received_amount` and needs no changes. Tax resolution order: the request's value, else the stored value, else 0. This lets a gross-only edit keep the existing tax. *Alternative:* a dedicated receive endpoint. Rejected: there is no new OpenAPI operation, and the existing PATCH already owns the receipt transition.
3. **Reject gross + received in one request.** If both were accepted, one would silently win. A field error on `gross_amount` is clearer.
4. **Tax only on received records; cleared on un-receive.** A pending row has no withholding yet. Clearing `received_amount` sets tax to NULL in the same UPDATE, so a re-receive starts clean.
5. **Rounding.** `received_amount` is stored as the plain f64 `gross − tax`. This follows the project rule against rounding stored values, and display formatting stays at 2 decimal places.
6. **The frontend preview is display-only.** The US forms show `派息 − Tax` live, but they submit `gross_amount` + `withholding_tax`. The value used is the one the backend returns. On receipt, 派息 is prefilled from `estimated_amount`. When editing a received US record, 派息 is prefilled from `gross_amount ?? received_amount` and Tax from `withholding_tax`.

## Risks / Trade-offs

- [A tax-only patch on a received record changes the derived gross while the net stays fixed] → The UI always sends gross and tax together. The spec states that tax is stored as given and that `received_amount` is recomputed only when `gross_amount` is supplied.
- [Old US rows show no gross or tax] → This is expected. The received table shows a blank Tax for them.
- [f64 subtraction (e.g. 12.76 − 3.83 = 8.929999…)] → Stored unrounded, as with every other money value; it displays as 8.93. API tests compare with a tolerance.

## Migration Plan

`0027_dividend_withholding_tax.sql`: `ALTER TABLE dividends ADD COLUMN withholding_tax REAL;`. It is applied automatically on startup. Rollback means ignoring the column; the existing code never reads it.
