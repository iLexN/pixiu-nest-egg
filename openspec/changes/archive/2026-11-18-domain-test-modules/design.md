# Design

## Decisions (from grilling)

- **D1 — Separate test binaries.** One `tests/<domain>.rs` per domain + `tests/common/mod.rs`, the canonical Rust layout. The cross-crate boundary is the enforcement: anything two files share *must* live in `common`, which is what turns the helper pile into a named interface. Bonus: `cargo test --test deposits` runs one domain. _Rejected_: single `tests/api.rs` root with `mod` submodules — siblings could still import each other, so nothing forces helpers into `common` and the drift can resume.
- **D2 — 14 files: domain-per-surface plus two thematic.** Domain files mirror the backend's own module names so a test is found by asking "what surface does it exercise". `receives.rs` and `market_history.rs` group the two cross-cutting protocols (credit-cash→month-item; audit-trail recording) rather than splitting them by entity — those tests exercise the protocol, not the entity. Micro-files (`months` 1, `errors` 1, `cash_accounts` 1, `year_review` 2) stay for uniformity. _Rejected_: a `misc.rs` grab-bag for the small ones — that recreates the original problem at smaller scale.
- **D3 — Every helper in `common`, no exceptions.** Even the single-domain ones (`create_family_deposit`, `ym_add`, `history_rows`, `set_price`, `buy_trade`, `create_stock_with_ticker`): the alternative makes a helper's home depend on today's call-graph breadth — invisible knowledge that drifts the first time a second domain reuses it. With this rule there is one answer to "where does a test helper go".
- **D4 — Flat `common/mod.rs`.** ~16 helpers / ~300 lines is a readable module; submodules (`common/{app,seed,assert}`) would re-raise the "which file" question at a smaller scale for no payoff.

## Test-by-file map

| File | Tests |
|---|---|
| `stocks.rs` | stock_crud_and_conflicts, stock_with_trades_cannot_be_deleted, stock_order_can_be_reordered_per_market, price_upload_updates_matches_and_reports_the_rest, price_upload_rejects_a_body_without_a_stocks_array |
| `trades.rs` | trade_create_filter_edit_delete, moving_a_trade_across_markets_is_rejected, us_trade_settles_the_usd_broker_cash |
| `summary.rs` | summary_recomputes_after_edit, summary_marks_stocks_without_a_price_as_excluded |
| `market_history.rs` | manual_price_patch_records_market_history, trade_create_and_delete_record_history, note_only_stock_patch_records_nothing, summary_reads_and_freeze_write_no_history |
| `deposits.rs` | deposit_crud_and_filters, deposit_validation_errors_name_the_fields, deposit_summary_reports_upcoming_rollups_and_year_tables, deposit_placement_debits_the_account_and_delete_restores_it |
| `receives.rs` | deposit_receive_credits_cash_and_records_the_month_item, bond_receive_credits_principal_and_records_the_month_item, coupon_and_dividend_receive_banks_in_and_records_the_item, coupon_reversal_follows_the_recorded_account_after_default_moves, deposit_receive_rejects_a_wrong_currency_account, deposit_receive_without_bank_in_changes_no_account, hk_dividend_receive_into_a_chosen_account_credits_it |
| `dividends.rs` | dividend_lifecycle_freezes_its_snapshots, dividend_receipt_defaults_price_to_current_stock_price, dividend_receipt_without_stock_price_stores_null, dividend_us_receipt_with_withholding_tax, dividend_validation_errors_name_the_fields |
| `family.rs` | family_deposit_crud_and_filters, family_deposit_validation_errors_name_the_fields, family_deposit_receive_lifecycle_touches_no_ledger, family_holder_note_and_summary, family_deposits_do_not_touch_user_totals |
| `forecast.rs` | forecast_items_crud_and_validation, forecast_bill_amount_setting_round_trips, forecast_convert_turns_a_plan_into_a_deposit |
| `months.rs` | month_recapture_snapshots_start_cash_from_the_cash_sum |
| `cash_accounts.rs` | cash_account_crud_and_rules |
| `settings.rs` | settings_round_trips_at_the_new_path, fx_rate_round_trips_and_clears, fx_rate_rejects_bad_values_and_currencies |
| `year_review.rs` | year_review_reports_manual_figures_and_feeds_sold_pl, yearly_patch_stores_and_clears_sold_pl |
| `errors.rs` | errors_carry_status_and_field_messages |

## Mechanics

- Test bodies and helper bodies move verbatim; only the `use` header changes (each file gets the `mod common; use common::*;` pair plus whatever imports its tests reference).
- `common/mod.rs` is not a test target (files under a `tests/` subdirectory don't compile as binaries); helpers become `pub`.
- `cargo test` output regroups from `tests/api.rs`'s single `running 46 tests` block into per-binary blocks — count stays 46.

## Risks / Trade-offs

- **[A test lands in the wrong file]** → the map above is the contract; wrong-file placement is caught by asking "what surface does this test exercise", not by compile errors — implementation reviews the split against the map.
- **[Slower `cargo test` link]** → 14 test binaries link separately; each is small and cached — negligible on this codebase.
- **[A helper gets inlined into a domain file anyway]** → D3's absolute rule plus code review; nothing compiler-level prevents it, same as today's discipline around `routes/`.
