# Wealth report tracker

A local, single-user web app that replaces the hand-maintained sections of `財富分析報告.xlsx` — a personal wealth-tracking workbook — with a Rust + SQLite backend and a Vue frontend. It imports the existing workbook once, keeps the same calculation rules the spreadsheet used, and lets you verify the two agree.

Everything runs on your machine. The backend binds to loopback only, accepts cross-origin requests only from the Vite dev server, and makes no outbound network calls (the optional `/scalar` API playground loads its UI from a CDN, version-pinned and integrity-checked).

## What it covers

| Area | Features |
|---|---|
| 股票 | Stock registry (港股 / 美股), trade history, 現價 entry (single or bulk JSON), 持倉總覽 with holdings / cost / market value / P&L per stock and per market, 派息 lifecycle, yearly summary with freeze |
| 定期 | Time deposits, 收訖 with optional bank-in, rollups; 家人 record-only deposits |
| 投資 | MPF accounts and history, 債券 with coupon schedule, AIA policies and events |
| 總覽 | Asset table, 半流動資金, 投資目標 cards, IBKR block, monthly 月結 with itemised suggestions, 年結 year-in-review |
| 工具 | Workbook import, parity check against the workbook's cached values, OpenAPI docs with an interactive playground |

Sections of the workbook not listed here (the `YYYY回報率` projection sheets, 開心Pool ledger, 香港年金, FIRE, ref1) are not migrated; the spreadsheet stays the source of truth for those.

## Stack

- **Backend** — Rust 2024, [axum](https://github.com/tokio-rs/axum), [sqlx](https://github.com/launchbadge/sqlx) with SQLite, [calamine](https://github.com/tafia/calamine) for reading `.xlsx`, [utoipa](https://github.com/juhaku/utoipa) for OpenAPI.
- **Frontend** — Vue 3, TypeScript, Vite, pnpm.
- **Database** — a single SQLite file, `data/wealth.db`, with numbered migrations in `backend/migrations/` applied automatically at startup.

## Getting started

Prerequisites: a Rust toolchain, Node.js, and pnpm.

```sh
# 1. Build the frontend
cd frontend && pnpm install && pnpm build && cd ..

# 2. Import the workbook (creates data/wealth.db)
cargo run -p wealth-backend --bin import_xlsx -- "財富分析報告.xlsx"

# 3. Run the server
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
# Import the workbook. Idempotent: re-running skips rows that already exist.
cargo run -p wealth-backend --bin import_xlsx -- "財富分析報告.xlsx"

# Bulk-update 現價 from {"stocks": [{"symbol": "0005.HK", "price": 60.5}, ...]}
cargo run -p wealth-backend --bin import_prices -- current-price.json

# Compare the database's derived summary against the workbook's cached values
cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"
```

All three honor `WEALTH_DB` and open the workbook read-only; the app never writes to the spreadsheet.

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
- Where the spreadsheet's rules are quirky (e.g. 加權平均買入單價 divides by shares *bought*, not held), the app reproduces them deliberately so the parity check stays meaningful.
- Behavior changes go through [OpenSpec](https://github.com/Fission-AI/OpenSpec): a proposal and delta spec in `openspec/changes/`, implemented, then archived into `openspec/specs/`.

## Documentation

- `openspec/specs/` — normative specification of every migrated capability.
- `docs/DATA_FLOW.md` — how the pieces fit: UI action → API → table → calculation, with per-topic guides for stocks, deposits, investments, overview, import/parity, and the database schema.
- `openspec/changes/archive/` — design history for each migration step.
- `AGENTS.md` — working rules for AI coding assistants.

## Privacy

`data/`, `財富分析報告.xlsx`, and `portfolio.csv` are gitignored because they contain personal financial data. Do not commit them, and do not point `WEALTH_DB` at a path inside the repository other than `data/`.
