# Design

## Context

The workbook code is isolated. Only `backend/src/import.rs`, `backend/src/parity.rs`, `backend/src/bin/import_xlsx.rs`, `backend/src/bin/check_parity.rs`, and `backend/tests/import.rs` reference `crate::xlsx`/`import`/`parity`. No route, service, store, or frontend code imports them; binaries are auto-discovered from `src/bin/` (no `[[bin]]` entries). `calamine` is used only by `xlsx.rs`.

The import wrote data that runtime code still reads, so all of it stays:

- `app_meta` seed marks (`service/summary.rs` reads `seed_max_percent_key`/`seed_max_amount_key`)
- `mpf_accounts.seed_max_*`
- synthetic `market_history` rows
- `year_review` overrides
- settings keys

New deposits and stocks already append via `COALESCE(MAX(sort_order), 0) + 1` (`store/deposits.rs`, `store/stocks.rs`), so the specs' new "appends" wording describes existing behavior.

## Goals / Non-Goals

**Goals:**
- Delete the workbook reader, importer, parity checker, their binaries and tests, and `calamine`, with `cargo test`/clippy/fmt green.
- Specs and docs no longer describe import or parity; behavior they still specify is unchanged.

**Non-Goals:**
- Dropping or renaming seed/synthetic columns or `app_meta` keys. That would need a migration, and runtime still reads them.
- Removing the spreadsheet provenance notes in specs and code comments ("mirrors `Overview!B1`", "the sheet's 25% literal"). They document where a formula came from and aren't a dependency.
- Deleting the local `財富分析報告.xlsx` or touching `data/wealth.db`. Both are the user's files.
- Changing `import_prices` or `prices.rs`.

## Decisions

- **Delete outright, no feature flag.** The tooling has no remaining consumer, and git history preserves it if it's ever needed.
- **Remove helpers that become dead.** After deleting the modules, clippy with `-D warnings` reports `pub(crate)`/private items that only the importer used (store insert helpers, key constructors, xlsx-only models). Delete them. `pub` items in the library crate don't warn, so also grep for each symbol `import.rs`/`parity.rs` called and delete any that has no remaining caller outside tests. Keep any symbol a runtime path still uses (e.g. `seed_max_percent_key`).
- **Keep `routes::today()` and `store::today()`.** `tests/import.rs` used one of them, but runtime code uses both.
- **Spec restructuring:** OpenSpec forbids dropping a scenario inside MODIFIED and has no scenario rename. So the two requirements that lose a scenario are REMOVED and re-ADDED under new names, with identical surviving behavior:
  - `time-deposits` "Upcoming deposit view" becomes "Unreceived deposit list" + "Receipt-based deposit totals"
  - `stock-portfolio-summary` "Manual stock display order" becomes "Stock display order"
- **Retire the whole `spreadsheet-trade-import` capability** with `retire_capabilities: true`, so archive deletes its main spec instead of leaving it empty.
- **Docs:** delete `docs/import-parity.md` and `docs/MONTH_STAT_OVERVIEW.md` (the workbook analysis behind a finished migration). Rewrite the README quick start: build, run, and the backend creates an empty DB via migrations. Rewrite the AGENTS.md "Import and parity check" and "Migration status" sections.

## Risks / Trade-offs

- [No way to rebuild the DB from the workbook] → Accepted: the workbook is stale for every migrated section, so a rebuild would lose data anyway. Mitigation: the user backs up `data/wealth.db`, noted as a follow-up.
- [Removing a "dead" `pub` helper that something still uses] → The compiler catches it. `cargo build`/`test` after each deletion step.
- [Docs or comments still mention import] → The final task greps for `import_xlsx`, `check_parity`, `parity`, and `xlsx` across `backend/src`, `docs/`, `README.md`, and `AGENTS.md`, and checks every remaining hit is intentional (`import_prices`, provenance notes).

## Migration Plan

No schema or data change. Restart the backend after merge. Rollback is reverting the commit.
