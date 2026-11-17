---
title: "Settings-domain CRUD gets a service module; merge helpers live in crate::patch"
author: ILexN
date: 2026-11-17
branch: main
id: l-3334734b
scope: repo
status: active
src: agent
touches:
  - backend/src/lib.rs
  - backend/src/models/mod.rs
  - backend/src/models/months.rs
  - backend/src/routes/aia.rs
  - backend/src/routes/bonds.rs
  - backend/src/routes/deposits.rs
  - backend/src/routes/dividends.rs
  - backend/src/routes/family.rs
  - backend/src/routes/mod.rs
  - backend/src/routes/months.rs
---

## Decided
- `service::manual_assets` exists even though manual assets are single-domain CRUD: create = validate → sort_order → insert → reload, update = load → merge → validate → update → reload — mirroring `service::cash_accounts`. User chose this over route→store. Not a violation of the two-tier write seam: settings-like domains where create/update carry domain validation + ordering + merge belong in service.
- PATCH-merge helpers (`merge`/`merge_opt`/`merge_num`/`merge_text`) live in `crate::patch` (`backend/src/patch.rs`), not `routes/patch.rs` — services can't import routes. The 2026-11-05 learning's "lift on third callsite" prediction came true with `service::manual_assets::update`.

## Watch out
- No new private merge_text/merge_num copies in service — import `crate::patch`.
- `store::manual_assets::{sum,load_all,load_one,next_sort_order,insert,update,delete}` — the `manual_asset_*`/`load_assets` names are gone.
- Settings endpoint tests moved to `routes/settings.rs` tests; asset workflow tests are at the service seam, `unknown_liquidity` stays route-side.

In the code: `backend/src/patch.rs`, `backend/src/service/manual_assets.rs`, `backend/src/store/manual_assets.rs`, `backend/src/routes/manual_assets.rs`; openspec/changes/archive/2026-11-17-manual-assets-module.
