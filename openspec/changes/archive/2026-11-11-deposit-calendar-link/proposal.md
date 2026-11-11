# Proposal

## Why

The user keeps every 定期 maturity, their own and their family's, as an event in Google Calendar (web). Today each event is typed by hand from the 未到期定期 table: pick the date, write a title, copy the figures. That is slow and easy to get wrong. One click per row should open the calendar editor with all of it filled in.

## What Changes

- Each row of the 未到期定期 table on the 家人 page and on the 定期 page gets a **加到日曆** action in its `⋯` menu, listed before 編輯.
- Choosing it opens Google Calendar's new-event editor in a new browser tab, pre-filled with:
  - an **all-day event on the deposit's 到期日**;
  - a title `定期: 持有人 - 編號 - 銀行` (bank name only, e.g. `恒生`), with empty parts dropped; own deposits have no 持有人, so their title is `定期: 編號 - 銀行`;
  - a details body with one `label: value` line per field (持有人 for 家人 only, 到期日, 編號, 銀行, 本金, 利率 for own deposits only, 利息, 合計, 開始日), using the app's display formatting, with lines for empty fields left out.
- The user still presses Save in Google Calendar. The app writes nothing and keeps no state for this action.
- The 定期記錄 history tables on both pages do **not** get the action.
- **Calendar routing**: the user has two Google calendars, Ilex Ng (default) and Private. Family deposits held by `Irene` go to the default calendar; every other family deposit and every own deposit pre-selects the Private calendar.
- A new app setting `calendar_private_id` holds the Private calendar's ID. It is stored in the database so it travels with the database backup, with no env configuration. It is edited in the 月結 → 設定 form. While it is empty, every event goes to the default calendar.
- No migration: the setting is a new key in the existing `app_meta` key-value table.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `family-deposits`: adds a requirement for the 加到日曆 action on 未到期定期 rows (title, date, and details for a family deposit) and the calendar routing rule (Irene → default calendar, other holders → Private).
- `time-deposits`: adds a requirement for the 加到日曆 action on the user's own 未到期定期 rows (title without 持有人, details including 利率), the Private calendar setting, and own deposits routing to the Private calendar.

## Impact

- Backend: the settings store, model, and route (`backend/src/store/settings.rs`, `backend/src/models/settings.rs`, `backend/src/service/settings.rs`) gain `calendar_private_id` (`app_meta` key `calendar.private_id`), with a round-trip test. `GET/PATCH /api/settings` expose it. No migration.
- Generated `frontend/src/api-types.ts` via `pnpm gen:api`.
- `frontend/src/views/MonthStatView.vue`: 設定 form input and summary line.
- `docs/database.md`: the new `app_meta` key.
- Frontend deposit views:
  - `frontend/src/format.ts`: a bank-name formatter and the Google Calendar link builder (with an optional calendar ID).
  - `frontend/src/components/RowActions.vue`: a new optional 加到日曆 menu item.
  - `frontend/src/components/DepositTable.vue`: opt-in calendar action, enabled only by the 定期 page's 未到期定期 table.
  - `frontend/src/views/DepositsView.vue` and `frontend/src/views/FamilyDepositsView.vue`.
- Docs: `docs/deposits.md` row-action notes.
- External dependency: Google Calendar's "add event" link (`calendar.google.com/calendar/render?action=TEMPLATE`), which Google no longer documents but still honors. No new packages.
