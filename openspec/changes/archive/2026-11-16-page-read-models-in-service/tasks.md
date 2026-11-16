# Tasks

## 1. Shared helpers

- [x] 1.1 Add `calc::history_years(years: impl IntoIterator<Item = i32>, today: NaiveDate) -> Vec<i32>` (distinct + current year, sorted) in `calc/mod.rs` with a unit test.
- [x] 1.2 Add `routes::parse_status(raw, active, done)`, `routes::parse_received_at(Option<&str>)`, `routes::finite_or(field, Option<f64>, default)` to `routes/mod.rs`; use them in the deposits/family/bonds/dividends list handlers and the deposits/bonds/family receive handlers.
- [x] 1.3 Add `facts_of(&Deposit)` in `store::deposits`, `facts_of(&FamilyDeposit)` in `store::family`, `facts_of(&Dividend)` in `store::dividends` — parsing the stored date once into `StoreError::Corrupt` with the existing messages.
- [x] 1.4 Move family receive's already-received guard into `store::family::mark_received` → `StoreError::Conflict("family deposit {id} is already received")`; add a store test for the Conflict.

## 2. Move the read models

- [x] 2.1 `service::deposits::summary` + `DepositSummaryResponse`; route re-exports and delegates; move `matured_unreceived_deposit_counts_until_received` into `service::deposits::tests` calling `summary(&pool)` and `service::deposits::receive`.
- [x] 2.2 `service::bonds::summary` + `BondSummaryResponse`/`BondTotals`/`BondWithCoupons`/`UpcomingCoupon`; route re-exports and delegates; move `matured_unreceived_bond_counts_until_received` into `service::bonds::tests`.
- [x] 2.3 New `service/family.rs` (read side only): `summary` + `FamilyDepositSummary`/`FamilyHolderSummary`; route re-exports and delegates; move `matured_unreceived_deposit_counts_in_the_holders_active_principal` into `service::family::tests`.
- [x] 2.4 `service::dividends::summary(pool, market)` + `DividendSummaryResponse`; route parses `market` then delegates.
- [x] 2.5 `service::aia::summary` + `AiaSummaryResponse`/`PolicyWithEvents`; move `parse_day` into `service::aia` returning `StoreError::Corrupt` and share it with the event path.
- [x] 2.6 `service::mpf::overview` + `MpfResponse`; route delegates.

## 3. Docs

- [x] 3.1 Update wiring notes in `docs/deposits.md`, `docs/investments.md`, `docs/DATA_FLOW.md` where they describe these summaries.
- [x] 3.2 Amend `docs/adr/0005-layered-backend-seams.md`: page read models (`GET …/summary`) live in `service/`; routes only parse the query and call it (dated 2026-11-16).
- [x] 3.3 Add a "Page read model" term to `GLOSSARY.md`.

## 4. Verify

- [x] 4.1 `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- [x] 4.2 `cd frontend && pnpm gen:api` → `api-types.ts` diff empty; `pnpm build && pnpm exec vue-tsc --noEmit`.
- [x] 4.3 Restart the backend detached on :8787 and diff the summary endpoints' JSON against the pre-change captures — byte-identical.

## 5. Follow-up simplification

- [x] 5.1 `store::stored_date(column, raw)` centralises the stored `YYYY-MM-DD` → `StoreError::Corrupt` idiom; applied at every site in `store/{deposits,family,dividends}.rs` and `service::aia::parse_day`. Other store copies are left for a follow-up.
- [x] 5.2 `routes::parse_order(Option<&str>)` centralises the `?order=desc` flag across the five list handlers.
