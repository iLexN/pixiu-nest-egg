# Tasks

## 1. Backend: manual asset liquidity

- [x] 1.1 Add `backend/migrations/0025_manual_assets_liquidity.sql` (`ALTER TABLE manual_assets ADD COLUMN liquidity TEXT NOT NULL DEFAULT 'long' CHECK (liquidity IN ('short','long'))`) and document the column in `docs/database.md` under `manual_assets` (default `long`; ignored for `cash` rows); verify `cargo test` passes with the migration applied and a pre-existing row reads `long`
- [x] 1.2 In `models.rs`, add `ManualAssetLiquidity { Short, Long }` (snake_case serde, `as_str`/`parse`, `Default = Long`, mirroring `ManualAssetKind`); add `liquidity` to `ManualAsset`, `#[serde(default)] Option<ManualAssetLiquidity>` on `NewManualAsset`, and `Option<ManualAssetLiquidity>` on `ManualAssetPatch`
- [x] 1.3 In `routes/months.rs`, read the column in `row_to_asset`/`load_assets`, write it in `create_asset` (absent → `long`) and `patch_asset`; add tests covering create-without-liquidity → `long`, patch to `short` round-trips, and `"liquidity":"medium"` is rejected with a 4xx; verify with `cargo test months`

## 2. Backend: 策略 block

- [x] 2.1 In `calc.rs`, add `liquidity_tiers(...)` returning `LiquidityTiers` per design D3 (salary × 6; `can_use = semi_total − cannot_use`; `short_term` hinges on IBKR, `long_term` on 基金); unit tests cover the spec numbers (`316200` / `≈160823.67` / `≈1504503.87` / `≈1605802.91`), the no-salary and no-rate absences, negative `can_use` (`-16200`), and the invariant `Σ tiers == total_assets` when all present
- [x] 2.2 In `models.rs`, add `LiquidityTiers { can_use, cannot_use, short_term, long_term }` with cell-naming doc comments and `pub liquidity_tiers` on `OverviewResponse`; in `routes/overview.rs::overview`, split manual `asset` rows by `liquidity` into the two sums and call the helper; verify `GET /api/overview` against a scratch DB (`WEALTH_DB=/tmp/...`) returns the block and that `openapi_documents_every_api_operation` still passes
- [x] 2.3 Update `docs/overview.md` ("Load the 總覽 view") with the 策略 block formulas and absence rules, and add an "Implemented in app (add-overview-liquidity-tiers)" note to the `docs/MONTH_STAT_OVERVIEW.md` header; verify the documented formulas match `calc::liquidity_tiers`

## 3. Import and parity

- [x] 3.1 In `xlsx.rs`, add `liquidity` to `SheetManualAsset`, seed Irene → `short` and HS人壽/cash rows → `long` in `parse_overview`, and add `tier_can_use`/`tier_cannot_use`/`tier_short_term`/`tier_long_term` to `OverviewCached` from `K4:K7` (`cell_num(3..=6, 10)`); extend the workbook parsing test to assert the four cached cells and the Irene/HS人壽 liquidity
- [x] 3.2 In `import.rs`, bind `liquidity` in the `manual_assets` seed `INSERT` (still guarded by the empty-table check); add a test that a fresh seed stores Irene `short` / HS人壽 `long` and that a re-import leaves a reclassified row untouched
- [x] 3.3 In `parity.rs::check_overview`, compare `K4:K7` against `liquidity_tiers` using the informational class of `B14`/`H1`/`B1`; verify `cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"` (against a scratch DB freshly imported from the workbook) lists the four cells with `K5` matching `316200`

## 4. Frontend

- [x] 4.1 In `api.ts`, add `ManualAssetLiquidity`, `liquidity` on `ManualAsset` and on the create/patch payloads, and `LiquidityTiers` + `liquidity_tiers` on `OverviewResponse`; verify `pnpm exec vue-tsc --noEmit`
- [x] 4.2 In `OverviewView.vue`, add the 策略 card (可動用 / 不可動用（6個月薪金） / 短期可取回 / 長期可取回, money format, `—` for null, negative style on 可動用 below zero) beside the 半流動資金 card; verify against the dev server that the four figures render and 可動用 turns red when salary × 6 exceeds 半流動資金
- [x] 4.3 In `MonthStatView.vue`, add `liquidity` to `assetDraft` (default `long`), a 流動性 select (短期 / 長期) shown only for `kind === 'asset'`, send it on create/patch, and show 短期/長期 in the asset table; verify creating an `asset` row as 短期 moves its amount from 長期可取回 to 短期可取回 on 總覽

## 5. Integration

- [x] 5.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; then `import_xlsx` + `check_parity` against a scratch DB — the 策略 cells appear in the Overview section with `K5` matching and `K4`/`K6`/`K7` at most informational
