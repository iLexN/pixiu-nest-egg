# Proposal

## Why

月結 and 年結 each sit as a top-level group with a single sub-tab, even though both are cross-asset roll-ups of the same kind as 總覽 (now / per month / per year) rather than asset classes like 股票, 定期, MPF, 債券, or AIA. Folding them into the 總覽 group puts the three time-horizon views side by side, shrinks the top row from nine buttons to seven, and stops the nav from presenting roll-ups as peers of the things they roll up.

Along the way, the `app-navigation` spec still says the app opens on 股票 → 總覽 while `App.vue` has opened on the 總覽 group for some time; this change reconciles the spec with the actual (and desired) landing page.

## What Changes

- Remove the 月結 and 年結 top-level groups.
- The 總覽 group gains two sub-tabs so it reads `總覽 | 月結 | 年結`, in that order. `月結` opens the existing Month Stat page; `年結` opens the existing YearInReview page (its sub-tab label changes from 回顧 to 年結 because the group label no longer carries the word).
- The top-level order becomes 總覽, 股票, 定期, MPF, 債券, AIA, 家人.
- The default page requirement is corrected: the app opens on the 總覽 group's 總覽 page (the headline totals), matching the current code.
- MPF, 債券, AIA, and 家人 remain single-tab groups — they are distinct asset classes / owners, not roll-ups, so this change deliberately does not collapse them.
- No page content, API, or database behavior changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-navigation`: the group list and sub-tab contents (總覽 gains 月結/年結; the 月結 and 年結 groups are removed), the group-switch scenarios that reference them, the market-toggle "hidden for" list, and the default-page requirement (股票 → 總覽 becomes 總覽 → 總覽).
- `year-in-review`: the requirement that names the view "年結 → 回顧" is reworded to "總覽 → 年結"; behavior of the view itself is unchanged.

## Impact

- `frontend/src/App.vue`: `NAV` array and `Group` union only. `Tab` ids (`months`, `yearReview`) and the view components are untouched.
- `openspec/specs/app-navigation/spec.md` and `openspec/specs/year-in-review/spec.md` via the delta specs above.
- Prose references to "月結 → 總覽" / "年結 → 回顧" in `docs/DATA_FLOW.md` and `AGENTS.md` are updated to the new paths.
- No backend, API, or schema changes; no data migration.
