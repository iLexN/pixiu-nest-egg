# Tasks

## 1. Schema and models

- [x] 1.1 Add `backend/migrations/0016_itemize_entertainment.sql` rebuilding `month_items` with the extended category CHECK (`adjustment`/`extra_spend`/`income`/`entertainment`) and `exclude_from_living INTEGER NOT NULL DEFAULT 0`, migrating each nonzero `month_stats.entertainment` into a single `entertainment` item (label 娛樂支出), recreating `idx_month_items_auto`, and `ALTER TABLE month_stats DROP COLUMN entertainment`; verify the migration applies cleanly on `data/wealth.db` and a scratch DB
- [x] 1.2 Update `backend/src/models.rs`: `MonthItemCategory::Entertainment`; `exclude_from_living` on `MonthItem`/`NewMonthItem`/`MonthItemPatch`; `MonthStat.entertainment` → derived `entertainment_sum`; drop `MonthStatPatch.entertainment`; verify `cargo build`

## 2. Derivation

- [x] 2.1 In `backend/src/calc.rs`: `MonthItemSums` gains `entertainment` + `entertainment_excluded`; `MonthStatRow` drops `entertainment`; `living_spend` subtracts both exclusion sums; pool chain and yearly aggregates read `sums.entertainment`; verify unit tests cover: flagged entertainment lowering `living_spend` while still counting in `entertainment`, unflagged entertainment not touching `living_spend`, and the pool balance unchanged by the move

## 3. API

- [x] 3.1 In `backend/src/routes/months.rs`: thread the new column through item SELECT/INSERT/UPDATE; validate `exclude_from_living` only on `entertainment` items (field error otherwise; cleared when category changes); remove `entertainment` from `MonthStatPatch` handling; verify with curl that a flagged entertainment item round-trips, a flagged adjustment is rejected, and `entertainment_sum` matches the items

## 4. Workbook import and parity

- [x] 4.1 In `backend/src/xlsx.rs`/`import.rs`: materialize O cells as a single `entertainment` item (cached total, formula text as `note`, `exclude_from_living` 0) instead of the scalar column; verify a scratch-DB import reproduces every month's 娛樂支出
- [x] 4.2 In `backend/src/parity.rs`: compare sheet O against the derived entertainment sum; verify `check_parity` stays clean

## 5. Frontend

- [x] 5.1 `api.ts`: `MonthItemCategory` gains `'entertainment'`; `exclude_from_living` on item types; `MonthStat.entertainment` → `entertainment_sum`; `MonthStatPatch` drops `entertainment`
- [x] 5.2 `MonthStatView.vue`: item editor gains the 娛樂 category and a 不計入生活支出 checkbox (entertainment only); month editor's 娛樂支出 field becomes a read-only sum; table column reads `entertainment_sum`; verify `pnpm exec vue-tsc --noEmit` and a manual pass (add 500/4700-flagged/75 → 娛樂支出 5275, 生活支出 down 4700)

## 6. Docs and verification

- [x] 6.1 Update `docs/DATA_FLOW.md` and `docs/MONTH_STAT_OVERVIEW.md` for itemized 娛樂支出 and the exclusion flag
- [x] 6.2 Full verification: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, real-DB migration + re-import + `check_parity`
