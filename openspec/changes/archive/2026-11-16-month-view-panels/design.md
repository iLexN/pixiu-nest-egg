# Design

## Boundary

The component owns the whole collapsible card — everything behind the
`設定 ▸` toggle, since the three sub-sections render only when it is
open and share no state with the month tables:

- `settingsOpen` toggle itself.
- 活期帳戶: `editingAccount`, `accountDraft`, the role→HKD `watch`,
  `startAccountEdit`/`saveAccount`/`removeAccount`.
- 資產: `editingAsset`, `assetDraft`, `startAssetEdit`/`saveAsset`/
  `removeAsset`.
- Settings: the 16 draft refs, `editingSettings`, `startSettingsEdit`,
  `saveSettings` (including all client-side validation).

## Props / emits

- `settings: AppSettings | null` — display lines and `startSettingsEdit`
  seeding.
- `summary: MonthSummary | null` — the 開心Pool rate display
  (`pool_rate`, `pool_rate_year`).
- `accounts: CashAccount[]`, `assets: ManualAsset[]` — the two tables.
- `year: number` — the pool-rate year label and `pool_rate_year` body
  field.
- `saved: [message]` — a write succeeded; the view sets `message`, clears
  `error`, reloads (settings/accounts/assets all refetch there).
- `failed: [message]` — a write failed validation or the API; the view
  sets `error`.

The view keeps the single feedback line — the child renders none.

## Styles

`.inline-form`/`.form-actions`/`.note` are shared with the view's own
forms and items — duplicated into the child (scoped CSS can't cross);
`.settings-line` moves outright.
