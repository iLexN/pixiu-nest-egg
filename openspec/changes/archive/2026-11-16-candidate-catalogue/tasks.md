# Tasks

## 1. Pure catalogue (`calc/candidates.rs`)

- [x] 1.1 Add `CandidateKey`, with a `Display` impl that renders the stored string and a `source()` method. Add `Candidate { key, month, category, label, amount }`. Add a test pinning each variant's rendered string: `dep-start:7`, `dep-end:7`, `bond-end:7`, `coupon:7`, `div:7`, `ibkr:7`, `trade:7`, `aia-pay:7`, `pool-input`.
- [x] 1.2 Add `SuggestionBond { id, maturity_date, principal, credited_amount, label }` and a `bonds` field on `SuggestionEvents`.
- [x] 1.3 Add one builder per source (design D1): `dep_start`, `dep_end`, `bond_end`, `coupon`, `dividend`, `ibkr`, `trade`, `aia_pay`, `pool_input`. Each owns its key, month, sign and label, with the "定期" label fallback for deposits and `credited_amount ?? principal` for bond-end. Add unit tests for the sign, the month, the label fallback and the `None` cases.
- [x] 1.4 Rewrite `build_suggestions` over the builders and include bond-end, keeping its signature. Verify that the existing `calc/suggestions.rs` tests pass unchanged, and add tests for a bond-end suggestion before 收訖 and at the credited amount after it.

## 2. Store half (`store/months.rs` and the event mappings)

- [x] 2.1 Replace `record_receipt_item`, `delete_auto_item` and `delete_auto_items_by_key` with `record_candidate(conn, &Candidate)` and `retire_candidate(conn, &CandidateKey)` (design D2). Port the existing store tests: refresh, monthless skip and dismissal clearing for record; any-month delete with dismissals for retire.
- [x] 2.2 Add `bonds::bond_end_events(pool, range)`, which selects bonds by `maturity_date`, and fill `SuggestionEvents.bonds` in `service::months::load_events`. Verify with a `GET` month test that a maturing bond shows a `bond-end:<id>` suggestion with `source` `bond`.
- [x] 2.3 Add by-id, in-transaction event loaders (design D3): `deposits::suggestion_event`, `bonds::coupon_event`, `bonds::bond_end_event`, `dividends::suggestion_receipt` (HK only, `None` for US) and `ibkr::transfer_event`. Each shares its row→event function with the existing range query. Verify that `cargo test` passes.

## 3. Write paths

- [x] 3.1 `service::deposits`: `place` and `receive` load the event, build `dep_start`/`dep_end` and call `record_candidate`; `delete` calls `retire_candidate(DepStart)`. Verify that the existing deposit tests pass and that a new test shows an unlabeled deposit's dep-start item is stored as "定期".
- [x] 3.2 `service::bonds::receive` records `bond_end` built from the post-write row. Add a test where `credit_amount` 99500 on principal 100000 records an item of 99500, and one where `bank_in=false` records the principal.
- [x] 3.3 `service::bonds::update_coupon`: on 收訖 it records `coupon` built from the post-update row; on clearing it calls `retire_candidate(Coupon)`. Add a test where `pay_date` is moved and `received_amount` is cleared in one PATCH, leaving no item in any month.
- [x] 3.4 `service::dividends::update`: same as 3.3 for `div`, with the HK-only rule coming from the loader (design D5) and the `Market::Hk` branch removed. Add a test that clearing after a `pay_date` move leaves no item, and verify that the US dividend tests still record no item.
- [x] 3.5 `service::ibkr`: `create_transfer` records `ibkr` built from the inserted row; `delete_transfer` calls `retire_candidate(Ibkr)`. Verify that the existing ibkr tests pass.
- [x] 3.6 Add parity tests (month-stat "Recorded candidates match their suggestion") on `connect_memory`. For each of dep-start, dep-end, bond-end, coupon, div and 轉入, the suggestion seen before the write and the item stored by the write must agree on key, month, category and label, and the amount must be the 收訖 amount. Verify with `cargo test parity`.
- [x] 3.7 Remove the leftover hand-built key strings: no `format!("dep-start:` (or other prefixes) and no `format!("{}-01", &…[..7])` should remain in non-test code under `backend/src/service/`. Verify with a grep.

## 4. Docs

- [x] 4.1 Update `docs/DATA_FLOW.md`, `docs/deposits.md` and `docs/investments.md` wherever they name `record_receipt_item` or describe the Candidate wiring. Note the bond-end Candidate and the retire rule. Verify that grepping for `record_receipt_item` under `docs/` returns nothing.

## 5. Verify

- [x] 5.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`.
- [x] 5.2 Run `cd frontend && pnpm gen:api` and confirm the `api-types.ts` diff is empty, then run `pnpm build && pnpm exec vue-tsc --noEmit`.
- [x] 5.3 Restart the backend detached on :8787. Confirm that `GET /api/months/<current>` answers, and that the keys of its stored items match what the DB held before the change.

## Workflow follow-up

- Archive with `openspec-archive-change` and sync the delta specs.
- Run `trail check` before committing.
- Next change: `cash-movement-module` (see the architecture review, candidate #2).
