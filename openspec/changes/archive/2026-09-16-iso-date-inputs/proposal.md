## Why

Every date field in the UI is a native `<input type="date">`, which renders in the browser/OS locale — on the owner's machine that is `dd/mm/yyyy`, while everything the app stores, validates, and displays is `yyyy-mm-dd`. The mismatch makes data entry error-prone and inconsistent with the rest of the UI. Native date inputs cannot be reformatted by markup or CSS, so a small custom input is needed.

## What Changes

- Add a reusable date-entry component that presents and accepts dates in `yyyy-mm-dd` form and offers a calendar picker button.
- Use it for every date entry point: the 新增交易 form 日期 field, inline trade editing, the 新增定期 form end date field, inline deposit editing, and the trade-history 由/至 date-range filters.
- Date entry fields reject or flag input that is not `yyyy-mm-dd` before submission; server-side `YYYY-MM-DD` validation is unchanged.
- Date displays are unchanged — they already render `yyyy-mm-dd`.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `stock-trades`: add a requirement that 日期 entry (form, inline edit, and range filters) presents and accepts `YYYY-MM-DD` regardless of OS locale, with a calendar picker.
- `time-deposits`: add the same requirement for `end_date` entry (form and inline edit).

## Impact

- New `frontend/src/components/DateInput.vue`; edits in `TradeForm.vue`, `TradeTable.vue`, `DepositForm.vue`, `DepositTable.vue`, `TradesView.vue` (frontend only).
- No API, database, backend, or workbook changes — the backend already parses `%Y-%m-%d`.
- The picker button relies on `HTMLInputElement.showPicker()` (Chrome/Edge/Safari 16+); text entry works everywhere.
