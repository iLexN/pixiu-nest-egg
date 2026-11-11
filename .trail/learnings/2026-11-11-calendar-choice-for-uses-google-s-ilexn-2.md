---
title: "Calendar choice for 加到日曆 uses Google's src link parameter, with the Private calendar ID kept in the database"
author: ILexN
date: 2026-11-11
branch: main
id: l-6f6966fc
scope: repo
status: active
replaces: l-b105bc0c
src: agent
touches:
  - "frontend/src/format.ts#googleCalendarUrl"
  - "frontend/src/views/FamilyDepositsView.vue#openCalendar"
  - "frontend/src/components/DepositTable.vue#openCalendar"
  - backend/src/store/settings.rs
  - backend/src/service/settings.rs
  - backend/src/models/settings.rs
  - frontend/src/views/MonthStatView.vue
---

## Decided
- 加到日曆 opens Google Calendar's undocumented "add event" link (render?action=TEMPLATE). The calendar is pre-selected with the `src=<calendar ID>` parameter.
- The Private calendar ID is an app setting stored in the database, not an env var or a constant. The user runs one environment and backs up the database, so the setting travels with the backup. It also keeps a personal identifier out of the GitHub repo.
- Routing is fixed in the frontend: family deposits whose holder is exactly `Irene` go to the default calendar (no src); every other family deposit and every own 定期 go to Private.

## Tried and ruled out
- Treating `src` as unreliable: community docs mark it "from code, not verified", but it was tested on the user's account in 2026-11 and did pre-select the Private calendar.
- Env var (VITE_…) for the ID: rejected because it needs per-machine config and doesn't travel with the database backup.

## Watch out
- The link cannot set reminders; the calendar default notification applies.
- All-day events need an exclusive end date (the day after 到期日). Compute it with UTC date arithmetic so the time zone can't shift the day.

In the code: frontend/src/format.ts googleCalendarUrl; app_meta key calendar.private_id (AppSettings.calendar_private_id), edited in 月結 設定; FamilyDepositsView openCalendar holds the Irene rule.
