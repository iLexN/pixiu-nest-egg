# Proposal

## Why

`Month Stat` is the last data-entry surface still living in `財富分析報告.xlsx`: a monthly ledger where the user records the 活期 bank balance at payday plus non-spending bank flows, and monthly spending/saving fall out by subtraction. It is the dependency root of the remaining migration — `Overview` and `YearInReview` read its averages, and its current row reads the live portfolio totals — so it migrates before `Overview` (which becomes a derived dashboard in a later change). Migrating it also removes the sheet's two manual pain points: paste-valuing each month's totals, and opaque hand-written sum formulas for 調整.

Full workbook analysis and the confirmed workflow live in `docs/MONTH_STAT_OVERVIEW.md`.

## What Changes

- New `month_stats` table: one row per month (`YYYY-MM-01`) storing `start_cash` (月初出糧後 — the 活期 total the user types at payday), `salary` (per-row snapshot so 月尾 stays correct across salary eras), `total_assets`/`liquid_assets` (frozen values for imported history; `NULL` = derive live), `interest` (利息), `entertainment` (娛樂支出), `pool_input` (Irene + 開心 Pool), `end_cash_override` (hand-frozen 月尾 for months the chain can't reproduce; `NULL` = derive), `note`.
- New `month_items` table: labeled line items per month replacing the sheet's opaque sum formulas — `adjustment` (the G column: bank in/out that must not count as 支出), `extra_spend` (amounts subtracted to get 生活支出: TV, AIA premium, tax, doctor), and `income` (extras added to 存, e.g. yearly bonus). Items may carry an `auto_key` linking them to the app event they came from.
- Auto-suggested items: for the current and future months the API returns candidate `adjustment`/`extra_spend` items derived from dated events already in the DB — 定期 start (− principal) and end (+ principal+interest), HK BUY/SELL (−/+ total), received 派息, received 債券 coupons, AIA premium payments (extra_spend, HKD via the stored rate), and the month's pool input leaving the bank — so the user confirms instead of hand-writing sums. Accepting stores the item with its `auto_key`; dismissing records a tombstone (`month_item_dismissals`) so it stays gone.
- `deposits` gains an optional `start_date` (new column + deposit-form field, defaulting to today on create) so a 定期 start becomes a suggestable event; the import best-effort parses the first date in `note2` (the SC-marathon rate-schedule text) and leaves it empty otherwise.
- New `manual_assets` table for the Overview cells that feed the monthly totals: `cash` rows (HS, 渣打 — the 活期 behind 月初) and `asset` rows (Irene, HS人壽), so live `total_assets`/`liquid_assets` can be computed for the current month without the Overview page.
- Derived on read, never stored (repo convention): 月尾 = stored override, else next month's `start_cash` − `salary`; 月支出 = 月初 + Σ調整 − 月尾; 生活支出 = 月支出 − Σextra_spend; 存 = salary − 月支出 + Σincome; Changed columns = next month's totals − this month's; per-year aggregates (the sheet's rows 2–4) and running averages (row 8); and the 開心Pool balance chain `M(y) = M(y−1) + Σ利息(y) × rate(y) − Σ娛樂(y) + Σpool_input(y)` with a per-year rate (`app_meta` `overview.pool_rate.<year>`, falling back to the latest earlier year).
- Live totals for the current month reproduce `Overview!B1`/`H1`: 總數 = HK market value + US market value × rate + 定期 active total + 債券 active principal + AIA value × rate + MPF balance + manual assets + cash; 流動資產 = HK market value + 定期 + cash + 債券 + US value × rate − current pool balance. This aggregation is shared groundwork the later Overview change reuses.
- Month Stat API: `GET /api/months` (all rows with derived fields, year filter), `GET /api/months/:ym` (row + items + pending auto-suggestions), `PUT/PATCH /api/months/:ym` (upsert; captures live totals on create, `recapture` re-snapshots), `GET/POST /api/months/:ym/items`, `PATCH/DELETE /api/month-items/:id`, `POST /api/months/:ym/items/dismiss`, and settings endpoints for salary, manual assets and pool rate.
- New `月結` nav group (after AIA) with a single 總覽 view: yearly aggregate strip, month table with derived columns, expandable item editor showing auto-suggestions, and a settings card (cash/asset balances, salary, pool rate).
- `import_xlsx` extended to read `Month Stat` monthly rows (cached values; formula text kept as item notes) plus the Overview manual cells (B7/B8/B16/B17 → `manual_assets`, E1 → salary, N8 → current-year pool rate), idempotently.
- `check_parity` extended to compare the derived monthly figures and yearly aggregates against the sheet's cached cells.
- Not in this change: the `Overview` dashboard itself, `YearInReview`/`YYYY回報率` generation, `HappyPool`/`Mum`/`Dad`/`香港年金`/`FIRE`/`ref1` sheets, and the Overview forecast/target blocks.

## Capabilities

### New Capabilities

- `month-stat`: the monthly balance-difference ledger — month rows, labeled items, auto-suggested adjustments from app events, derived spending/saving/living figures, frozen-vs-live asset totals, manual cash/asset balances and salary, per-year 開心Pool rate and derived pool balance, yearly aggregates, and the 月結 page.

### Modified Capabilities

- `spreadsheet-trade-import`: the import command also imports the `Month Stat` monthly rows and seeds the Overview manual inputs (cash/asset balances, salary, pool rate) idempotently; the parity command also compares Month Stat derived figures and yearly aggregates against the workbook's cached cells.
- `app-navigation`: the first-level nav gains a `月結` group (after AIA) containing 總覽; the 港股/美股 market toggle stays hidden while it is active.
- `time-deposits`: deposit records gain an optional `start_date` so a 定期 start is a dated event the Month Stat suggestions can pick up; imported deposits seed it best-effort from `note2`.

## Impact

- Backend: new migration `0014_month_stat.sql`; new `routes/months.rs`; additions to `models.rs`, `calc.rs` (monthly derivations, pool chain, live portfolio totals), `xlsx.rs`, `import.rs`, `parity.rs`, `routes/mod.rs`, and the import/parity binaries' coverage.
- Frontend: new `月結` nav group and `MonthStatView.vue` (+ item editor and settings card); `api.ts` types and endpoints; `App.vue` nav wiring.
- Docs: `docs/DATA_FLOW.md` gains the Month Stat data flow; `AGENTS.md` roadmap updates.
- No new dependencies; `財富分析報告.xlsx` stays read-only and remains source of truth for `Overview` and other unmigrated sections.
- HKD-side live totals depend on the manual `aia.usd_hkd_rate`; when unset, the affected components are absent — same accepted approximation as AIA.
