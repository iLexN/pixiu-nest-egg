# Proposal

## Why

The frontend grew the same four idioms independently in every view and
form: `numOrNull` is copied verbatim into 6 files, `fmtMonth` into 2, the
`err instanceof ApiError ? err.message : String(err)` catch body into ~80
sites, the `new ApiError(0, …, [])` re-wrap into 3, and the
`error?.fieldMessage('x')` `<small>` block into ~45 template slots. Every
copy is a place the next edit can drift.

## What Changes

- New `frontend/src/parse.ts` exporting `numOrNull` (input-side parsing,
  distinct from display `format.ts`); replaces the 6 copies and the two
  inlined parses in `YearReviewView`/`OverviewView`.
- `frontend/src/api.ts` exports `errorMessage(err)` and `toApiError(err)`
  beside `ApiError`; rewrites the ~80 catch bodies and the 3 re-wrap sites.
- `frontend/src/format.ts` exports `fmtMonth` (wider
  `string | null | undefined` signature) and `shiftYearIso` for AIA's
  `defaultNextPay`.
- New `components/FieldError.vue` rendering `<small class="error">` only
  when `error?.fieldMessage(field)` resolves; replaces the repeated block
  in DepositForm, DividendForm, TradeForm, DividendReceiveForm, StockForm.

No behavior change: `skip_specs: true`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor; `skip_specs: true` is set.

## Impact

- `frontend/src/parse.ts`, `components/FieldError.vue` — new files.
- `frontend/src/api.ts`, `frontend/src/format.ts` — additive exports.
- All 15 `frontend/src/views/*.vue` plus 6 form components — mechanical
  call-site swaps.
