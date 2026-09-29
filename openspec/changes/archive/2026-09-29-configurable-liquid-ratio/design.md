# Design

## Context

The 25% threshold is a workbook formula literal, not a referenced cell — `Overview!C14` is `B14 − 0.25 × H1` and the forecast's `ref check` baseline `N7` is `H1 ÷ 4`. The app mirrors both literally: `routes/overview.rs` computes `vs_quarter_liquid = semi_total − 0.25 × liquid_assets`, and `calc::forecast_months` computes `ref_check = semi_liquid − liquid_assets / 4.0`. `check_parity` (`parity.rs`) compares the response's `vs_quarter_liquid` against the workbook's cached `C14`.

The codebase already has the exact mechanism this needs: `forecast.bill_amount` is an `app_meta` key with a code default (`DEFAULT_BILL_AMOUNT = 2158`), exposed on `GET`/`PATCH /api/months/settings` via `MonthSettings`/`MonthSettingsPatch` (`null` resets to default), validated in `update_settings`, and edited as a field in the Month Stat settings form. No migration is needed — `app_meta` is a generic key-value table.

## Goals / Non-Goals

**Goals:**

- One stored ratio drives both thresholds — `vs_quarter_liquid` (`C14`) and forecast `ref_check` (`N7`) — which are the same "quarter of 流動資產" concept the sheet writes two ways.
- Editable through the existing settings endpoint and Month Stat settings form; unset/`null` means `0.25`, so behavior is identical to today until the user changes it.
- Parity keeps verifying the workbook's own formula regardless of the configured display ratio.

**Non-Goals:**

- Splitting the two uses into independent knobs (they are one threshold; splitting can be revisited if ever wanted).
- Promoting other sheet literals (`salary × 6` buffer, `living_spend × 1.05` budget markup, the `0.0001 × 30 + 9000` budget floor) to settings.
- Any workbook or import-path change; the sheet keeps its 25% literal.

## Decisions

- **`app_meta` key `overview.semi_liquid_target`, fraction, effective default `0.25`.** The `overview.` namespace fits — the figure lives on the Overview sheet — and `semi_liquid_target` names the intent ("share of 流動資產 the 半流動資金 should cover") without colliding with the existing `liquid_ratio` field, which is the unrelated `J1 = 流動資產 ÷ (薪金 × 100)` headline. Alternatives considered: `forecast.ref_check_ratio` (too narrow — covers only one of the two uses) and `overview.liquid_ratio` (collides with the `J1` field name).
- **Settings plumbing mirrors `bill_amount`.** `MonthSettings` gains `semi_liquid_target: f64` returning the effective value; `MonthSettingsPatch` gains a nullable `semi_liquid_target` (`null` → clear → default). `update_settings` validates finite and in `[0, 1)` — same range check as `pool_rate`; a buffer target ≥ 100% of liquid assets is meaningless. The frontend edits it as a percent (×100 round-trip, like `pool_rate`).
- **`calc::ForecastInput` gains a `liquid_target`/`semi_liquid_target` field; `forecast_months` multiplies instead of dividing by 4.** `forecast.rs` reads the meta key and passes it in, keeping `calc` free of IO — the same split `bill_amount` already uses.
- **Parity recomputes `C14` with the literal `0.25`** from `response.semi_liquid.total` and `response.liquid_assets`, comparing that to the workbook's cached `C14`. Alternative considered: report `Info` when the configured ratio differs from `0.25` — rejected, because parity's job is sheet fidelity (the sheet still computes 25%), and an unconditional `Info` would also mask real regressions in the `total`/`liquid_assets` figures that feed the comparison.
- **`GET /api/overview` carries the effective ratio** (e.g. a `semi_liquid_target` field on the response or inside `semi_liquid`) so the 總覽 footer label `與…%流動相差` renders the configured percent without a second request — the same pattern as `ForecastResponse.bill_amount`.

## Risks / Trade-offs

- User sets a non-25% ratio, forgets, and sees the app disagree with the workbook → the footer label always displays the configured percent, and parity deliberately continues checking the sheet's literal so no permanent diff noise appears.
- `ref_check` and `C14` share one knob; a future want to separate them needs a second key → accepted: they are the same sheet concept, and adding a key later is cheap.
- A ratio near `1` makes `vs_quarter_liquid`/`ref_check` almost always negative → acceptable; the setting exists precisely to set a stricter target, and both figures already render negative values in red.
