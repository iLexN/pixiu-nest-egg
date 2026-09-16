## Context

See `proposal.md` — Why. Current state relevant to the approach:

- The app already has the stock-trade migration: axum + sqlx (runtime queries, no compile-time macros), a pure `calc.rs`, `routes/{stocks,trades,summary}.rs`, `xlsx.rs` (calamine, cached values), idempotent `import.rs`, `parity.rs`, and three Vue views with an `api.ts` client. This change follows those conventions exactly.
- `定期Info` holds `表_定期List` (F1:P24, 23 data rows): `status` (a `TODAY()`-derived formula, one row pasted literal `End`), `id` label, `input` principal, `rate`, `利息`, `total` (= input + 利息), `end date`, `note1`, `note2`, derived `month`/`year`. Three rows are interest-only (no id/principal); one row has an id with interest 0 and no principal; `input`/`rate`/`利息` cells sometimes contain arithmetic formulas.
- `定期` is the active view: a `FILTER(status="")` list, `B1` = Σ active principal (445,000), a month rollup (Σ total/利息/principal per end month), and bank rollups keyed on `SC-*`/`HS-*` label prefixes. It also carries the 定期 start/end step checklists (manual workbook chores for Month Stat, money master, 回報率, Overview 預測).
- `定期Info` columns A–D hold two year tables (`表_2027定期`, `表_2028定期`): per month, `SUMIFS` over ALL rows (any status) by end month+year → Σ 利息 and Σ total. Plus an A1:B5 scratch rate calc that is not migrated.
- Owner confirmed: two-section view (Upcoming = future end dates; History = filter by year, default current year); keep the checklists as reminder hints; keep ended deposits' history.

## Goals / Non-Goals

**Goals:**
- One `deposits` table holding exactly what the user types; every derived figure (`total`, `status`, month/year) computed on read so edits can never leave stale aggregates.
- Numeric parity with the workbook's cached deposit aggregates, demonstrable by extending `check_parity`.
- A deposits UI matching the owner's two-section request without restructuring `App.vue`.

**Non-Goals:**
- Start dates, currency, or term fields — the sheet tracks none; adding them would invent data the import cannot fill.
- Drag-reordering deposits, a renewal/rollover action, or the A1:B5 scratch calc.
- Migrating the sections the checklists point at (Month Stat, 回報率, Overview, money master).

## Decisions

### Store five inputs, derive the rest
`deposits(id, label, principal, rate, interest, end_date, note1, note2, sort_order, created_at, updated_at)`. `label`/`principal`/`rate`/`interest` are nullable because the source rows use those blanks meaningfully (interest-only rows, label-only row). `end_date` is `TEXT 'YYYY-MM-DD'` NOT NULL — every source row has one, ISO text sorts correctly, and it is what `status` derives from.

