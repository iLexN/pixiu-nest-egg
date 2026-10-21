# Tasks

## 1. Backend

- [x] 1.1 Change `store::months::record_receipt_item` to upsert the keyed item: when `(month, auto_key)` already exists, UPDATE its `label` and `amount` instead of skipping; keep the monthless skip and the dismissal clear. Update the doc comment. Verify with an inline test: a stored `dep-end`-keyed item at the old amount becomes the new amount after a second `record_receipt_item`.
- [x] 1.2 Verify callers unchanged (`dep-start`, `dep-end`, `coupon`, `div`, `bond-end` all inherit the upsert). Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`. Verify: all pass.

## 2. Docs

- [x] 2.1 Update the receipt flows in `docs/deposits.md`, `docs/investments.md`, `docs/stocks.md`, and `docs/overview.md` where they say the item is "skipped when already stored" — now refreshed to the receipt amount. Verify: grep shows no stale "skipped when already stored" wording in `docs/`.

## 3. Verification

- [x] 3.1 Restart the real backend per AGENTS.md. Verify on a scratch DB (`WEALTH_DB=/tmp/...`): accept a pending `coupon:<id>` suggestion at the expected amount, then PATCH the coupon `received_amount` to a different figure — the stored item shows the received amount and the cash account gained it.
