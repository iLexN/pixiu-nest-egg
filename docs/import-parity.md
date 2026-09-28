# Import & parity — `財富分析報告.xlsx`

Part of the [data-flow guide](DATA_FLOW.md). The workbook is the read-only import source and parity baseline; neither tool modifies it.

## Import workbook data

```text
cargo run -p wealth-backend --bin import_xlsx -- "財富分析報告.xlsx"
  → xlsx.rs opens the workbook read-only
  → cached formula values are read
  → import.rs inserts/updates stocks and inserts missing trades
```

The importer reads:

- `港股Trade` — trade columns plus the J–O 派息 block
- `美股Trade` — trade columns plus the J–O 派息 block
- `港股`
- `美股` — plus the IBKR account cells `B1` (累計轉入 HKD → one `ibkr_transfers` row dated to the first US trade, seeded only while the log is empty), `B2` (IBKR App 現值), `B4`/`B5` (HKD/USD cash) → `ibkr.*` `app_meta` keys, seeded only while unset
- `定期Info` (表_定期List) and `定期` (cached aggregates for parity)
- `MPF` — the account table under the 總供款額/帳戶結存 headers; the fund-details table below it and the remark row are ignored
- `債券` — the registry table under the `end` header (label / 發行編號 / principal / maturity), then each bond's coupon block under its 發行編號 label line: 付息日 / 利息釐定日 / 年息率 / 每1萬利息 / cached 利息; `待定` cells import as NULL
- `AIA` — the D–O policy block: rows with numeric `buy usd`/`now usd` cells and a label or policy number; a blank label inherits the plan name above it, and the remark cells (L onward) join into `note`. A row resuming after a blank gap imports `in_account` false, and the row whose removal reconciles Σ premium/Σ value to the cached `buy usd`/`now usd` cells imports `excluded` — `irene 20%` and `irene 年金` respectively today; unresolvable cases flag nothing and report a warning
- `Overview` — `N3`, the cached USD→HKD rate, which seeds `aia.usd_hkd_rate` once; plus `E1` (salary → `overview.salary`), `N8` (current-year pool rate → `overview.pool_rate.<year>`), the manual cells `B7`/`B8`/`B16`/`B17` → `manual_assets` (all seed only when unset), and the 投資目標 block's `J:K` year/invested cells → `year_review.invested_adjustment` for years the YearInReview sheet has no block for (e.g. 2023's +110000), never overwriting a stored value
- `Month Stat` — monthly rows since 2023-12: F/O/P store as-is (blank → 0), B/D store their cached literal unless the cell links to `Overview!` (then NULL = live), G/I−J/L-tail materialize as `month_items` keeping the formula text in `note`, and N 利息 imports as an `interest` item holding `sheet N − all in-month auto events` (the unexplained remainder, labeled 其他利息 — skipped for blank cells and exact matches; unreceived deposits are still subtracted so their later 收訖 doesn't double-count a pre-typed cell); each row's salary is recovered from its L formula's leading literal (fallbacks: previous row's H trailing literal, latest known, `Overview!E1`); the yearly block feeds only the historical pool-rate solve — past years' `overview.pool_rate.<y>` are derived from the sheet's M/G/H/N chain (e.g. 2024 → 0.53, 2025 → 0.425); date-only rows are skipped
- The market sheets' year blocks (B year, C net invested, F 成本, H 總市值) and `YearInReview`'s 股票 rows — seeded into `year_snapshots` for years before the current one; the current year stays live

For each dividend row the importer stores J (stock), K (pay date, Excel serial dates accepted), M (派息 amount, cached value), and O (股數 snapshot). The remaining snapshots are recovered from the cached rates the same way the sheet computed them — `buy_cost = M ÷ L`, `received_price = M ÷ (N × O)` — falling back to trade-derived snapshots when a rate is absent. The M formula text, when present, is kept in `note`. Rows with an N rate, or a pay date already past, import as received; future rows without it import as pending estimates.

The import is idempotent. A second run skips trades already stored with the same stock, date, type, shares, and total, deposits already stored with the same label, end date, principal, and interest, dividends already stored with the same stock, pay date, and amount, bonds already stored with the same 發行編號 (or the same label/principal/maturity when the sheet has none), and coupons already stored with the same bond and pay date. Year snapshots merge at field level: a stored value — seeded or edited — is never overwritten, while empty fields on an existing row are filled from the workbook. MPF accounts are keyed by label: a second run skips them entirely. AIA policies are keyed by `policy_no` (falling back to label + premium + value when the sheet has none), so a second run skips them too — and `aia.usd_hkd_rate` seeds only when unset, never clobbering an edit.

