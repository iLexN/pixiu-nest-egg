# Design

## Context

See proposal.md — Why. Relevant current state:

- `routes/overview.rs::overview` already holds every input the block needs: `input.hk_market_value`, `input.bonds_active_principal`, `aia_hkd: Option<f64>`, `input.mpf_balance`, `ibkr.computed_total_hkd: Option<f64>`, the manual `asset` rows, `semi_total`, and `salary: Option<f64>`.
- `manual_assets` has `kind ∈ {cash, asset}` and no liquidity notion. The sheet hard-wires Irene (`B7`) into `K6` and HS人壽 (`B8`) into `K7`; the live database holds only `HS 人壽`.
- Manual rows are created/edited in `MonthStatView.vue`'s 資產 editor (`assetDraft` with a `kind` select) via `routes/months.rs` (`create_asset`, `patch_asset`, `row_to_asset`); amounts also edit inline on 總覽.
- Overview parity lives in `xlsx.rs::parse_overview` → `OverviewCached` and `parity.rs::check_overview`, one pin per cached cell, with an informational class for cells that inherit manual/live drift.
- The sheet block: `K4 = B14 − K5`, `K5 = N6 = E1×3×2`, `K6 = B7+B4+B3+B9`, `K7 = B5+B6+B8`. Invariants: `K4+K5 = B14`, `K6+K7 = B10`, so `ΣK4:K7 = B1`.

## Goals / Non-Goals

**Goals:**

- Reproduce `K4:K7` exactly from already-derived figures, with the same absence rules the module uses elsewhere.
- Make the short/long placement of manual asset rows an explicit, user-editable attribute rather than a hidden label rule.
- Keep the change additive: no new endpoints, no breaking request shapes.

**Non-Goals:**

- Migrating the `M3:N8` rates block (`N3`–`N7`) as a displayed block; `N6` is folded into `cannot_use` and `N7` already exists as `semi_liquid.vs_quarter_liquid`.
- Making the 6-month multiplier configurable.
- Targets (`J10:M18`), the 預測 forecast grid, and the long-term projection — still spreadsheet-side.

## Decisions

### D1. `liquidity` as a column on `manual_assets`, not a label rule or a separate table

Options: (a) treat all manual assets as long-term, (b) a `liquidity` column, (c) classify by label. Chosen **(b)**: `liquidity TEXT NOT NULL DEFAULT 'long' CHECK (liquidity IN ('short','long'))` via a new migration `0025_manual_assets_liquidity.sql`. It is one column, the default is correct for every row in the live database, and the user can reclassify without a code change. (a) silently misplaces a future Irene row; (c) is brittle. Applied to `cash` rows too (column is NOT NULL) but ignored for them — simpler than a nullable column with a kind-dependent check.

### D2. Model as a new `ManualAssetLiquidity` enum mirroring `ManualAssetKind`

`#[serde(rename_all = "snake_case")] enum ManualAssetLiquidity { Short, Long }` with `as_str`/`parse`, following the exact `ManualAssetKind` pattern in `models.rs`. `ManualAsset` gains `liquidity: ManualAssetLiquidity`; `NewManualAsset` gains `#[serde(default)] liquidity: Option<ManualAssetLiquidity>` (absent → `Long`); `ManualAssetPatch` gains `liquidity: Option<ManualAssetLiquidity>`. Serde rejects unknown variants, which satisfies the "invalid value rejected" scenario without hand validation. `Default` for the enum is `Long`.

### D3. Block derivation lives in a pure `calc.rs` helper

`calc::liquidity_tiers(inputs) -> LiquidityTiers` taking plain `Option<f64>`/`f64` terms (salary, semi_total, hk, bonds, ibkr: Option, aia: Option, mpf, manual_short_sum, manual_long_sum). Keeps `routes/overview.rs::overview` a wiring function and makes the invariant `Σ tiers = total_assets` a cheap unit test, matching how `invest_targets` and `trailing_averages` are already structured.

