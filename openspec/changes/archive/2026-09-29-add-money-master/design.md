# Design

## Context

See proposal.md — the sheet's `J29:N35` block is a bank-app savings challenge whose inputs the user copies monthly. Existing infrastructure covers everything needed: `app_meta` key-value settings (`ibkr.*`, `overview.*`, `forecast.*`), the `PATCH /api/months/settings` endpoint that already owns `overview.salary`, the `GET /api/overview` aggregation that already reads salary, the seed-once import pattern, and informational parity rows.

The key unknown resolved during planning: two of the three hand-copied figures are derivable. Confirmed against the bank app's live display on 2026-09-29:

- `month_now = full months elapsed since 2023-10-27 + 1` → **36**, matching the bank (months are 1-indexed; the challenge runs 2023-10-27 → 2026-10-26)
- `coming_save = salary + (target_amount − saved)/months_left` with `months_left = max(1, target_months − month_now + 1)` → **−41,705.06**, matching the bank exactly (negative while ahead; `can_use = salary − coming_save` then equals the over-target excess, 94,405.06)

The `coming_save` formula is confirmed by one data point only — hence the override escape hatch.

## Goals / Non-Goals

**Goals:**
- Monthly upkeep reduced to typing one number (`saved`); everything else derives or holds from settings.
- A new challenge after 2026-10-26 is pure settings reconfiguration.
- Faithful sheet semantics: figures absent while inputs are missing; `can_use` may exceed salary.

**Non-Goals:**
- Tracking challenge history (the sheet keeps none — `J29:N35` holds only the current challenge).
- Deriving `saved` from month data (it is the bank's own running total, covering months that predate tracking).
- Auto-detecting challenge completion — `month_now` keeps counting past `target_months`, `months_left` floors at 1.

## Decisions

- **Store six `app_meta` keys** — `money_master.start_date` (TEXT `YYYY-MM-DD`), `saved`, `target_months`, `target_amount`, `month_now`, `coming_save`. The last two are optional overrides, the same pattern as `month_stats.end_cash_override`: set pins the figure, `null` restores derivation. No new table — five scalar settings and two overrides do not justify one, and `app_meta` is already the home of `ibkr.*`/`overview.*` figures.
- **Derive rather than copy** for `month_now` and `coming_save`, with overrides. Alternative considered: keep all three manual like the sheet — rejected because the derivations reproduce the bank's figures exactly and cut monthly upkeep to one field. `saved` stays manual: it is the bank's own total including pre-tracking months (Oct/Nov 2023 ≈ $172k) that no app data can reconstruct.
- **Seed constants at import, not workbook cells.** The workbook's `K31`/`L31`/`K34` literals are stale (month 35 vs live 36); the user supplied current bank figures, which import writes once (`2023-10-27`, `1094405.06`, `36`, `1000000`). The `J29:N35` cells are still parsed into `OverviewCached` for traceability and informational parity.
- **Settings live in the 月結 settings card**, not an inline editor on the card. The user updates monthly at 月結 time, and `PATCH /api/months/settings` already exists — no new endpoint, so the OpenAPI pin test is untouched.
- **Parity is informational for the whole block.** The app deliberately carries newer figures than the frozen workbook; derived cells follow whatever is stored, so every comparison can legitimately differ.

## Risks / Trade-offs

- [Bank's `coming_save` formula is guessed from one data point] → the `coming_save` override pins the bank's figure if it diverges; first real check is next month's bank update.
- [Stale override silently pins a figure] → clearing is one field in the same settings form; documented in the month-stat delta's reconfiguration scenario.
- [`month_now` keeps growing past challenge end] → acceptable; a new challenge resets `start_date` and the counter restarts.
- [`months_left` floor at 1 divides the whole gap by 1 at/after the target month] → matches the bank's observed `−41,705.06`; a `0` divisor would be nonsense.