A deposit whose `end_date` is already past imports as received (`received_at = end_date`); future deposits stay unreceived — the same heuristic the dividend and coupon imports use.

A bond coupon whose pay date is already past imports as received with the sheet's cached interest value as `received_amount`; future coupons stay unreceived — the same heuristic the dividend import uses. The sheet deletes matured bonds outright, so nothing is ever un-imported: history the app keeps simply stops appearing in later imports.

Each new MPF account also seeds one synthetic `mpf_history` row at last month-end. With exactly two accounts, the per-account last-month rates plus the portfolio's cached last-month rate+gain pin down the actual month-end contributions exactly, and the seeded balance is `contributions × (1 + rate)`; with any other account count the seed uses the current contributions, so the rate stays exact while the net gain approximates. The sheet's per-account max rate seeds `seed_max_rate`, and a reconstructed `rate × contributions` seeds `seed_max_gain`. The portfolio-level `max`/`last month` cells go to `app_meta` as floors, because per-account history cannot rebuild them.

The workbook is never modified.

## Run parity check

```text
cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"
  → backend recomputes stock summaries from SQLite trades
  → compares them to cached workbook summary values
  → recomputes deposit aggregates from SQLite deposits
  → compares them to 定期's cached month/bank rows, 定期!B1,
    and the 定期Info year tables
  → counts stored dividends and totals the effective amount per market,
    compared against the J–O block's row count and Σ 派息
  → recomputes MPF per-account 總供款額/帳戶結存/回報率 and portfolio
    buy/now/回報率/淨收益 strictly against the MPF sheet's cached cells,
    and the seeded last-month/max figures with a loose tolerance, since
    they reconstruct values the sheet stored only as rates
  → sums active bond principal and compares it to 債券!B1, then each
    coupon's effective amount (received else per_10k-derived expected)
    against the sheet's cached interest cell; 待定 rows carry no cached
    figure and are skipped
  → recomputes AIA per-policy premium/value/balance_pct and the totals
    (buy/now USD, overall return, display value, HKD cells via the stored
    rate) against the sheet's cached G/H/I cells and B1–B9 block; cells
    the sheet leaves blank skip quietly
  → compares Month Stat stored fields (F/O/P/B/D) and derived columns
    (H/I/J/L/C/E plus N — derived interest vs the sheet's cell)
    per month, the yearly block per year, the row-8 running
    averages, and the seeded settings/manual cells against the sheet's
    cached values; `interest_avg` averages only months carrying a value
    (non-NULL start_cash or nonzero interest), matching AVERAGE's
    non-empty-cell semantics
  → recomputes the Overview A3:C18 block (asset rows by label, Sum, the C
    shares, A13, the 半流動資金 cells, C14) and the B1/H1/J1 headline, plus
    the 美股 IBKR header cells; module-derived figures (債券/基金/MPF/已定期)
    count as real differences while cells downstream of live prices or
    user-edited manual inputs report informational. The J22:N27 投資目標
    block compares J22, the K invested cells, and completed-year N growth
    normally; the L/M cells and the current-year N report informational
    since the unified formula supersedes the sheet's per-year ones
  → rebuilds each YearInReview block and compares every cell it parses:
    ledger sums, interest, seeded manual figures (收入/invested/投資P/L),
    past-year bond/deposit overrides, and the derived blends; the
    current-year block and the sheet's own stale stock cells report
    informational
```

Month Stat parity reports informational rows rather than failures where the sheet legitimately diverges: the live `Overview!`-linked current row (stale cached B/D/C/E), any row edited after import (`updated_at > created_at`, propagated to the previous row whose derived cells depend on it), and rows whose H cell is a hand-frozen literal rather than the `=F(next) − salary` chain (2023-12, 2024-01). YearInReview parity reports informational where the sheet diverges by construction: the current year's live-moving block, its stale frozen 股票 cells (which disagree with the market sheet's own year block), its pending-inclusive 派息 total, and the current year's 債券 figure (the matured bond the registry no longer holds).

This verifies that the database reproduces the spreadsheet's trade-derived figures, deposit rollups, dividend totals, MPF figures, bond figures, and AIA figures. Deposit parity is point-in-time: the cached values reflect the workbook's last recalculation, so a deposit that matures after that point shows as a difference. MPF parity is the same: once the app is edited after import, its current figures legitimately diverge from the frozen sheet. Bond parity too: the sheet's `Total` cell only covers bonds it still lists, so a bond that matured since the workbook last recalculated — or an already-received coupon recorded with a different amount — shows as an informational difference.
