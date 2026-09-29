# Wealth tracker data-flow guide

This guide explains where each action saves data, which database table changes, where calculations happen, and what the frontend reloads afterward.

For the normative statement of what the app must do, see `openspec/specs/` — this guide describes how it does it today.

## Mental model

```text
Vue UI
  ↓ user action / form submission
Rust HTTP API
  ↓ validation + derivation
SQLite database (`data/wealth.db`)
  ↓ read back rows
Rust calculation layer
  ↓ API response
Vue UI displays formatted values
```

The **backend is authoritative**. The frontend may show a temporary preview while typing, but the backend validates and recalculates every value before saving or returning summary figures.

There is **no summary table** in SQLite. Summary figures are recomputed from `stocks` and `trades` every time the summary API is called, and deposit rollups are recomputed from `deposits`.

## API reference and playground

The backend serves an interactive API playground at `http://127.0.0.1:8787/scalar` (Scalar) backed by the generated OpenAPI document at `GET /api-docs/openapi.json`. Every `/api` endpoint is listed there with its parameters, request body, and response schema, and each operation can be executed against the running backend from the browser. The document is generated from the `#[utoipa::path]` annotations on the route handlers at build time, so it cannot drift from the compiled code. The playground page loads its UI script from a CDN, pinned to a specific version with a Subresource Integrity hash (see `SCALAR_HTML` in `backend/src/main.rs` for the upgrade procedure); the backend itself makes no outbound calls.

The API accepts cross-origin browser requests only from the Vite dev server origins (`http://127.0.0.1:5173`, `http://localhost:5173`). The built frontend is served same-origin and needs no CORS. This matters because loopback binding alone does not stop a web page open in the same browser from calling `127.0.0.1:8787`.

When adding an endpoint, annotate the handler with `#[utoipa::path]` and register it in `api_router` via `utoipa_axum::routes!` — a plain `.route()` call compiles but leaves the endpoint undocumented. `routes::tests::openapi_documents_every_api_operation` pins the full path+method list and fails when the surface changes.

## Topic guides

| File | Covers |
|---|---|
| [database.md](database.md) | Every SQLite table's columns and meaning, plus the list of figures derived on read and never stored |
| [stocks.md](stocks.md) | 股票: stock registry, 現價 (single + bulk JSON), trade CRUD, 持倉總覽 load and totals math, reorder/hide/filter, 派息 lifecycle, yearly table + freeze |
| [deposits.md](deposits.md) | 定期: deposit CRUD, 收訖/取消收訖 with optional bank-in, summary/rollups; 家人 → 定期 (isolated record-only deposits + holder notes) |
| [investments.md](investments.md) | MPF accounts + history + last-month/max derivation, 債券 + coupons (待定 → pending → received, principal 收訖), AIA policies + events + USD rate |
| [overview.md](overview.md) | 總覽: asset table, 半流動資金, B1/H1/J1, averages + 投資目標 cards, 預測 forecast grid (items, convert to deposit, bill setting), IBKR block, Month Stat 月結 (items/suggestions/recapture), 年結 + year-end actions |
| [import-parity.md](import-parity.md) | Workbook import (per-sheet detail, idempotency, seeding heuristics) and the parity check |

Related: [MONTH_STAT_OVERVIEW.md](MONTH_STAT_OVERVIEW.md) — the workbook analysis and decisions behind the Month Stat / Overview migration.

## Frontend vs backend responsibilities

| Responsibility | Frontend | Backend |
|---|---:|---:|
| Collect form input | Yes | — |
| Temporary fee/total preview | Yes | — |
| Validate input before saving | Basic required fields | Authoritative validation |
| Derive HK fee | Preview only | Yes |
| Derive US total | Preview only | Yes |
| Store stocks/trades | — | Yes |
| Per-trade 平均單價 | Displays it | Calculates it |
| Holdings/cost/average | Displays them | Calculates them |
| Market value/unrealized | Displays them | Calculates them |
| Sector rollup/totals | Displays them | Calculates them |
| Formatting/rounding | Yes | Keeps full `f64` precision |
| Manual stock order | Drag/drop UI | Persists `sort_order` |
| Deposit CRUD | Form (add + edit) | Yes, validates |
| Deposit totals/status/rollups | Displays them | Calculates them |
| Dividend CRUD + receipt | Forms | Yes, validates and snapshots |
| Dividend rates/variance/rollups | Displays them | Calculates them |
| Yearly rollup columns | Displays them; inline edit + freeze button | Calculates them; stores snapshots |
| MPF account editing | Form | Yes, validates + records history |
| MPF last-month/max figures | Displays them | Derives them from history + seeds |
| AIA policies + events | Forms | Yes, validates; events update policies atomically |
| AIA totals/HKD figures | Displays them | Calculates them from the stored rate |
| Workbook import | — | Yes |
| Parity check | — | Yes |

## Common questions

### Is 平均單價 saved in the database?

No. It is calculated as `total ÷ 股數` when a trade is returned.

### Is the summary saved in the database?

No. Every `GET /api/summary` recalculates it from `stocks` and `trades`.

### Does updating 現價 touch trades?

No. It updates only `stocks.manual_price` and `stocks.price_updated_at`.

### Does updating 現價 fetch one row or the whole summary?

The update itself affects one stock row. Afterward, the frontend fetches the complete summary for the selected market so all totals and rollups are consistent.

### Does deleting a stock delete its trades?

No. The backend refuses to delete a stock that still has trades or dividend records.

### Does buying more shares change a recorded dividend?

No. The 股數 and 總買入成本 on a dividend row are snapshots stored when the dividend was recorded; the rate columns use those frozen values.

### Does recording the receipt price update the stock's 現價?

No. `received_price` is stored on the dividend only — leaving 當時現價 blank snapshots the stock's current 現價 onto the dividend, which is a read of `manual_price`, not a write. The 同時更新現價 checkbox issues a separate stock update, so recording a receipt on a different day than the price change is safe.

### Does reordering stocks change calculations?

No. It changes only `sort_order`.

### Does the app modify the Excel workbook?

No. Import and parity open it read-only.
