# Design

## Context

See proposal.md for motivation. Current state:

- `DividendTable.vue` renders the Tax column unconditionally inside its received-table block (`v-if="!props.receivable"`): header at the `<th>Tax</th>` and cell `fmtMoney(dividend.withholding_tax)`.
- `DividendsView.vue` owns `props.market` and mounts `DividendTable` twice — once with `receivable` for 待收派息, once plain for 已收記錄. It is the component's only consumer.
- `DividendReceiveForm.vue` and `DividendForm.vue` already gate the Tax input on `dividend.market === 'US'`; this change brings the table in line with the same rule.

## Goals / Non-Goals

**Goals:**

- HK 已收記錄 table has no Tax column; US keeps it.
- The empty-row `colspan` matches the rendered column count in every combination.

**Non-Goals:**

- No backend or API changes — `withholding_tax` remains in the `Dividend` response even for HK.
- No change to the pending (待收派息) table, which never had the column.
- No removal of the Tax input from the US receive/edit forms.

## Decisions

- **Pass `market` as a required prop rather than a `showTax`-style boolean.** The view already holds `props.market`, a `Market` prop is self-documenting about *why* the column appears, and it leaves room for future market-conditional columns. Both `DividendTable` usages pass it so the prop stays required and unambiguous — the pending table ignores it.
- **Prop over deriving market from the rows.** `dividends[0]?.market` is undefined for the empty table and would hide the header (and column count) based on data rather than view context.
- **Extend the existing `v-if="!props.receivable"` block** by adding `v-if="props.market === 'US'"` on the Tax `<th>`/`<td>` rather than restructuring the template. The `columns` computed becomes `receivable ? 9 : market === 'US' ? 12 : 11`.

## Risks / Trade-offs

- [A future third market silently loses the Tax column] → Acceptable: withholding tax is US-specific today; a new market would revisit the forms' `market === 'US'` gates anyway.
