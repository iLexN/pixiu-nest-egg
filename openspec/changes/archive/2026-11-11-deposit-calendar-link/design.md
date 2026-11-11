# Design

## Context

See proposal.md (Why) and the two delta specs for the exact title and details rules.

- The 定期 page renders both its 未到期定期 list and its history with the shared `DepositTable.vue` (type `Deposit`). The 家人 page renders its two tables inline in `FamilyDepositsView.vue` (type `FamilyDeposit`). The two types differ: only `FamilyDeposit` has `holder`; only `Deposit` has `rate`/`rate_derived`.
- Row menus are `RowActions.vue`, which already has opt-in items (`receive`, `hideable`, `copyable`) that emit events to the parent.
- Display formatters live in `frontend/src/format.ts`: `fmtMoney` (2 dp, en-US grouping), `fmtPercent` (3 dp), `fmtBank` (`恒生 (HS)`), backed by a private `BANK_NAMES` map.
- Every value needed (`total`, the derived `rate`) already comes from the API. The backend owns the calculations; this change only formats them.
- App settings are keys in the generic `app_meta` table, loaded by `store::settings::Settings::load`, patched by `SettingsPatch::apply`, and exposed as `AppSettings` / `AppSettingsPatch` through `GET/PATCH /api/settings` (`service::settings::{get, update}`). The 月結 page's 設定 section shows summary lines and one edit form (`startSettingsEdit` / `saveSettings` in `MonthStatView.vue`). Frontend reads use `api.getSettings()`.

## Goals / Non-Goals

**Goals:**
- One shared link builder, so both pages produce the same URL shape and line format.
- No feedback or state to manage for the action.

**Non-Goals:**
- Setting reminders or guests on the event (the link cannot set reminders; the calendar default applies).
- A configurable holder → calendar map; only the Private calendar ID is configurable.
- `.ics` downloads or support for other calendar providers.
- Any backend endpoint for building the link.

## Decisions

**Google "add event" link, not an `.ics` file or the Calendar API.**
`https://calendar.google.com/calendar/render?action=TEMPLATE&text=…&dates=…&details=…` opens the editor pre-filled. It needs no OAuth or keys and fits a loopback-only local app. Google no longer documents it, but it is widely used and was checked against the live editor in 2026. Alternatives: an `.ics` download adds a file round-trip and opens the desktop calendar rather than the web one; the Calendar API needs OAuth credentials and would write without review.

**All-day event via `dates=YYYYMMDD/YYYYMMDD` with an exclusive end.**
The end is the day after 到期日. It is computed from the `YYYY-MM-DD` string with UTC date arithmetic (`Date.UTC`, `getUTC*`), so the local time zone or DST can never shift the day. A date-only value also avoids `ctz`.

**URL building in `format.ts`, line content in each caller.**
`googleCalendarUrl({ title, date, lines })` takes the title, the `YYYY-MM-DD` date, and `[label, value][]` pairs. It drops pairs whose value is empty, renders `label: value`, joins them with `\n`, and encodes everything with `URLSearchParams`. Each caller builds its own title parts and line list, because the two record types have different fields. A shared `titleParts.filter(Boolean).join(' - ')` keeps the drop-empty rule identical. Alternative: one function taking either record type would need type narrowing on `holder`/`rate` and hide the per-page field order.

**`fmtBankName(code)` alongside `fmtBank`.**
It returns `BANK_NAMES[code] ?? code`, or `''` for null. It reuses the private map, so a bank added later shows correctly in both places.

**Opt-in `calendar` prop on `RowActions` and `DepositTable`.**
`RowActions` gains `calendar?: boolean` and a `calendar` emit; the item is placed after 複製 and before 編輯. `DepositTable` gains `calendar?: boolean` and builds the 定期 link itself, because it owns the row. Only the 未到期定期 instance in `DepositsView` passes `calendar`, so the history table stays unchanged. In `FamilyDepositsView`, only the 未到期定期 table's `RowActions` gets it.

**Pre-select the calendar with the link's `src` parameter.**
`src=<calendar ID>` pre-selects that calendar in the editor. A community reference lists it as read by Google's client code but never confirmed in the UI, so it was tested against the user's account in 2026-11 and did pre-select the Private calendar. `googleCalendarUrl` takes an optional `src`; a null or empty value omits the parameter, so the editor falls back to the default calendar.

**Private calendar ID as an `app_meta` setting, not an env var or a constant.**
The key is `calendar.private_id`, exposed as `calendar_private_id: Option<String>` on `AppSettings` and `Option<Option<String>>` on `AppSettingsPatch`, following the existing nullable-patch pattern. `service::settings::update` trims it and stores blank as cleared. No format check, because Google calendar IDs vary (an email address for a primary calendar, `…@group.calendar.google.com` for a created one). The user runs one environment and backs up `data/wealth.db`, so a database setting travels with the backup and needs no env setup. A constant would also commit a personal identifier to a GitHub-hosted repo. It needs no migration, because `app_meta` is key-value.

**Routing rule fixed in the frontend.**
Own deposits always pass the Private ID. Family deposits pass it unless `holder === 'Irene'`, an exact, case-sensitive match on the stored name, which `FamilyDepositsView` already lists among its default holders. A configurable map was rejected as UI for a rule that rarely changes. The rule is display/navigation behaviour, not a financial calculation, so it stays in the frontend.

**Pages read the setting with `api.getSettings()` in their existing `load()`.**
`DepositsView` passes the ID to `DepositTable` through a new `calendarSrc?: string | null` prop. `FamilyDepositsView` keeps it in a ref. Both fetch it alongside their current summary call, so it adds no extra round trip in sequence.

**Open with `window.open(url, '_blank', 'noopener')`.**
This runs synchronously in the click handler, so popup blockers allow it. `noopener` keeps the Google tab from reaching back into the app.

## Risks / Trade-offs

- [Google changes or removes the undocumented link] → The failure mode is an editor without pre-fill, or a Google error page. No app data is involved. The builder sits in one function, so a fix is one edit.
- [Reminders cannot be pre-set] → Accepted. The user's calendar default notification applies. Recorded as a non-goal.
- [Long details in the URL] → Eight short lines encode to well under 1 KB, far below browser and Google URL limits.
- [Holder casing] → The holder is copied as stored (`Irene`). This is accepted; no case transformation.
- [Google stops honoring `src`] → Events open on the default calendar and the user picks Private by hand. The rest of the pre-fill is unaffected.
- [Irene renamed in the data] → The rule is an exact match on `Irene`. A renamed holder routes to Private until the one-line rule is updated.