Derived on read: `total = COALESCE(principal,0) + COALESCE(interest,0)` (the sheet's `=利息+input` with blanks as 0); `status = 'End' iff end_date <= today`; `end_month`/`end_year` from the date. Alternative: store `status` and `total` — rejected, a stored `End` goes stale at midnight and a stored `total` goes stale on edit; the sheet itself derives both.

Validation (pure function in `calc.rs`): `end_date` must parse as a real date; present `principal`/`rate`/`interest` must be ≥ 0 and `rate` < 1; at least one of `label`/`principal`/`interest` must be non-empty so a row means something.

### Ordering without a reorder endpoint
`sort_order` preserves workbook row order on import; new deposits append (`MAX+1`). The Upcoming list sorts by `end_date` ASC (what matures first — also the order the sheet's table happens to be in); History sorts `end_date` DESC within the selected year. No `POST /api/deposits/order` in this change — the sheet gives no evidence the owner reorders this list, unlike the stock summary.

### Rollups reproduce the sheet, generalized minimally
- Month rollup on `定期` groups active deposits by `end month` only; the app groups by (year, month). Identical on current data — actives span Oct 2026–Jan 2027 — and not silently wrong if two active deposits ever share a month number across years. Parity compares per month number, so a future mismatch would surface rather than hide.
- Bank rollup: sheet hardcodes `SUMIF("SC-*")`/`SUMIF("HS-*")`; the app stores a dedicated `bank` column (revised from label-prefix grouping — encoding the bank inside the label was fragile and label-less rows could never join a rollup). Import derives `bank` from the label prefix (text before the first `-`), so `HS-Irene-53` lands in `HS` and any future bank works without code changes. Bank-less rows get their own bucket; parity compares only the banks the sheet lists.
- History year tables: the sheet keeps one hand-built table per year; the app generates the same shape (per month: Σ interest, Σ total) for every year present in the data, over all deposits ending that year regardless of status — that is exactly what the sheet's `SUMIFS` do.

### Same layered architecture as trades
`calc.rs` gains `validate_deposit` and the rollup functions (pure, unit-tested). `routes/deposits.rs` mirrors `trades.rs`: `GET /api/deposits?status=&year=`, `POST`, `PATCH`, `DELETE`, plus `GET /api/deposits/summary` returning `{ upcoming, active_totals, months, banks, years }` — the frontend formats, never computes.

### Frontend: one `定期` tab, two sections
`App.vue` gets a fourth tab `deposits`; the HK/US market nav is hidden while it is active (deposits have no market axis). `DepositsView.vue` renders the Upcoming section (active table + total + month + bank rollups), the History section (year `<select>` defaulting to the current year, year month-table + rows), the static checklist hints, and an add/edit `DepositForm.vue` with a live `total` preview — same pattern as `TradeForm`'s derived preview. `DepositTable.vue` mirrors `TradeTable.vue`.

### Import reads the table by headers, not by fixed columns
`xlsx.rs` gains `parse_deposits`: find the `定期Info` row whose `status`/`id`/`input`/`rate`/`利息`/`total`/`end date` headers appear (row 1, columns F–P today), then read rows until the table ends, taking cached values for formula cells. This survives column drift better than hardcoding F..P. `WorkbookData` gains `deposits` plus the cached aggregate cells needed for parity (`定期!B1`, the month table, the bank rows, both year tables). Import dedupes on `(label, end_date, principal, interest)` as f64-bit keys with the same multiplicity counting trades use — label-less rows dedupe on `(None, date, None, interest)` which is still unique in the current data.

### Parity compares aggregates, not row order
The cached `FILTER` output on `定期` is table-ordered while the app sorts by end date, so parity compares the active set's sums and per-bucket rollups rather than row-by-row order. Documented caveat (also in the spec): status is `TODAY()`-derived, so cached values reflect the last recalc — a deposit that matured since then is an expected diff, shown with both values.

## Risks / Trade-offs

- Cached sheet values are point-in-time → same mitigation as stock parity: tolerance-based comparison, diffs printed with both values, run after a fresh recalc for a clean report.
- Label-less rows weaken the dedupe key → `(label, end_date, principal, interest)` with multiplicity counting; a collision only matters if two unlabeled rows share a date and interest, in which case both are imported anyway (the multiplicity scheme preserves genuine duplicates).
- `rate` stored as a fraction vs. percent confusion → store the sheet's fraction verbatim, format as `%` only at display; validation bounds `rate < 1` catches the obvious slip of typing `3` for 3%.
- The sheet's month-only bucketing could merge cross-year actives → app groups by (year, month); parity surfaces rather than hides any divergence.
- Double entry during migration continues (deposits in app, Month Stat/Overview still by hand) → the checklist hints stay visible in the UI precisely because of this; rollback is deleting `data/wealth.db` again.

## Migration Plan

1. Apply migration `0003_deposits.sql`; build backend + frontend.
2. Run `import_xlsx`; confirm the 23 deposit rows (reported imported/skipped).
3. Run `check_parity`; the deposit section should report no diffs against a freshly saved workbook.
4. Enter new deposits in the app; keep doing the checklist's workbook steps by hand until those sections migrate.
5. Rollback: deposits live only in `data/wealth.db`; keep using the sheet and re-import later.

## Open Questions

- Whether a "roll over into a new deposit" convenience action is worth adding once real usage shows how often renewals happen — deferred; add/edit covers it today.
- Whether History should eventually group interest-only rows separately from real deposits — deferred; they currently sit in the year they were paid, which matches the sheet.
