-- Normalize audit timestamps to the app's format. store::now_timestamp()
-- writes local RFC 3339 with a numeric offset, e.g.
-- 2026-10-20T15:10:11.946813+08:00; older migrations wrote UTC `...Z` (0016,
-- 0017, 0028, 0029) and one `datetime('now')` `YYYY-MM-DD HH:MM:SS` row (0022).
-- strftime(...,'localtime') converts the stored UTC instant to local time,
-- '000' pads the %f millisecond fraction to the app's six digits, and the
-- offset suffix is the local-UTC difference so it is correct whatever zone
-- the machine is in. The GLOB guard skips rows already carrying an offset.
-- `received_at` and the `*_date`/`recorded_on`/`month` columns hold
-- YYYY-MM-DD dates, not timestamps, and stay untouched.

UPDATE month_items SET created_at =
    strftime('%Y-%m-%dT%H:%M:%f', created_at, 'localtime')
    || '000'
    || printf('%s%02d:%02d',
        CASE WHEN julianday(created_at, 'localtime') >= julianday(created_at) THEN '+' ELSE '-' END,
        abs(CAST(round((julianday(created_at, 'localtime') - julianday(created_at)) * 1440) AS INTEGER)) / 60,
        abs(CAST(round((julianday(created_at, 'localtime') - julianday(created_at)) * 1440) AS INTEGER)) % 60)
WHERE created_at NOT GLOB '*[+-][0-9][0-9]:[0-9][0-9]';

UPDATE fx_rates SET updated_at =
    strftime('%Y-%m-%dT%H:%M:%f', updated_at, 'localtime')
    || '000'
    || printf('%s%02d:%02d',
        CASE WHEN julianday(updated_at, 'localtime') >= julianday(updated_at) THEN '+' ELSE '-' END,
        abs(CAST(round((julianday(updated_at, 'localtime') - julianday(updated_at)) * 1440) AS INTEGER)) / 60,
        abs(CAST(round((julianday(updated_at, 'localtime') - julianday(updated_at)) * 1440) AS INTEGER)) % 60)
WHERE updated_at NOT GLOB '*[+-][0-9][0-9]:[0-9][0-9]';

UPDATE cash_accounts SET updated_at =
    strftime('%Y-%m-%dT%H:%M:%f', updated_at, 'localtime')
    || '000'
    || printf('%s%02d:%02d',
        CASE WHEN julianday(updated_at, 'localtime') >= julianday(updated_at) THEN '+' ELSE '-' END,
        abs(CAST(round((julianday(updated_at, 'localtime') - julianday(updated_at)) * 1440) AS INTEGER)) / 60,
        abs(CAST(round((julianday(updated_at, 'localtime') - julianday(updated_at)) * 1440) AS INTEGER)) % 60)
WHERE updated_at NOT GLOB '*[+-][0-9][0-9]:[0-9][0-9]';

UPDATE ibkr_transfers SET created_at =
    strftime('%Y-%m-%dT%H:%M:%f', created_at, 'localtime')
    || '000'
    || printf('%s%02d:%02d',
        CASE WHEN julianday(created_at, 'localtime') >= julianday(created_at) THEN '+' ELSE '-' END,
        abs(CAST(round((julianday(created_at, 'localtime') - julianday(created_at)) * 1440) AS INTEGER)) / 60,
        abs(CAST(round((julianday(created_at, 'localtime') - julianday(created_at)) * 1440) AS INTEGER)) % 60)
WHERE created_at NOT GLOB '*[+-][0-9][0-9]:[0-9][0-9]';