Absence rules:

- `cannot_use = salary.map(|s| s * 6.0)`; `can_use = cannot_use.map(|c| semi_total - c)`.
- `short_term = ibkr.map(|i| hk + bonds + i + manual_short)`; `long_term = aia.map(|a| a + mpf + manual_long)`. Both hinge on the rate exactly like `semi_liquid.share` does, rather than skipping absent terms the way `assets_sum` does — the spec's standing rule ("figures needing the rate SHALL be absent") wins; a partial `K6` would look plausible and be wrong.

### D4. Response shape

```
"liquidity_tiers": {
  "can_use":    Option<f64>,   // K4
  "cannot_use": Option<f64>,   // K5 = salary × 6
  "short_term": Option<f64>,   // K6
  "long_term":  Option<f64>    // K7
}
```

Added as `pub liquidity_tiers: LiquidityTiers` on `OverviewResponse` with `ToSchema` doc comments naming the cells, like `TwelveMonthAverages`.

### D5. Import seeds liquidity through `SheetManualAsset`

`xlsx.rs::SheetManualAsset` gains `liquidity`; the `parse_overview` seed table becomes `(6, Asset, "Irene", Short)`, `(7, Asset, "HS人壽", Long)`, cash rows `Long`. `import.rs` binds it in the existing `INSERT`, still guarded by `COUNT(*) == 0`. No re-import ever touches an existing row — the seed-once rule is unchanged.

### D6. Parity pins `K4:K7` as informational

`OverviewCached` gains `tier_can_use`, `tier_cannot_use`, `tier_short_term`, `tier_long_term` from `cell_num(3..=6, 10)`. `check_overview` compares each to the response block using the same informational class as `B14`/`H1`/`B1` — `K4`/`K6`/`K7` inherit manual-balance and live-price drift; `K5` is deterministic (salary × 6) but is grouped with them for simplicity since salary itself is a seeded setting the user may have edited.

### D7. Frontend

- `api.ts`: `ManualAssetLiquidity = 'short' | 'long'`; `ManualAsset.liquidity`; `LiquidityTiers` on `OverviewResponse`; `createManualAsset`/`patchManualAsset` payloads accept `liquidity`.
- `OverviewView.vue`: a 策略 card beside the 半流動資金 card, four rows (可動用 / 不可動用（6個月薪金） / 短期可取回 / 長期可取回), `fmtMoney` with `—` for null, `neg` class on 可動用 below zero — same primitives as the existing cards.
- `MonthStatView.vue`: `assetDraft` gains `liquidity` (default `'long'`); a 流動性 `<select>` (短期 / 長期) rendered only when `kind === 'asset'`; the table shows 短期/長期 for asset rows. The 總覽 inline amount editor is untouched.

## Risks / Trade-offs

- [Cash rows carry a meaningless `liquidity`] → Documented in `docs/database.md`; the editor hides the control for `cash` rows. Accepted for schema simplicity.
- [`K6`/`K7` absent without a rate even though 港股/債券/MPF are known] → Deliberate (D3); consistent with the module's rule and avoids plausible-but-wrong partial tiers. The card shows `—`.
- [Salary edits after import make `K4`/`K5` differ from cached cells] → Parity reports them as informational, like every other seeded-setting-derived cell.
- [Existing tests constructing `ManualAsset`/`SheetManualAsset` literals break on the new field] → Expected compile-time fallout; fix at the call sites (`months.rs`, `xlsx.rs`, `parity.rs`, `import.rs` tests).

## Migration Plan

1. Add `0025_manual_assets_liquidity.sql`; it applies automatically on startup and defaults every existing row to `long` — correct for the live database.
2. Deploy backend + frontend together (single binary serves both). Older frontend bundles simply ignore the new response field.
3. Rollback: revert the code; the extra column is harmless to the previous binary (it never selects it by name in `INSERT` — verify `INSERT INTO manual_assets` lists columns explicitly, which it does).

## Open Questions

None.
