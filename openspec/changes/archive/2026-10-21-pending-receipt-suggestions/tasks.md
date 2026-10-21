# Tasks

## 1. Backend

- [x] 1.1 In `calc::suggestions::build_suggestions`, drop the `received` check for dividends and coupons; skip receipts whose `amount` is `None` (待定 coupon, estimate-less dividend). Update the `SuggestionReceipt` and `build_suggestions` comments — pending receipts now suggest too; only `auto_interest` stays received-only. Verify: `cargo test`.
- [x] 1.2 Update `suggestions_cover_each_event_kind`: `div:9` (pending, estimate 720) is now suggested at +720; `coupon:10` (待定) stays absent. Verify: the test asserts both.

## 2. Docs

- [x] 2.1 Update `docs/overview.md` Suggestions paragraph — `div:<id>`/`coupon:<id>` suggest +received or pending expected/estimated figure, skipped while no amount is known. Verify: the paragraph no longer says "received amounts" only.

## 3. Verification

- [x] 3.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`. Verify: all pass.
- [x] 3.2 Restart the real backend per AGENTS.md. Verify: `GET /api/months/2026-10` suggests `coupon:<id>` at the expected amount for the pending 2026-10-23 silver-bond coupon, and the interest breakdown still shows it as pending.
