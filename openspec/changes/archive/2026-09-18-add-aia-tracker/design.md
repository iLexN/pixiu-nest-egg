# Design

## Context

The workbook `AIA` sheet holds a policy table (rows 2–8 plus a detached row 12, columns D–O: label / `next pay` premium-due date / policy number / `buy usd` / `now usd` / derived `balance %%` / `remaining years` / `Withdrew` / remark hyperlinks and free-text notes) and a summary block in `A1:B9` (`buy hkd` / `now hkd` / `drew HKD` / `balance %%` / `buy usd` / `now usd` / `AIA display value`). Today it carries eight policy lines: five annuity/savings plans plus two "share" rows — `irene 20%` (the other party's share of a policy held in the user's account, subtracted out of the totals) and `irene 年金` (the user's 20% share of a policy in the other party's account, counted in the totals but outside the `AIA display value` range). `GG`/`u04`/`322 unit` at rows 17–18 are scratch. All HKD figures convert through `Overview!N3`, a GOOGLEFINANCE `USDHKD` formula cached at `7.84522932` — nothing the app can fetch. See proposal.md for motivation and specs/ for requirements.

Existing conventions to follow: `deposits`-style REST routes (`list`/`create`/`update`/`remove` + a `summary` handler), `app_meta` key-value storage (`mpf.note` precedent, including a `PATCH` route for it), `stocks.manual_price`/`price_updated_at` for a single stored current value, sqlx migrations numbered sequentially (next is `0013`), `REAL`/`f64` money with display-only formatting, derived fields computed on read.

## Goals / Non-Goals

**Goals:**
- Mirror the sheet faithfully enough for parity: store the entered figures (premium, value, withdrew), derive the percentages — same formula shape as the sheet's `balance %%` column.
- Reproduce both of the sheet's totals figures (`now usd` and `AIA display value`) via explicit per-row flags rather than hardcoded row positions, so the model stays correct when rows are added or removed.
- Import + parity follow the established pattern so the migration stays verifiable against the workbook.

**Non-Goals:**
- 香港年金, Month Stat, Overview sheets (the workbook keeps the live GOOGLEFINANCE rate).
- Value-update history: the sheet stores one current figure per policy — a stored `value_usd` + `value_updated_at` matches it exactly (same choice as `manual_price`).
- Premium-payment or withdrawal ledgers: the sheet holds single cumulative figures.
- Fetching an exchange rate automatically, HKD-denominated policy entry, hyperlink import (calamine does not read hyperlinks — `link` is manual entry), and the `GG` scratch cells.

## Decisions

### Schema (migration `0013_aia.sql`)

```sql
CREATE TABLE aia_policies (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    label            TEXT    NOT NULL,
    policy_no        TEXT,
    next_pay_date    TEXT,
    premium_usd      REAL    NOT NULL DEFAULT 0,
    value_usd        REAL    NOT NULL DEFAULT 0,
    value_updated_at TEXT,
    remaining_years  REAL,
    withdrew_usd     REAL    NOT NULL DEFAULT 0,
    note             TEXT,
    link             TEXT,
    excluded         INTEGER NOT NULL DEFAULT 0,
    in_account       INTEGER NOT NULL DEFAULT 1,
    sort_order       INTEGER NOT NULL,
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL
);

CREATE TABLE aia_events (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id             INTEGER NOT NULL REFERENCES aia_policies (id),
    kind                  TEXT    NOT NULL,      -- 'payment' | 'withdrawal'
    event_date            TEXT    NOT NULL,
    amount_usd            REAL    NOT NULL,
    note                  TEXT,
    prev_next_pay_date    TEXT,                  -- policy values before this
    prev_remaining_years  REAL,                  -- event applied, for undo
    created_at            TEXT    NOT NULL
);
```

