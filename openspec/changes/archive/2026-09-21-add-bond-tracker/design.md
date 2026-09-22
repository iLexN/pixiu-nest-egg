# Design

## Context

The workbook `債券` sheet holds a registry table (one row per bond: label / 發行編號 / principal / `end` maturity; `B1 = SUM` of principal) and, below it, one coupon-schedule block per bond (a label line containing 發行編號, then 付息日 / 利息釐定日 / 年息率 / 每1萬利息 / derived-interest rows). Today it carries one bond — `silver bond` / `03GB2710R` / 50000 / matures 2027-10-23 — with six semiannual coupons, three fixed at 4% and three 待定. Ended bonds were deleted from the source, so import seeds active bonds only; the app accumulates matured history from then on. See proposal.md for motivation and specs/ for requirements.

Existing conventions to follow: `deposits`-style REST routes (`list`/`create`/`update`/`remove` + a `summary` handler), dividend-style nullable PATCH models and `received_amount`-flips-status derivation, sqlx migrations numbered sequentially (next is `0012`), `crate::routes::today()` for the current date, `REAL`/`f64` money with display-only formatting, derived fields computed on read.

## Goals / Non-Goals

**Goals:**
- Mirror the sheet faithfully enough for parity: per-10k interest stored, expected derived — same formula shape as the sheet's interest column.
- Lifecycle with zero stored status: bond maturity and coupon states are pure functions of dates and nullable fields, so nothing needs a sweep or a cron.
- Import + parity follow the established pattern so the migration stays verifiable against the workbook.

**Non-Goals:**
- AIA, 香港年金, Month Stat, Overview sheets.
- Principal repayment as a transaction: maturity only flips status (the sheet models no cash flow either).
- Currencies other than HKD (the sheet tracks none), accrued-interest pricing, bond trades/market values.
- A second nav tab or year rollups — the data is too small (one bond today); a single 總覽 page carries the matured section inline.

## Decisions

### Schema (migration `0012_bonds.sql`)

```sql
CREATE TABLE bonds (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    label         TEXT    NOT NULL,
    issue_no      TEXT,
    principal     REAL    NOT NULL,
    maturity_date TEXT    NOT NULL,
    note          TEXT,
    sort_order    INTEGER NOT NULL,
    created_at    TEXT    NOT NULL,
    updated_at    TEXT    NOT NULL
);

CREATE TABLE bond_coupons (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    bond_id         INTEGER NOT NULL REFERENCES bonds (id),
    pay_date        TEXT    NOT NULL,
    fixing_date     TEXT,
    annual_rate     REAL,          -- NULL = 待定
    per_10k         REAL,          -- NULL = 待定
    received_amount REAL,
    note            TEXT,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL
);
```

- `per_10k` (not a total amount) is stored because that is the figure HKMA announcements publish and the cell the sheet holds; `expected = per_10k × principal ÷ 10000` is derived on read. Alternative considered — store the total coupon amount: rejected, it would lose the announced figure and break the parity mapping to the sheet's per-10k column.
- `annual_rate` stored alongside `per_10k` (the sheet keeps both; rate is display context, per_10k drives the math).
- `fixing_date` is stored for reference only — no logic reads it today.

### Derived state (`calc.rs`)

- Bond `status`: `maturity_date > today` → `active`, else `matured`. Same predicate shape as `deposit_active` (`end_date > today`).
- Coupon `status`: `received_amount IS NOT NULL` → `received`; `annual_rate IS NULL OR per_10k IS NULL` → `待定` (`pending_fix`); else `pending`. This is the dividend lifecycle extended with a pre-pending state — the dividend's `estimated_amount` analogue is the derived `expected`.
- `expected`, `variance = received − expected` derived per row; `bond.next_pay_date` = earliest `pay_date` among non-received coupons.

### API

