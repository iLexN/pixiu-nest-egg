# Tasks

## 1. Schema and models

- [x] 1.1 Add `backend/migrations/0013_aia.sql` creating `aia_policies` and `aia_events` (per design.md: policy `policy_no`, `next_pay_date`, `premium_usd`, `value_usd`, `value_updated_at`, `remaining_years`, `withdrew_usd`, `note`, `link`, `excluded`, `in_account`, `sort_order`; event `policy_id` FK, `kind`, `event_date`, `amount_usd`, `note`, `prev_next_pay_date`, `prev_remaining_years`); verify `cargo test` migration tests pass and `sqlite3 data/wealth.db ".schema aia_policies"` shows the tables after running the backend once
- [x] 1.2 Add `AiaPolicy`, `NewAiaPolicy`, `AiaPolicyPatch` (nullable-field serde pattern from `DividendPatch`), `AiaEvent`, `NewAiaEvent`, `AiaEventKind` and `AiaRatePatch` to `backend/src/models.rs`; verify `cargo build` compiles

## 2. Derivation

- [x] 2.1 In `backend/src/calc.rs`, implement `validate_aia_policy`, `aia_balance_pct` (`(value + withdrew − premium) ÷ premium`, absent at zero premium), `aia_totals` (premium/value/withdrew/`balance_pct` over non-excluded rows, `display_value` over in-account rows, HKD conversions via the rate), `next_premium_due` (earliest `next_pay_date` ≥ today), and the event-apply/undo helpers (payment: `premium += amount`, `remaining_years −= 1` when set, `next_pay_date` = given or +1yr; withdrawal: `withdrew += amount`); verify unit tests cover: withdrawal-inclusive return (`(7845.67+127.86−6000)/6000 ≈ 0.3289`), zero-premium → no figure, excluded row absent from totals but in `display_value`, out-of-account row in totals but not `display_value`, next-due picks the earliest future date, payment with unset `remaining_years` leaves it unset, default next date is +1 year

## 3. API

- [x] 3.1 Add `backend/src/routes/aia.rs` with `GET /api/aia/policies` (rows + derived `balance_pct`), `POST /api/aia/policies`, `PATCH /api/aia/policies/:id` (changing `value_usd` refreshes `value_updated_at`), `DELETE /api/aia/policies/:id`, `GET /api/aia/summary` (`rate`, `next_premium_due`, `policies`, `totals` incl. HKD conversions — absent with no rate), `PATCH /api/aia/rate` reading/writing `app_meta` key `aia.usd_hkd_rate`, and `GET /api/aia/events?policy_id=` / `POST /api/aia/events` (one transaction: snapshot `prev_*`, insert event, apply premium/remaining/next-pay or withdrew update) / `DELETE /api/aia/events/:id` (reverses amount + restores `prev_*`; deleting a policy cascades its events); wire in `routes/mod.rs`; verify with curl that PATCH updates a value + timestamp, the rate PATCH round-trips, a POST payment event advances premium/remaining/next-pay in one call, deleting that event restores them, and DELETE removes a policy from the summary totals

## 4. Workbook import and parity

- [x] 4.1 Extend `backend/src/xlsx.rs` to parse the `AIA` sheet (optional, like MPF/債券): policy rows = numeric `buy usd`+`now usd` cells with a label or policy-number cell; carry labels down to continuation rows; capture cached `B1`–`B9` summary cells, per-row `G`/`H`/`I` cells, and `Overview!N3`; verify parse output against `財富分析報告.xlsx` yields eight policy rows in sheet order
- [x] 4.2 Extend `backend/src/import.rs` to insert policies (key `policy_no`, fallback label+premium+value) idempotently, flagging `in_account` false for rows resuming after a blank gap and `excluded` for the row reconciling `Σ premium`/`Σ value` to cached `B7`/`B8`; join remark text cells into `note`; seed `aia.usd_hkd_rate` via `INSERT OR IGNORE`; verify a second `import_xlsx` run reports all skipped, adds no rows, and leaves an edited rate untouched
- [x] 4.3 Extend `backend/src/parity.rs` with an AIA section comparing per-policy premium/value (and `balance_pct` where cached) against `G`/`H`/`I` cells and the derived totals against `B7`/`B8`/`B9`/`B3÷rate`/`B4` plus HKD cells `B1`/`B2`/`B3` via the seeded rate; verify `check_parity` reports matches against the workbook's cached figures

## 5. Frontend

- [x] 5.1 Add `AIA` nav group (after 債券) with a single `總覽` sub-tab in `App.vue` (market toggle hidden while active); verify nav shows 股票, 定期, MPF, 債券, AIA in order and the toggle hides on AIA
- [x] 5.2 Add `api.ts` types/endpoints and `frontend/src/views/AiaView.vue` + policy form + event form: totals strip (USD totals + display value + overall return with HKD conversions + next premium due), inline exchange-rate editor, policy table (label/policy no, next premium due, premium, value + updated-at, withdrew, remaining years, derived return, flags, note) with add/edit/delete plus 繳費/提取 row actions (event form: date, USD amount, note, editable proposed next date defaulting to +1 year for payments) and a per-policy events history with delete-to-undo; verify `pnpm exec vue-tsc --noEmit` and a manual UI pass: add a policy, update its value, edit the rate, record a payment watching premium/remaining/next-date move together, delete the event watching them restore, see totals and HKD figures update
- [x] 5.3 Show the workbook reminder hint (Overview/Month Stat unmigrated; rate is a manual copy of `Overview!N3`) on the AIA page, matching the MPF page's pattern

## 6. Docs and verification

- [x] 6.1 Update `docs/DATA_FLOW.md` with the AIA data flow (registry, value updates, flags, rate, payment/withdrawal events with undo, totals derivation, import/parity) and `AGENTS.md` roadmap to mark AIA migrated; verify docs match implemented behavior
- [x] 6.2 Run full verification: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, plus a real `import_xlsx` (twice — second run all-skipped) + `check_parity` against `財富分析報告.xlsx`
