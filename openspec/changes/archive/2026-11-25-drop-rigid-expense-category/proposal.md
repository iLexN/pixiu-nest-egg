# Proposal

## Why

`rigid_expenses[].category` labels each entry with the plan item's kind (`tax`/`bill`/`other`), but the kinds are bookkeeping categories, not real expense semantics: an "AIA 醫療" premium or a "新年家用" transfer gets labelled `"tax"`, which is semantically wrong and confusing for the AI consumer. Dropping the field makes the structure leaner and removes a label that adds maintenance burden without carrying meaning.

## What Changes

- `rigid_expenses` entries lose `category` and become `{item, amount_hkd}`.
- What counts as a rigid expense is unchanged: stored plan items of kinds Tax/Bill/Other, plus the synthesized quarterly 差餉 when the recurring-bill default fires and no stored Bill item replaces it. The kind filter becomes an internal definition of "rigid" rather than a field on each entry.

## Capabilities

### Modified Capabilities

- `analysis-export`: the `rigid_expenses` entry shape drops `category`; the scenarios' example entries are updated.

## Impact

- Backend: `RigidExpense` loses the `category` field in `service/export.rs`; stored-Bill detection moves from scanning collected expenses to scanning the month's Bill-kind cells.
- No frontend state changes; `pnpm gen:api` regenerates the schema types.
- Docs: `docs/overview.md` and `GLOSSARY.md` wording updated.
