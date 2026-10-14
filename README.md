# Wealth report tracker

A local, single-user web app that replaces a personal wealth-tracking workbook (`財富分析報告.xlsx`) with a Rust + SQLite backend and a Vue frontend. The workbook has been fully migrated and retired; the app keeps the same calculation rules the spreadsheet used.

Everything runs on your machine. The backend binds to loopback only, accepts cross-origin requests only from the Vite dev server, and makes no outbound network calls (the optional `/scalar` API playground loads its UI from a CDN, version-pinned and integrity-checked).

## What it covers

| Area | Features |
|---|---|
| 股票 | Stock registry (港股 / 美股), trade history, 現價 entry (single or bulk JSON), 持倉總覽 with holdings / cost / market value / P&L per stock and per market, 派息 lifecycle, yearly summary with freeze |
| 定期 | Time deposits, 收訖 with optional bank-in, rollups; 家人 record-only deposits |
| 投資 | MPF accounts and history, 債券 with coupon schedule, AIA policies and events |
| 總覽 | Asset table, 半流動資金, 投資目標 cards, IBKR block, monthly 月結 with itemised suggestions, 年結 year-in-review |
| 工具 | Bulk 現價 import (`import_prices`), OpenAPI docs with an interactive playground |

## Stack

- **Backend** — Rust 2024, [axum](https://github.com/tokio-rs/axum), [sqlx](https://github.com/launchbadge/sqlx) with SQLite, [utoipa](https://github.com/juhaku/utoipa) for OpenAPI.
- **Frontend** — Vue 3, TypeScript, Vite, pnpm.
- **Database** — a single SQLite file, `data/wealth.db`, with numbered migrations in `backend/migrations/` applied automatically at startup.

## Getting started

Prerequisites: a Rust toolchain, Node.js, and pnpm.

```sh
# 1. Build the frontend
cd frontend && pnpm install && pnpm build && cd ..

# 2. Run the server (migrations create an empty data/wealth.db on first start)
cargo run -p wealth-backend --bin wealth-backend
```

Open `http://127.0.0.1:8787`. The API playground is at `http://127.0.0.1:8787/scalar`.

For frontend development with hot reload, run the backend as above and, in a second terminal:

```sh
cd frontend && pnpm dev     # http://127.0.0.1:5173, /api proxied to :8787
```

### Configuration

| Variable | Default | Purpose |
|---|---|---|
| `WEALTH_DB` | `data/wealth.db` | SQLite database path |
| `WEALTH_ADDR` | `127.0.0.1:8787` | Bind address (keep it loopback) |
| `WEALTH_FRONTEND_DIST` | `frontend/dist` | Static frontend directory served by the backend |

## Command-line tools

```sh
# Bulk-update 現價 from {"stocks": [{"symbol": "0005.HK", "price": 60.5}, ...]}
cargo run -p wealth-backend --bin import_prices -- current-price.json
```

`import_prices` honors `WEALTH_DB`.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cd frontend && pnpm build && pnpm exec vue-tsc --noEmit
```

Use a scratch database for experiments so the real data is never touched:

```sh
WEALTH_DB=/tmp/wealth-test.db WEALTH_ADDR=127.0.0.1:8790 cargo run -p wealth-backend --bin wealth-backend
```

### Design principles

- The backend owns all validation and financial calculations; the frontend only collects input and formats output.
- Money is stored as `REAL`/`f64`, unrounded, matching the spreadsheet. Rounding happens at display time only.
- Where the spreadsheet's rules are quirky (e.g. 加權平均買入單價 divides by shares *bought*, not held), the app reproduces them deliberately — the workbook's formulas were the specification.
- Behavior changes go through [OpenSpec](https://github.com/Fission-AI/OpenSpec): a proposal and delta spec in `openspec/changes/`, implemented, then archived into `openspec/specs/`.

## Documentation

- `openspec/specs/` — normative specification of every migrated capability.
- `docs/DATA_FLOW.md` — how the pieces fit: UI action → API → table → calculation, with per-topic guides for stocks, deposits, investments, overview, and the database schema.
- `openspec/changes/archive/` — design history for each migration step.
- `AGENTS.md` — working rules for AI coding assistants.

## Privacy

`data/` and `portfolio.csv` are gitignored because they contain personal financial data. Do not commit them, and do not point `WEALTH_DB` at a path inside the repository other than `data/`. Back up `data/wealth.db` — it is the only copy of the data.
