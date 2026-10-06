# Proposal

## Why

The 已收記錄 table on the 港股 派息 page renders a Tax column that is always blank: `withholding_tax` is a US-only concept — the receive and edit forms only collect it for US dividends, and HK records always store NULL. An always-empty column is noise on the HK page.

## What Changes

- In `frontend/src/components/DividendTable.vue`, render the Tax column (`<th>Tax</th>` and the `fmtMoney(dividend.withholding_tax)` cell) in the received table only when the market is US; adjust the empty-row `colspan` (12 → 11 for HK).
- In `frontend/src/views/DividendsView.vue`, pass the view's `market` to `DividendTable`.
- The 美股 已收記錄 table keeps the Tax column unchanged; the 待收派息 table is unaffected (it never had the column).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — this is a frontend presentation change. Which columns a table renders is not spec-level behavior; the `withholding_tax` data requirements in `stock-dividends` (storage, validation, derived `gross_amount`) are unchanged and the field stays in the API response. `skip_specs: true` is set in `.openspec.yaml`, matching the precedent in the archived `collapsible-add-forms` and `row-action-menus` changes.

## Impact

- `frontend/src/components/DividendTable.vue` — new `market` prop; Tax `<th>`/`<td>` rendered only for US; colspan fix.
- `frontend/src/views/DividendsView.vue` — pass `market` to both `DividendTable` usages.
- No backend, API, database, or docs impact.