- `GET /api/bonds?status=active|matured`, `POST /api/bonds`, `PATCH /api/bonds/:id`, `DELETE /api/bonds/:id` — delete cascades coupons (no other table references a bond; unlike stocks there is nothing to orphan).
- `GET /api/bonds/summary` → `{ today, totals { active_principal }, active: [Bond + next_pay_date + coupons], matured: [Bond + coupons], upcoming_coupons: [Coupon + bond label] }`.
- `GET/POST /api/coupons` (`?bond_id=`), `PATCH/DELETE /api/coupons/:id` — flat routes with `bond_id` on the body, matching the dividend route style rather than nesting under `/api/bonds/:id/coupons`; PATCH uses the nullable-field serde pattern so `annual_rate`/`per_10k`/`received_amount` can be cleared back to 待定/unreceived.

### Frontend

New `債券` nav group (after MPF), single `總覽` tab → `BondsView.vue`: totals strip (active principal + next coupon), one card per active bond with its coupon table (status chips 待定/pending/received, inline rate entry on 待定 rows, 收訖 action on pending rows, variance on received rows), matured bonds in a collapsed section at the bottom. Market toggle hidden per the nav delta. Add/edit forms follow `DepositForm.vue`/`DividendReceiveForm.vue` patterns.

### Import (`xlsx.rs`, `import.rs`)

- Registry: locate the row whose cell reads `end` (D4 today); every subsequent contiguous row with a numeric principal and a serial date in the maturity column is a bond (label / issue / principal / serial→ISO date). Single-bond layout is the only sample, so the parser assumes contiguous rows — a deliberate documented guess, cheap to revisit when a second bond appears.
- Coupon blocks: scan for the 付息日/利息釐定日/年息率 header; the label row above it is matched to a bond by extracting `發行編號(\S+)` and comparing to `issue_no`. Rows below the header until a blank row are coupons; `待定` text cells → `NULL`.
- Receipt heuristic: pay date ≤ import day → `received_amount` = the row's computed interest (the coupon auto-credits; same past-dated ⇒ received rule dividends use). Future-dated → unreceived. A mistaken received mark can be cleared via PATCH (`received_amount: null`).
- Idempotency: bond key = `issue_no` (fallback label+principal+maturity when absent); coupon key = `bond_id + pay_date`. Second import skips both.

### Parity (`parity.rs`)

- Compare `Σ active principal` vs cached `B1` (Total); per-coupon effective amount (`received_amount` else `expected`) vs the cached interest cells, skipping 待定 rows — the cached cell can disagree with 每1萬利息 (the workbook's own `2026-04-23` row caches `1000` where `199.45 × 5` would give `997.25`), so comparing expected alone would flag the sheet's own inconsistency. Date-dependent status caveat inherited from deposits: a bond that matured since the workbook last recalculated shows as an informational diff, not a hard failure.

## Risks / Trade-offs

- **One-bond sample**: multi-bond sheet layout is inferred → parser assumes contiguous registry rows and repeating coupon blocks; if the sheet grows a second bond in a different shape, the parser gets adjusted then. Low cost — one user, one file.
- **Receipt heuristic**: a past-dated coupon is assumed received; if the payout was delayed the record overstates → user clears `received_amount` via PATCH; parity does not compare receipt status at all, only expected amounts.
- **Overview staleness**: `Overview`/`Month Stat` keep reading the frozen workbook 債券 cells until those sheets migrate → documented in DATA_FLOW.md; parity only compares 債券 internals.
- **HKD-only**: no currency column anywhere in the sheet → totals are a bare sum, consistent with deposits; a future USD bond would add a `currency` column then.

## Migration Plan

1. Migration `0012_bonds.sql` creates the two tables.
2. `import_xlsx` parses + inserts bonds and coupons; `check_parity` reports the bonds section — both verified against the real workbook before UI work is trusted.
3. API + `BondsView.vue` land together behind the new nav group.
4. Rollback: drop the new tables; the workbook is untouched throughout.