- `premium_usd`/`value_usd`/`withdrew_usd` store the sheet's resolved cell values, not its formulas: the sheet mixes literals and shortcuts like `=8320*(5-J2)` (annual premium × years paid), and the figures are what parity checks. Alternative considered — store annual premium and payment term and derive premium paid: rejected, it over-models data the sheet itself doesn't keep consistently and would break parity on rows like `=7643*5`.
- `excluded` (not counted in `buy usd`/`now usd`/withdrew totals) and `in_account` (counted in `display_value`) are two independent flags because the sheet needs both: `irene 20%` is in the account but not the user's money (`excluded=1, in_account=1`); `irene 年金` is the user's money held elsewhere (`excluded=0, in_account=0`). Alternative — a single `include` flag: rejected, it cannot express `irene 年金`.
- `next_pay_date` is the next premium the user owes AIA — the summary reads it (`next_premium_due`) and payment events advance it. `remaining_years`, `note`, `link` are stored for reference; `remaining_years` is additionally decremented by payment events. `link` holds the remark column's hyperlink target, entered by hand.
- `aia_events` is an append-style log, not the source of truth: `premium_usd`/`withdrew_usd` stay stored cumulative figures (the sheet's cell values, imported as-is) and events drive their updates. Alternative — derive `premium = Σ payment events`: rejected, imported rows carry no events so a seed column would be needed anyway, and parity reads the stored figure directly. The `prev_*` columns snapshot the two policy fields an event touches so `DELETE` can restore them exactly.
- No status/lifecycle columns: policies have no state machine in the sheet — nothing is derived but the percentages and `next_premium_due`.
- The `irene 年金` row intentionally counts the full premium (the user pays 100% of the premiums into irene's policy) against 20% of the value (their stake) — the resulting ≈−41.6% row return is real, not an import bug. `display_value` is kept because it is the reconciliation figure against the AIA portal: matching `sum(H2:H9)` means every `now usd` is fresh; a mismatch flags a stale value to update.

### Derived state (`calc.rs`)

- Per-policy `balance_pct = (value_usd + withdrew_usd − premium_usd) ÷ premium_usd`, absent when `premium_usd == 0` — the sheet's `I` column formula verbatim.
- Totals over `excluded = 0` rows: `premium`, `value`, `withdrew`; overall `balance_pct` with the same formula on the totals (the sheet's `B4`).
- `display_value` = `Σ value_usd` over `in_account = 1` rows (the sheet's `B9 = SUM(H2:H9)`).
- `next_premium_due` = earliest `next_pay_date` on or after today across all policies — the premium the user still owes.
- HKD figures = USD total × stored rate, only while a rate exists.

### Exchange rate (`app_meta`)

- Key `aia.usd_hkd_rate`, written once by import (`INSERT OR IGNORE`, seeded from cached `Overview!N3`) and by `PATCH /api/aia/rate` (`{ rate: f64 | null }`). Chosen over a column on a settings row because `app_meta` already exists for exactly this and the MPF note route is a direct template. Rate edits never rewrite stored policies — they only rescale the derived HKD figures.

### API

- `GET /api/aia/policies` (each row + derived `balance_pct`), `POST /api/aia/policies`, `PATCH /api/aia/policies/:id` (nullable-field serde pattern from `DividendPatch`; changing `value_usd` refreshes `value_updated_at`), `DELETE /api/aia/policies/:id` — nothing references a policy, so deletion is always permitted, same as bonds.
- `GET /api/aia/summary` → `{ rate, next_premium_due, policies: [… + balance_pct], totals: { premium, value, withdrew, balance_pct, display_value, premium_hkd, value_hkd, withdrew_hkd } }` — flat route with a `summary` handler, matching deposits/bonds.
- `PATCH /api/aia/rate` — mirrors `PATCH /api/mpf/note`.
- `GET /api/aia/events?policy_id=`, `POST /api/aia/events`, `DELETE /api/aia/events/:id` — flat routes like `/api/coupons`. `POST` runs in one transaction: snapshot `prev_next_pay_date`/`prev_remaining_years` onto the event row, insert it, then apply — `payment` adds `amount_usd` to `premium_usd`, decrements `remaining_years` when set, and sets `next_pay_date` to the submitted date or `current + 1 year`; `withdrawal` adds to `withdrew_usd`. `DELETE` reverses the amount and restores the snapshotted fields — an undo for misentries, mirroring how a cleared `received_amount` unreceives a coupon.

### Frontend

New `AIA` nav group (after 債券), single `總覽` tab → `AiaView.vue`: totals strip (USD premium / value / withdrew / display value / overall return with HKD conversions underneath, colored like other summary strips, plus the next premium due date), an inline rate editor, and the policy table — one row per policy showing label + policy number, next premium due, premium, value (+ updated-at), withdrew, remaining years, derived return, flags and note. Row actions: add/edit/delete via a form following `DepositForm.vue`/`BondForm` patterns, plus 繳費 and 提取 actions opening the event form (date, USD amount, note; payment shows the proposed next date — default +1 year — editable). A per-policy events history lists recorded payments/withdrawals with delete-to-undo. Market toggle hidden per the nav delta.

### Import (`xlsx.rs`, `import.rs`)

- `AIA` read as an optional sheet like MPF/債券 (a workbook without it seeds nothing).
- Policy rows: any row where the `buy usd` (G) and `now usd` (H) cells are both numeric and the label (D) or policy-number (F) cell is non-empty. That matches rows 2–8 and 12 and excludes the summary block (its numbers sit in B) and the `GG` scratch cells (no G/H values).
- Labels carry down: a row with an empty label cell inherits the previous row's label (`B335167809` under `年金 - 2024 - 2029`; `irene 20%` under `5yr 5.5 full paid` — its `F` text is kept as `policy_no`, faithfully odd as in the sheet).
- `in_account`: the contiguous run of policy rows starting at the block's first row is in-account; a policy row resuming after a blank gap is not (`irene 年金` at row 12 vs the block at rows 2–8). Reproduces `B9`'s range without hardcoding row numbers.
- `excluded`: reconciled against the cached `buy usd`/`now usd` cells rather than guessed — if `Σ premium` over parsed rows exceeds the cached `B7`, the row whose removal makes both `Σ premium == B7` and `Σ value == B8` is marked `excluded`. Chosen over parsing `B8`'s formula text (`=SUM(H1:H8)-H7+H12`) because cached values are always present while `worksheet_formula` can fail, and over name-matching `irene` because the rule is data-driven. Fallback: no reconciling row → nothing excluded and a warning in the import report; the flag is editable anyway.
- Remark text: the D–O row's text cells (`AIA PDF Table`, `age 56 to 65 draw`, `7032 usd / year`, `hkd/month`) are joined into `note`; hyperlink targets are not readable → `link` stays `NULL`.
- Idempotency: policy key = `policy_no` (fallback label+premium+value when absent); second import skips everything. Rate seeded `INSERT OR IGNORE` — a user-edited rate is never clobbered.

### Parity (`parity.rs`)

- Per policy: stored `premium_usd`/`value_usd` vs cached `G`/`H` cells (and derived `balance_pct` vs cached `I` where present).
- Totals: derived `premium`/`value`/`withdrew`/`display_value`/`balance_pct` vs cached `B7`/`B8`/`B9`/`B3÷rate`/`B4`; HKD cells `B1`/`B2`/`B3` compared against USD totals × the same seeded rate — identical inputs, so a match is meaningful rather than tautological only for rows the sheet itself computes.
- No date-dependent caveat (unlike deposits/bonds): nothing here keys off today. The one expected-difference source is a stale cached `Overview!N3` inside the workbook itself.

## Risks / Trade-offs

- **Flag inference depends on fresh cached cells**: if the workbook's `B7`/`B8` are stale relative to the policy rows, reconciliation may mark the wrong row or none → the import reports what it decided and both flags are editable; parity exposes any remaining disagreement.
- **Manual rate drifts from GOOGLEFINANCE**: HKD figures are an approximation between edits → documented in DATA_FLOW.md; parity is unaffected (same rate both sides) and the sheet remains the live-rate source until Overview migrates.
- **Single-workbook layout**: parser assumes the D–O policy block shape; if the sheet grows a new section shape, the parser gets adjusted then — same accepted risk as the bond importer.
- **`value_updated_at` only updates through the app**: values typed into the sheet later and re-imported won't refresh the timestamp (import skips existing rows) — consistent with how every migrated section treats the sheet post-import.
- **Event delete restores snapshot values**: deleting an old event (not the latest) restores *its* recorded `next_pay_date`/`remaining_years`, which may resurrect a stale date if later events already advanced it → undo is meant for the just-entered mistake; the UI lists events newest-first so the intended target is obvious, and the field stays hand-editable for anything else.

## Migration Plan

1. Migration `0013_aia.sql` creates `aia_policies` and `aia_events`.
2. `import_xlsx` parses + inserts policies and seeds the rate; `check_parity` reports the AIA section — both verified against the real workbook before UI work is trusted.
3. API + `AiaView.vue` land together behind the new nav group.
4. Rollback: drop the new tables and delete `aia.usd_hkd_rate` from `app_meta`; the workbook is untouched throughout.
