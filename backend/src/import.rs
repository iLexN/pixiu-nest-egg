//! One-off import of the spreadsheet's trade history into the database.
//!
//! Idempotent: a trade already stored with the same stock, 日期, 類別, 股數 and
//! buy total is skipped, while genuinely duplicated source rows are preserved.

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use chrono::Datelike;
use sqlx::{Row, SqlitePool};

use crate::calc::{
    validate_deposit, validate_dividend, validate_mpf_account, validate_trade, DepositInput,
    MpfAccountInput, TradeInput,
};
use crate::models::Market;
use crate::xlsx::{MarketSheets, SheetDeposit, SheetStock, SheetYearFigure, WorkbookData};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MarketReport {
    pub stocks_created: usize,
    pub stocks_updated: usize,
    pub trades_imported: usize,
    pub trades_skipped: usize,
    pub dividends_imported: usize,
    pub dividends_skipped: usize,
    /// Rows the status heuristic imported as pending, `code pay_date` each —
    /// the ones worth eyeballing after the run.
    pub dividends_pending: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DepositReport {
    pub deposits_imported: usize,
    pub deposits_skipped: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SnapshotReport {
    pub snapshots_seeded: usize,
    /// Current-year figures and rows with no figures at all skip.
    pub snapshots_skipped: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MpfReport {
    pub accounts_created: usize,
    pub accounts_skipped: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub hk: MarketReport,
    pub us: MarketReport,
    pub deposits: DepositReport,
    pub snapshots: SnapshotReport,
    pub mpf: MpfReport,
}

impl ImportReport {
    pub fn market(&self, market: Market) -> &MarketReport {
        match market {
            Market::Hk => &self.hk,
            Market::Us => &self.us,
        }
    }
}

pub async fn import(pool: &SqlitePool, data: &WorkbookData) -> anyhow::Result<ImportReport> {
    Ok(ImportReport {
        hk: import_market(pool, &data.hk).await?,
        us: import_market(pool, &data.us).await?,
        deposits: import_deposits(pool, &data.deposits).await?,
        snapshots: import_year_snapshots(pool, &data.year_figures).await?,
        mpf: import_mpf(pool, data).await?,
    })
}

/// MPF accounts are keyed by label. Each new account seeds one synthetic
/// last-month history row from the sheet's cached last-month rate. With two
/// accounts, the portfolio's cached last-month rate+gain pin down the actual
/// last-month contributions exactly (`mpf_last_month_contributions`); with
/// more or fewer, the seed falls back to the current contributions and its
/// gain only approximates. The cached max rate plus a reconstructed max gain
/// seed the `seed_max_*` floor; the portfolio-level cached maxima go to
/// `app_meta` — per-account history cannot rebuild them.
async fn import_mpf(pool: &SqlitePool, data: &WorkbookData) -> anyhow::Result<MpfReport> {
    let mut report = MpfReport::default();
    let today = crate::routes::today();
    let first_of_month = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .ok_or_else(|| anyhow!("today {today} has no first of month"))?;
    let prev_month_end = first_of_month - chrono::Days::new(1);

    let past_contributions = mpf_seed_contributions(data);

    for (index, account) in data.mpf.iter().enumerate() {
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT id FROM mpf_accounts WHERE label = ?")
                .bind(&account.label)
                .fetch_optional(pool)
                .await?;
        if existing.is_some() {
            report.accounts_skipped += 1;
            continue;
        }

        let contributions = account.contributions.unwrap_or(0.0);
        let balance = account.balance.unwrap_or(0.0);
        validate_mpf_account(MpfAccountInput {
            label: &account.label,
            contributions,
            balance,
        })
        .map_err(|errors| {
            anyhow!(
                "row {} of the {} sheet is not valid: {}",
                account.source_row,
                crate::xlsx::MPF_SHEET,
                errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

        let now = crate::routes::now_timestamp();
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO mpf_accounts (label, trustee, contributions, balance, plan_name, \
             member_no, sort_order, seed_max_rate, seed_max_gain, created_at, \
             updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&account.label)
        .bind(account.trustee.as_deref())
        .bind(contributions)
        .bind(balance)
        .bind(account.plan_name.as_deref())
        .bind(account.member_no.as_deref())
        .bind(account.sort_order)
        .bind(account.max_rate)
        .bind(account.max_rate.map(|rate| rate * contributions))
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await
        .with_context(|| format!("inserting MPF row {}", account.source_row))?;

        if let Some(rate) = account.last_month_rate {
            let past = past_contributions
                .map(|pair| if index == 0 { pair.0 } else { pair.1 })
                .unwrap_or(contributions);
            sqlx::query(
                "INSERT INTO mpf_history \
                 (account_id, recorded_on, contributions, balance, synthetic) \
                 VALUES (?, ?, ?, ?, 1)",
            )
            .bind(id)
            .bind(prev_month_end.to_string())
            .bind(past)
            .bind(past * (1.0 + rate))
            .execute(pool)
            .await?;
        }
        report.accounts_created += 1;
    }

    for (key, value) in [
        (crate::mpf::SEED_MAX_RATE_KEY, data.mpf_cached.max_rate),
        (crate::mpf::SEED_MAX_GAIN_KEY, data.mpf_cached.max_gain),
    ] {
        if let Some(value) = value {
            sqlx::query("INSERT OR IGNORE INTO app_meta (key, value) VALUES (?, ?)")
                .bind(key)
                .bind(value.to_string())
                .execute(pool)
                .await?;
        }
    }

    Ok(report)
}

/// With exactly two accounts the sheet's per-account last-month rates plus
/// the portfolio last-month rate+gain recover the actual last-month
/// contributions; any other shape or missing figure falls back to None and
/// the seed uses current values instead.
fn mpf_seed_contributions(data: &WorkbookData) -> Option<(f64, f64)> {
    let [first, second] = data.mpf.as_slice() else {
        return None;
    };
    let rates = (first.last_month_rate?, second.last_month_rate?);
    crate::calc::mpf_last_month_contributions(
        rates,
        data.mpf_cached.last_month_rate?,
        data.mpf_cached.last_month_gain?,
    )
}

/// Frozen year-end figures seed `year_snapshots`, but only for years before
/// the current one — the current year's row is meant to stay live until the
/// owner freezes it, and the sheet's live-formula cells would otherwise pin
/// a mid-year value. The upsert fills only NULL fields, so a stored value —
/// seeded earlier or edited by hand — is never overwritten by a re-import.
async fn import_year_snapshots(
    pool: &SqlitePool,
    figures: &[SheetYearFigure],
) -> anyhow::Result<SnapshotReport> {
    let current_year = crate::routes::today().year();
    let mut report = SnapshotReport::default();
    for figure in figures {
        if figure.year >= current_year
            || (figure.invested.is_none() && figure.cost.is_none() && figure.market_value.is_none())
        {
            report.snapshots_skipped += 1;
            continue;
        }
        sqlx::query(
            "INSERT INTO year_snapshots \
             (market, year, invested, cost, market_value, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT (market, year) DO UPDATE SET \
             invested = COALESCE(year_snapshots.invested, excluded.invested), \
             cost = COALESCE(year_snapshots.cost, excluded.cost), \
             market_value = COALESCE(year_snapshots.market_value, excluded.market_value)",
        )
        .bind(figure.market.as_str())
        .bind(figure.year)
        .bind(figure.invested)
        .bind(figure.cost)
        .bind(figure.market_value)
        .bind(crate::routes::now_timestamp())
        .execute(pool)
        .await?;
        report.snapshots_seeded += 1;
    }
    Ok(report)
}

async fn import_market(pool: &SqlitePool, sheets: &MarketSheets) -> anyhow::Result<MarketReport> {
    let market = sheets.market;
    let mut report = MarketReport::default();

    for stock in &sheets.stocks {
        match upsert_stock(pool, market, stock).await? {
            Upsert::Created => report.stocks_created += 1,
            Upsert::Updated => report.stocks_updated += 1,
            Upsert::Unchanged => {}
        }
    }

    // Codes that only appear in the trade sheet still need a stock row, placed
    // after the stocks listed on the summary sheet.
    let mut next_sort_order = next_stock_sort_order(pool, market).await?;
    for trade in &sheets.trades {
        if stock_id(pool, market, &trade.code).await?.is_none() {
            upsert_stock(
                pool,
                market,
                &SheetStock {
                    code: trade.code.clone(),
                    ticker: None,
                    exchange: None,
                    sector: None,
                    sort_order: next_sort_order,
                },
            )
            .await?;
            next_sort_order += 1;
            report.stocks_created += 1;
        }
    }

    // Snapshot the trades already stored, so inserts made by this run cannot
    // make a genuinely duplicated source row look like an existing one.
    let mut existing = existing_trade_keys(pool, market).await?;

    for trade in &sheets.trades {
        let stock_id = stock_id(pool, market, &trade.code)
            .await?
            .ok_or_else(|| anyhow!("stock {} was not created", trade.code))?;

        let validated = validate_trade(TradeInput {
            trade_type: &trade.trade_type,
            trade_date: &trade.trade_date,
            shares: trade.shares,
            unit_price: trade.unit_price,
            total: trade.total,
            fee: trade.fee,
            input_mode: trade.input_mode,
            note: trade.note.as_deref(),
        })
        .map_err(|errors| {
            anyhow!(
                "row {} of the {} trade sheet is not valid: {}",
                trade.source_row,
                market.as_str(),
                errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

        let key = TradeKey::new(
            stock_id,
            &validated.trade_date,
            validated.trade_type.as_str(),
            validated.shares,
            validated.total,
        );
        if let Some(remaining) = existing.get_mut(&key) {
            if *remaining > 0 {
                *remaining -= 1;
                report.trades_skipped += 1;
                continue;
            }
        }

        let now = crate::routes::now_timestamp();
        sqlx::query(
            "INSERT INTO trades (stock_id, trade_type, trade_date, shares, unit_price, fee, \
             total, input_mode, note, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(stock_id)
        .bind(validated.trade_type.as_str())
        .bind(&validated.trade_date)
        .bind(validated.shares)
        .bind(validated.unit_price)
        .bind(validated.fee)
        .bind(validated.total)
        .bind(validated.input_mode.as_str())
        .bind(trade.note.as_deref())
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .with_context(|| format!("inserting row {} of {}", trade.source_row, market.as_str()))?;
        report.trades_imported += 1;
    }

    import_dividends(pool, sheets, &mut report, next_sort_order).await?;

    Ok(report)
}

/// The J–O 派息 block: snapshots prefer the sheet's own denominators
/// (buy_cost = M ÷ L, price = M ÷ (N × O)); missing pieces fall back to
/// deriving from the imported trades as of the pay date.
async fn import_dividends(
    pool: &SqlitePool,
    sheets: &MarketSheets,
    report: &mut MarketReport,
    mut next_sort_order: i64,
) -> anyhow::Result<()> {
    let market = sheets.market;
    let today = crate::routes::today();

    // Snapshot facts are per-stock: an unfiltered market-wide sum would
    // inflate every row's 股數.
    let mut facts_by_code: HashMap<&str, Vec<(chrono::NaiveDate, crate::calc::TradeFacts)>> =
        HashMap::new();
    for trade in &sheets.trades {
        let Some(date) = chrono::NaiveDate::parse_from_str(&trade.trade_date, "%Y-%m-%d").ok()
        else {
            continue;
        };
        let Some(trade_type) = crate::models::TradeType::parse(&trade.trade_type) else {
            continue;
        };
        facts_by_code.entry(trade.code.as_str()).or_default().push((
            date,
            crate::calc::TradeFacts {
                trade_type,
                shares: trade.shares,
                total: trade.total.unwrap_or(0.0),
            },
        ));
    }

    let mut existing = existing_dividend_keys(pool, market).await?;

    for dividend in &sheets.dividends {
        // Dividend rows can name a stock that only exists in the J–O block.
        let stock_id = match stock_id(pool, market, &dividend.code).await? {
            Some(id) => id,
            None => {
                upsert_stock(
                    pool,
                    market,
                    &SheetStock {
                        code: dividend.code.clone(),
                        ticker: None,
                        exchange: None,
                        sector: None,
                        sort_order: next_sort_order,
                    },
                )
                .await?;
                report.stocks_created += 1;
                next_sort_order += 1;
                stock_id(pool, market, &dividend.code)
                    .await?
                    .ok_or_else(|| anyhow!("stock {} was not created", dividend.code))?
            }
        };

        let pay_date =
            chrono::NaiveDate::parse_from_str(&dividend.pay_date, "%Y-%m-%d").map_err(|_| {
                anyhow!(
                    "row {} of the {} trade sheet has no usable 派息 date",
                    dividend.source_row,
                    market.as_str()
                )
            })?;

        let empty: Vec<(chrono::NaiveDate, crate::calc::TradeFacts)> = Vec::new();
        let facts = facts_by_code.get(dividend.code.as_str()).unwrap_or(&empty);
        let (derived_shares, derived_cost) = crate::calc::holdings_snapshot(facts, pay_date);
        let shares_held = dividend.shares.or(Some(derived_shares));
        let buy_cost = match dividend.rate_on_cost {
            Some(rate) if rate > 0.0 => Some(dividend.amount / rate),
            _ => Some(derived_cost),
        };
        let received_price = match (dividend.rate_on_price, shares_held) {
            (Some(rate), Some(shares)) if rate > 0.0 && shares > 0.0 => {
                Some(dividend.amount / (rate * shares))
            }
            _ => None,
        };

        // The sheet has no status marker: rows with the second rate recorded —
        // or already past their pay date — were received; future ones are the
        // pending estimate.
        let received = dividend.rate_on_price.is_some() || pay_date <= today;
        let (estimated_amount, received_amount) = if received {
            (None, Some(dividend.amount))
        } else {
            (Some(dividend.amount), None)
        };

        let key = DividendKey::new(stock_id, &dividend.pay_date, dividend.amount);
        if let Some(remaining) = existing.get_mut(&key) {
            if *remaining > 0 {
                *remaining -= 1;
                report.dividends_skipped += 1;
                continue;
            }
        }

        let validated = validate_dividend(crate::calc::DividendInput {
            pay_date: &dividend.pay_date,
            per_share: None,
            shares_held,
            buy_cost,
            estimated_amount,
            received_amount,
            received_price,
        })
        .map_err(|errors| {
            anyhow!(
                "row {} of the {} trade sheet is not valid: {}",
                dividend.source_row,
                market.as_str(),
                errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

        crate::routes::dividends::insert_dividend(
            pool,
            stock_id,
            &validated,
            dividend.amount_formula.as_deref(),
        )
        .await
        .with_context(|| {
            format!(
                "inserting 派息 row {} of {}",
                dividend.source_row,
                market.as_str()
            )
        })?;
        report.dividends_imported += 1;
        if !received {
            report
                .dividends_pending
                .push(format!("{} {}", dividend.code, dividend.pay_date));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DividendKey {
    stock_id: i64,
    pay_date: String,
    amount: u64,
}

impl DividendKey {
    fn new(stock_id: i64, pay_date: &str, amount: f64) -> Self {
        Self {
            stock_id,
            pay_date: pay_date.to_string(),
            amount: amount.to_bits(),
        }
    }
}

async fn existing_dividend_keys(
    pool: &SqlitePool,
    market: Market,
) -> anyhow::Result<HashMap<DividendKey, usize>> {
    let rows = sqlx::query(
        "SELECT d.stock_id, d.pay_date, COALESCE(d.received_amount, d.estimated_amount) \
         FROM dividends d JOIN stocks s ON s.id = d.stock_id WHERE s.market = ?",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut keys: HashMap<DividendKey, usize> = HashMap::new();
    for row in &rows {
        let key = DividendKey::new(
            row.try_get("stock_id")?,
            &row.try_get::<String, _>("pay_date")?,
            row.try_get::<Option<f64>, _>(2)?.unwrap_or(0.0),
        );
        *keys.entry(key).or_insert(0) += 1;
    }
    Ok(keys)
}

enum Upsert {
    Created,
    Updated,
    Unchanged,
}

async fn upsert_stock(
    pool: &SqlitePool,
    market: Market,
    stock: &SheetStock,
) -> anyhow::Result<Upsert> {
    let existing = sqlx::query(
        "SELECT id, ticker, exchange, sector FROM stocks WHERE market = ? AND code = ?",
    )
    .bind(market.as_str())
    .bind(&stock.code)
    .fetch_optional(pool)
    .await?;

    match existing {
        None => {
            sqlx::query(
                "INSERT INTO stocks (market, code, ticker, exchange, sector, is_active, sort_order) \
                 VALUES (?, ?, ?, ?, ?, 1, ?)",
            )
            .bind(market.as_str())
            .bind(&stock.code)
            .bind(stock.ticker.as_deref())
            .bind(stock.exchange.as_deref())
            .bind(stock.sector.as_deref())
            .bind(stock.sort_order)
            .execute(pool)
            .await?;
            Ok(Upsert::Created)
        }
        Some(row) => {
            let id: i64 = row.try_get("id")?;
            let ticker: Option<String> = row.try_get("ticker")?;
            let exchange: Option<String> = row.try_get("exchange")?;
            let sector: Option<String> = row.try_get("sector")?;

            // Fill in what the sheet knows without overwriting manual edits.
            let new_ticker = ticker.clone().or_else(|| stock.ticker.clone());
            let new_exchange = exchange.clone().or_else(|| stock.exchange.clone());
            let new_sector = sector.clone().or_else(|| stock.sector.clone());
            if new_ticker == ticker && new_exchange == exchange && new_sector == sector {
                return Ok(Upsert::Unchanged);
            }

            sqlx::query("UPDATE stocks SET ticker = ?, exchange = ?, sector = ? WHERE id = ?")
                .bind(new_ticker)
                .bind(new_exchange)
                .bind(new_sector)
                .bind(id)
                .execute(pool)
                .await?;
            Ok(Upsert::Updated)
        }
    }
}

async fn next_stock_sort_order(pool: &SqlitePool, market: Market) -> anyhow::Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM stocks WHERE market = ?")
            .bind(market.as_str())
            .fetch_one(pool)
            .await?,
    )
}

async fn stock_id(pool: &SqlitePool, market: Market, code: &str) -> anyhow::Result<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT id FROM stocks WHERE market = ? AND code = ?")
            .bind(market.as_str())
            .bind(code)
            .fetch_optional(pool)
            .await?,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TradeKey {
    stock_id: i64,
    trade_date: String,
    trade_type: String,
    shares: u64,
    total: u64,
}

impl TradeKey {
    fn new(stock_id: i64, trade_date: &str, trade_type: &str, shares: f64, total: f64) -> Self {
        Self {
            stock_id,
            trade_date: trade_date.to_string(),
            trade_type: trade_type.to_string(),
            shares: shares.to_bits(),
            total: total.to_bits(),
        }
    }
}

async fn existing_trade_keys(
    pool: &SqlitePool,
    market: Market,
) -> anyhow::Result<HashMap<TradeKey, usize>> {
    let rows = sqlx::query(
        "SELECT t.stock_id, t.trade_date, t.trade_type, t.shares, t.total FROM trades t \
         JOIN stocks s ON s.id = t.stock_id WHERE s.market = ?",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut keys: HashMap<TradeKey, usize> = HashMap::new();
    for row in &rows {
        let trade_type: String = row.try_get("trade_type")?;
        let trade_date: String = row.try_get("trade_date")?;
        let key = TradeKey::new(
            row.try_get("stock_id")?,
            &trade_date,
            &trade_type,
            row.try_get("shares")?,
            row.try_get("total")?,
        );
        *keys.entry(key).or_insert(0) += 1;
    }
    Ok(keys)
}

/// 定期Info rows have no natural id (labels can be blank), so dedupe uses the
/// same natural-fields approach as trades.
async fn import_deposits(
    pool: &SqlitePool,
    deposits: &[SheetDeposit],
) -> anyhow::Result<DepositReport> {
    let mut report = DepositReport::default();
    let mut existing = existing_deposit_keys(pool).await?;

    for deposit in deposits {
        let validated = validate_deposit(DepositInput {
            label: deposit.label.as_deref(),
            // The sheet encodes the bank in the label prefix (SC-9632 → SC).
            bank: crate::calc::label_prefix(deposit.label.as_deref()).as_deref(),
            principal: deposit.principal,
            rate: deposit.rate,
            interest: deposit.interest,
            end_date: &deposit.end_date,
        })
        .map_err(|errors| {
            anyhow!(
                "row {} of the {} sheet is not valid: {}",
                deposit.source_row,
                crate::xlsx::DEPOSIT_INFO_SHEET,
                errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

        let key = DepositKey::new(
            validated.label.as_deref(),
            &validated.end_date,
            validated.principal,
            validated.interest,
        );
        if let Some(remaining) = existing.get_mut(&key) {
            if *remaining > 0 {
                *remaining -= 1;
                report.deposits_skipped += 1;
                continue;
            }
        }

        let now = crate::routes::now_timestamp();
        sqlx::query(
            "INSERT INTO deposits (label, bank, principal, rate, interest, end_date, note1, \
             note2, sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(validated.label.as_deref())
        .bind(validated.bank.as_deref())
        .bind(validated.principal)
        .bind(validated.rate)
        .bind(validated.interest)
        .bind(&validated.end_date)
        .bind(deposit.note1.as_deref())
        .bind(deposit.note2.as_deref())
        .bind(deposit.sort_order)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .with_context(|| format!("inserting row {} of 定期Info", deposit.source_row))?;
        report.deposits_imported += 1;
    }

    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DepositKey {
    label: Option<String>,
    end_date: String,
    principal: Option<u64>,
    interest: Option<u64>,
}

impl DepositKey {
    fn new(
        label: Option<&str>,
        end_date: &str,
        principal: Option<f64>,
        interest: Option<f64>,
    ) -> Self {
        Self {
            label: label.map(str::to_string),
            end_date: end_date.to_string(),
            principal: principal.map(f64::to_bits),
            interest: interest.map(f64::to_bits),
        }
    }
}

async fn existing_deposit_keys(pool: &SqlitePool) -> anyhow::Result<HashMap<DepositKey, usize>> {
    let rows = sqlx::query("SELECT label, end_date, principal, interest FROM deposits")
        .fetch_all(pool)
        .await?;

    let mut keys: HashMap<DepositKey, usize> = HashMap::new();
    for row in &rows {
        let label: Option<String> = row.try_get("label")?;
        let end_date: String = row.try_get("end_date")?;
        let key = DepositKey::new(
            label.as_deref(),
            &end_date,
            row.try_get("principal")?,
            row.try_get("interest")?,
        );
        *keys.entry(key).or_insert(0) += 1;
    }
    Ok(keys)
}
