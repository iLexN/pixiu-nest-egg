# Design

## Context

The 資產 block reads `hk_years` (the HK yearly table) for its stock rows; `service::yearly::build(pool, Market::Us)` already produces the same `YearRow` shape (cost/dividends/market value/sold_pl) in USD. The blended 回報率 are HKD figures mixing 債券 + 港股 + 定期. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**

- Surface the US yearly figures on the 年結 page without changing what the HK rows mean.
- Keep the blended rates a whole-portfolio measure rather than HK-only.

**Non-Goals:**

- No `美股P/L` editing on the 年結 page — it stays on 美股 → 總覽 → 每年總覽 (the per-market `year_snapshots` PATCH already exists); `投資純利` stays interest + HK `sold_pl`.
- No per-year FX rates — the single stored `aia.usd_hkd_rate` converts every year's US figures inside the blends.

## Decisions

- **USD display, HKD in blends.** The 美股 rows show raw USD (the numbers the US summary and yearly table already report); only the blends convert. Alternative rejected: converting the display too would hide the USD figures the owner actually sees in IBKR.
- **A blend is absent when the year has US figures but no rate is set.** A blend computed HK-only under a missing rate would look plausible and be wrong — the same rule Overview uses for unset FX on `short_term`/`long_term`.
- **Additive response fields only** (`us_stock_*`, `us_sold_pl`): no API version bump, no endpoint changes, no schema migration — `year_review` stores no per-market data, the US numbers derive live like the HK ones.

## Risks / Trade-offs

- [Single FX rate applied to all years] → noted on the page footnote; acceptable since the sheet had no per-year rate and the rate is a manually maintained approximation.
- [Past-year 美股 市值 needs a US snapshot] → same freeze rule as 港股; the year-end guide already instructs freezing once per market.
