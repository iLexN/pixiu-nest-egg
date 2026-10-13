# Tasks

## 1. Receipt-based activity in calc

- [x] 1.1 In `backend/src/calc/deposits.rs`, add `received: bool` to `DepositFacts`, change `deposit_active` to take the facts and return `!received`, and drop the `today` parameter from `active_totals`, `active_month_rollup`, and `bank_rollup`. Update the doc comments: receipt rule, parity keeps the sheet rule. Verify that `cargo build -p wealth-backend` reports only caller errors, which are fixed in 2.x.
- [x] 1.2 Update the `workbook_deposits()` fixture so rows with `end_date <= 2026-09-15` are `received: true`, keeping the existing 445,000 / 365,000 / 80,000 / month-bucket assertions. Add tests:
  - a matured-but-unreceived deposit counts in all three aggregates
  - a received deposit with a future end date counts in none

  Verify with `cargo test -p wealth-backend calc::deposits`.

## 2. Callers and loaders

- [x] 2.1 In `backend/src/store/deposits.rs`, make `active_facts` and `year_facts` select `received_at` and set `received` from it. Fix the `calc/year_review.rs` test fixtures. Verify that `cargo test -p wealth-backend year_review` passes.
- [x] 2.2 Pass the received flag in `routes/deposits.rs::summary` and `routes/family.rs::summary` (`received_at.is_some()`), and in `service/months.rs::live_totals_input`, which reads it through `active_facts`. Drop the now-unused `today` arguments. Add route/service tests:
  - an own deposit with a past `end_date` and no `received_at` still counts in `/api/deposits/summary` active totals and in live `liquid_assets`, and stops counting after `receive`
  - a family deposit with a past `end_date`, unreceived, counts in the holder's `active_principal`

  Verify with `cargo test -p wealth-backend deposits family months`.
- [x] 2.3 In `backend/src/parity.rs::check_deposits`, build facts with `received = end_date <= today` (the sheet rule). Verify that `cargo test -p wealth-backend parity` passes.

## 3. Docs

- [x] 3.1 Update the docs:
  - `docs/deposits.md`: the totals line at ~L38, the L57 paragraph, and family 活躍本金 at ~L68. Totals and rollups are now receipt-based, matching the list. Note the 收訖 → 存入活期 workflow and that parity keeps the end-date rule.
  - `docs/overview.md`: make sure the live 已定期 wording and the forecast `locked` anchor note (~L56, "not the live 'end date not passed' total") say "live active (unreceived) total".

  Verify by grepping `docs/` for "end date not passed" and "derive from `end_date`" and confirming no stale wording remains.

## 4. Integration

- [ ] 4.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check`; all pass.
- [x] 4.2 Restart the backend on the live DB (`lsof -ti :8787 | xargs kill`, then `cargo run -p wealth-backend --bin wealth-backend` in the background) and check, while #18/#19 are still unreceived:
  - `/api/overview` `semi_liquid.deposits` = 500000
  - `liquid_assets` ≈ 145,000 higher than before the change
  - `/api/family/deposits/summary` 媽媽 `active_principal` includes 100000
