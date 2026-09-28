//! One-off import of the spreadsheet's trade history into the database.
//!
//! Idempotent: a trade already stored with the same stock, 日期, 類別, 股數 and
//! buy total is skipped, while genuinely duplicated source rows are preserved.

use std::collections::HashMap;

use anyhow::{Context, anyhow};
use chrono::Datelike;
use sqlx::{Row, SqlitePool};

use crate::calc::{
    AiaPolicyInput, BondInput, CouponInput, DepositInput, MpfAccountInput, TradeInput, approx_eq,
    validate_aia_policy, validate_bond, validate_coupon, validate_deposit, validate_dividend,
    validate_mpf_account, validate_trade,
};
use crate::models::Market;
use crate::xlsx::{
    MarketSheets, SheetAiaPolicy, SheetBond, SheetDeposit, SheetManualAsset, SheetStock,
    SheetYearFigure, SheetYearReview, WorkbookData,
};

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
    /// Rows whose `start_date` was recovered from the first `DD Mon YYYY`
    /// date in `note2` (only where `start_date` was still NULL).
    pub start_dates_seeded: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MonthStatReport {
    pub months_imported: usize,
    pub months_skipped: usize,
    pub items_imported: usize,
    /// `app_meta`/`manual_assets` seeds written this run.
    pub settings_seeded: usize,
    /// Existing rows whose hand-frozen H 月尾 was recovered (override was NULL).
    pub end_cash_seeded: usize,
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
pub struct BondReport {
    pub bonds_imported: usize,
    pub bonds_skipped: usize,
    pub coupons_imported: usize,
    pub coupons_skipped: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct YearReviewReport {
    /// Year blocks that seeded at least one figure into `year_review`.
    pub years_seeded: usize,
    /// Blocks with nothing to seed.
    pub years_skipped: usize,
    /// 投資P/L cells written onto the HK `year_snapshots` rows.
    pub sold_pl_seeded: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AiaReport {
    pub policies_imported: usize,
    pub policies_skipped: usize,
    /// 1 when the `aia.usd_hkd_rate` meta was seeded this run.
    pub rate_seeded: usize,
    /// Judgment calls worth eyeballing (e.g. the excluded flag could not be
    /// reconciled against the cached totals).
    pub warnings: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub hk: MarketReport,
    pub us: MarketReport,
    pub deposits: DepositReport,
    pub snapshots: SnapshotReport,
    pub mpf: MpfReport,
    pub bonds: BondReport,
    pub aia: AiaReport,
    pub months: MonthStatReport,
    pub year_review: YearReviewReport,
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
    let hk = import_market(pool, &data.hk).await?;
    let us = import_market(pool, &data.us).await?;
    let deposits = import_deposits(pool, &data.deposits).await?;
    let snapshots = import_year_snapshots(pool, &data.year_figures).await?;
    let mpf = import_mpf(pool, data).await?;
    let bonds = import_bonds(pool, &data.bonds).await?;
    let aia = import_aia(pool, data).await?;
    let months = import_months(pool, data).await?;
    // After the snapshots: the invested adjustment is seeded against the
    // effective (snapshot-aware) HK net invested.
    let year_review = import_year_review(pool, &data.year_review, &data.overview).await?;
    seed_market_history(pool).await?;
    seed_market_figures(pool, data).await?;
    Ok(ImportReport {
        hk,
        us,
        deposits,
        snapshots,
        mpf,
        bonds,
        aia,
        months,
        year_review,
    })
}

/// Seed `market_history` with one synthetic Dec-31 row per market per year
/// that has both 成本 and 總市值, so 上月/最高 have real history from day
/// one. `INSERT OR IGNORE` keeps a re-import from duplicating rows or
/// overwriting a real record that happens to land on a year-end date.
async fn seed_market_history(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO market_history \
         (market, recorded_on, buy_cost_priced, market_value, synthetic) \
         SELECT market, printf('%04d-12-31', year), cost, market_value, 1 \
         FROM year_snapshots WHERE cost IS NOT NULL AND market_value IS NOT NULL",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Seed each market's cached last-month/max cells. The `last month` cell
/// holds a rate only, so the synthetic previous-month-end row reconstructs
/// `(buy_cost_priced, market_value)` as `(cost, cost × (1 + rate))` with
/// `cost` = the market's imported Σ BUY total — the rate reproduces the
/// sheet exactly while the amount approximates last month's true figure,
/// the same concession MPF's import makes. `max Balance %`/`max net` are
/// lone maxima with no recoverable (cost, value) pair, so they live as
/// `app_meta` marks that floor the derived 最高. Re-imports are safe: the
/// upsert refreshes a synthetic seed on the same date but never overwrites
/// a real record (the `WHERE synthetic = 1` guard), and the marks just take
/// the sheet's latest values.
async fn seed_market_figures(pool: &SqlitePool, data: &WorkbookData) -> anyhow::Result<()> {
    let today = crate::routes::today();
    let first_of_month = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .ok_or_else(|| anyhow!("today {today} has no first of month"))?;
    let prev_month_end = first_of_month - chrono::Days::new(1);
    for market in [Market::Hk, Market::Us] {
        let cached = &data.market(market).cached;
        if let Some(rate) = cached.last_month_percent {
            let mut cost: f64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(t.total), 0.0) FROM trades t \
                 JOIN stocks s ON s.id = t.stock_id \
                 WHERE s.market = ? AND t.trade_type = 'BUY'",
            )
            .bind(market.as_str())
            .fetch_one(pool)
            .await?;
            // The rate reproduces the sheet exactly whatever cost we assume,
            // so shrink the assumed cost if the implied amount would exceed
            // the sheet's own `max net` — the row stays consistent with both
            // cached figures.
            if rate > 0.0
                && cost > 0.0
                && let Some(max_amount) = cached.max_amount.filter(|m| *m > 0.0)
            {
                cost = cost.min(max_amount / rate);
            }
            // A rate at or below −100% — or a non-finite cell — would seed a
            // worthless row, so skip it rather than store garbage.
            let market_value = cost * (1.0 + rate);
            if cost > 0.0 && market_value > 0.0 {
                sqlx::query(
                    "INSERT INTO market_history \
                     (market, recorded_on, buy_cost_priced, market_value, synthetic) \
                     VALUES (?, ?, ?, ?, 1) \
                     ON CONFLICT (market, recorded_on) DO UPDATE SET \
                     buy_cost_priced = excluded.buy_cost_priced, \
                     market_value = excluded.market_value \
                     WHERE market_history.synthetic = 1",
                )
                .bind(market.as_str())
                .bind(prev_month_end.to_string())
                .bind(cost)
                .bind(market_value)
                .execute(pool)
                .await?;
            }
        }
        crate::mpf::meta_put(
            pool,
            &crate::market_history::seed_max_percent_key(market),
            cached.max_percent.map(|v| v.to_string()).as_deref(),
        )
        .await?;
        crate::mpf::meta_put(
            pool,
            &crate::market_history::seed_max_amount_key(market),
            cached.max_amount.map(|v| v.to_string()).as_deref(),
        )
        .await?;
    }
    Ok(())
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

/// Seed YearInReview's hand-entered figures: 收入 and the invested
/// adjustment (`sheet invested − effective HK net invested`) for every block
/// year, plus the bond/deposit cells for years before the current one — the
/// current year derives live, like `year_snapshots`. 投資P/L seeds the HK
/// snapshot's `sold_pl` (may be negative). NULL-filling upserts keep seeded
/// and hand-edited values across re-imports.
/// The adjustment that makes a year's `invested` match its sheet cell:
/// `sheet invested − effective HK net invested − the year's IBKR 轉入` — the
/// snapshot-aware HK figure and the transfer-log sum both already derive, so
/// the adjustment keeps only the remainder (e.g. the 110000 silver-bond
/// purchase the sheet folds in by hand).
async fn invested_adjustment_for(
    pool: &SqlitePool,
    year: i32,
    sheet_invested: Option<f64>,
) -> anyhow::Result<Option<f64>> {
    let Some(sheet_invested) = sheet_invested else {
        return Ok(None);
    };
    // The snapshot-aware HK net invested, matching the yearly row.
    let hk_invested: Option<f64> = sqlx::query_scalar(
        "SELECT COALESCE(\
            (SELECT invested FROM year_snapshots WHERE market = 'HK' AND year = ?),\
            (SELECT SUM(CASE WHEN t.trade_type = 'BUY' THEN t.total ELSE -t.total END) \
             FROM trades t JOIN stocks s ON s.id = t.stock_id \
             WHERE s.market = 'HK' \
             AND CAST(strftime('%Y', t.trade_date) AS INTEGER) = ?))",
    )
    .bind(year)
    .bind(year)
    .fetch_one(pool)
    .await?;
    // The sheet's invested cell folds the year's IBKR 轉入 in (美股!B1);
    // that part derives from the transfer log, so the adjustment keeps
    // only the remainder.
    let transferred: Option<f64> = sqlx::query_scalar(
        "SELECT SUM(amount_hkd) FROM ibkr_transfers \
         WHERE CAST(strftime('%Y', transfer_date) AS INTEGER) = ?",
    )
    .bind(year)
    .fetch_one(pool)
    .await?;
    Ok(Some(
        sheet_invested - hk_invested.unwrap_or(0.0) - transferred.unwrap_or(0.0),
    ))
}

async fn import_year_review(
    pool: &SqlitePool,
    blocks: &[SheetYearReview],
    overview: &crate::xlsx::OverviewCached,
) -> anyhow::Result<YearReviewReport> {
    let current_year = crate::routes::today().year();
    let mut report = YearReviewReport::default();
    for block in blocks {
        if let Some(sold_pl) = block.sold_pl {
            sqlx::query(
                "INSERT INTO year_snapshots (market, year, sold_pl, updated_at) \
                 VALUES ('HK', ?, ?, ?) \
                 ON CONFLICT (market, year) DO UPDATE SET \
                 sold_pl = COALESCE(year_snapshots.sold_pl, excluded.sold_pl)",
            )
            .bind(block.year)
            .bind(sold_pl)
            .bind(crate::routes::now_timestamp())
            .execute(pool)
            .await?;
            report.sold_pl_seeded += 1;
        }

        let invested_adjustment = invested_adjustment_for(pool, block.year, block.invested).await?;

        // Bond/deposit cells seed as overrides only for past years — the
        // sheets delete matured entries, so history needs the frozen cells,
        // while the live year keeps deriving.
        let past_year = block.year < current_year;
        let (bond_principal, bond_interest, deposit_principal, deposit_interest) = if past_year {
            (
                block.bond_principal,
                block.bond_interest,
                block.deposit_principal,
                block.deposit_interest,
            )
        } else {
            (None, None, None, None)
        };

        if [
            block.income,
            invested_adjustment,
            bond_principal,
            bond_interest,
            deposit_principal,
            deposit_interest,
        ]
        .iter()
        .all(Option::is_none)
        {
            report.years_skipped += 1;
            continue;
        }
        sqlx::query(
            "INSERT INTO year_review \
             (year, income, invested_adjustment, bond_principal, bond_interest, \
              deposit_principal, deposit_interest, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (year) DO UPDATE SET \
             income = COALESCE(year_review.income, excluded.income), \
             invested_adjustment = COALESCE(year_review.invested_adjustment, \
                 excluded.invested_adjustment), \
             bond_principal = COALESCE(year_review.bond_principal, excluded.bond_principal), \
             bond_interest = COALESCE(year_review.bond_interest, excluded.bond_interest), \
             deposit_principal = COALESCE(year_review.deposit_principal, \
                 excluded.deposit_principal), \
             deposit_interest = COALESCE(year_review.deposit_interest, \
                 excluded.deposit_interest)",
        )
        .bind(block.year)
        .bind(block.income)
        .bind(invested_adjustment)
        .bind(bond_principal)
        .bind(bond_interest)
        .bind(deposit_principal)
        .bind(deposit_interest)
        .bind(crate::routes::now_timestamp())
        .execute(pool)
        .await?;
        report.years_seeded += 1;
    }

    // The Overview J:K cells carry `invested` for years the YearInReview
    // sheet has no block for (e.g. 2023's hand-entered 206523.15); seed the
    // same adjustment so those years reproduce the sheet figure.
    let block_years: std::collections::BTreeSet<i32> =
        blocks.iter().map(|block| block.year).collect();
    for row in &overview.invest_targets {
        if block_years.contains(&row.year) {
            continue;
        }
        let Some(invested_adjustment) =
            invested_adjustment_for(pool, row.year, row.invested).await?
        else {
            continue;
        };
        sqlx::query(
            "INSERT INTO year_review (year, invested_adjustment, updated_at) \
             VALUES (?, ?, ?) \
             ON CONFLICT (year) DO UPDATE SET \
             invested_adjustment = COALESCE(year_review.invested_adjustment, \
                 excluded.invested_adjustment)",
        )
        .bind(row.year)
        .bind(invested_adjustment)
        .bind(crate::routes::now_timestamp())
        .execute(pool)
        .await?;
        report.years_seeded += 1;
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
        if let Some(remaining) = existing.get_mut(&key)
            && *remaining > 0
        {
            *remaining -= 1;
            report.trades_skipped += 1;
            continue;
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
        if let Some(remaining) = existing.get_mut(&key)
            && *remaining > 0
        {
            *remaining -= 1;
            report.dividends_skipped += 1;
            continue;
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
            start_date: note2_start_date(deposit.note2.as_deref()).as_deref(),
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
        if let Some(remaining) = existing.get_mut(&key)
            && *remaining > 0
        {
            *remaining -= 1;
            report.deposits_skipped += 1;
            // Backfill start_date on rows already stored without one —
            // a non-NULL start_date is never overwritten.
            if let Some(start_date) = note2_start_date(deposit.note2.as_deref()) {
                report.start_dates_seeded += sqlx::query(
                    "UPDATE deposits SET start_date = ? \
                         WHERE start_date IS NULL AND note2 = ? AND end_date = ?",
                )
                .bind(&start_date)
                .bind(deposit.note2.as_deref())
                .bind(&deposit.end_date)
                .execute(pool)
                .await?
                .rows_affected() as usize;
            }
            continue;
        }

        let now = crate::routes::now_timestamp();
        // A deposit already past its end date counts as received (its interest
        // fed 利息 all along); future deposits stay unreceived until 收訖.
        let received_at = (validated.end_date.as_str()
            <= crate::routes::today().to_string().as_str())
        .then(|| validated.end_date.clone());
        sqlx::query(
            "INSERT INTO deposits (label, bank, principal, rate, interest, start_date, end_date, \
             received_at, note1, note2, sort_order, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(validated.label.as_deref())
        .bind(validated.bank.as_deref())
        .bind(validated.principal)
        .bind(validated.rate)
        .bind(validated.interest)
        .bind(validated.start_date.as_deref())
        .bind(&validated.end_date)
        .bind(&received_at)
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

/// The first `DD Mon YYYY` date inside a `note2` string like
/// `17 Jun 2026 to 02 Aug 2026: 2.60%` — the deposit's start date.
fn note2_start_date(note2: Option<&str>) -> Option<String> {
    let month_of = |name: &str| -> Option<u32> {
        match name.get(..3)? {
            "Jan" => Some(1),
            "Feb" => Some(2),
            "Mar" => Some(3),
            "Apr" => Some(4),
            "May" => Some(5),
            "Jun" => Some(6),
            "Jul" => Some(7),
            "Aug" => Some(8),
            "Sep" => Some(9),
            "Oct" => Some(10),
            "Nov" => Some(11),
            "Dec" => Some(12),
            _ => None,
        }
    };
    let tokens: Vec<String> = note2?
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|c: char| ",:;().".contains(c))
                .to_string()
        })
        .collect();
    for window in tokens.windows(3) {
        let Ok(day) = window[0].parse::<u32>() else {
            continue;
        };
        let Some(month) = month_of(&window[1]) else {
            continue;
        };
        let Ok(year) = window[2].parse::<i32>() else {
            continue;
        };
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
            return Some(date.to_string());
        }
    }
    None
}

/// Month Stat rows are keyed by month: an existing row skips whole, and its
/// items belong to it (no separate dedupe). Settings and the manual Overview
/// cells seed only while unset — after first import the user owns them.
async fn import_months(pool: &SqlitePool, data: &WorkbookData) -> anyhow::Result<MonthStatReport> {
    let mut report = MonthStatReport::default();
    let now = crate::routes::now_timestamp();
    let existing: std::collections::HashSet<String> =
        sqlx::query_scalar::<_, String>("SELECT month FROM month_stats")
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();

    // 利息 is derived (auto events + `interest` items), so the sheet's N cell
    // imports as a residual item: the part deposits/coupons/dividends do not
    // explain. Events are loaded once — months import after all sources.
    let interest_events = crate::routes::months::load_interest_events(pool).await?;

    for month in &data.month_stat.months {
        // A hand-frozen H cell keeps its cached value as an override; the
        // `=F(n+1) − salary` chain rows derive, and a blank H stores NULL.
        let end_cash_override = month
            .end_cash_frozen
            .then_some(month.derived.end_cash)
            .flatten();
        if existing.contains(&month.month) {
            report.months_skipped += 1;
            if let Some(end_cash) = end_cash_override {
                let updated = sqlx::query(
                    "UPDATE month_stats SET end_cash_override = ? \
                     WHERE month = ? AND end_cash_override IS NULL",
                )
                .bind(end_cash)
                .bind(&month.month)
                .execute(pool)
                .await?;
                if updated.rows_affected() > 0 {
                    report.end_cash_seeded += 1;
                }
            }
            continue;
        }
        let mut tx = pool.begin().await?;
        sqlx::query(
            "INSERT INTO month_stats (month, start_cash, salary, total_assets, liquid_assets, \
             pool_input, end_cash_override, note, created_at, \
             updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(&month.month)
        .bind(month.start_cash)
        .bind(month.salary)
        .bind(month.total_assets)
        .bind(month.liquid_assets)
        .bind(month.pool_input)
        .bind(end_cash_override)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("inserting Month Stat row {}", month.source_row))?;
        for item in &month.items {
            sqlx::query(
                "INSERT INTO month_items (month, category, label, amount, auto_key, note, \
                 created_at) VALUES (?, ?, NULL, ?, NULL, ?, ?)",
            )
            .bind(&month.month)
            .bind(item.category.as_str())
            .bind(item.amount)
            .bind(item.note.as_deref())
            .bind(&now)
            .execute(&mut *tx)
            .await?;
            report.items_imported += 1;
        }
        // Sheet N − the auto events = the manual part (bank 活期, promos);
        // blank cells and exact matches leave no item, and sub-cent float
        // noise is skipped like in migration 0017. The residual subtracts ALL
        // in-month components — including still-unreceived deposits — so a
        // sheet cell typed ahead of 收訖 is not double-counted later.
        let auto: f64 = crate::calc::interest_components(&month.month, &interest_events)
            .iter()
            .map(|component| component.amount.unwrap_or(0.0))
            .sum();
        let residual = month.interest - auto;
        if month.interest != 0.0 && residual.abs() >= 0.005 {
            sqlx::query(
                "INSERT INTO month_items (month, category, label, amount, note, \
                 created_at) VALUES (?, 'interest', '其他利息', ?, '匯入差額', ?)",
            )
            .bind(&month.month)
            .bind(residual)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
            report.items_imported += 1;
        }
        tx.commit().await?;
        report.months_imported += 1;
    }

    let current_year = crate::routes::today().year();
    if crate::mpf::meta_get(pool, crate::routes::months::SALARY_KEY)
        .await?
        .is_none()
        && let Some(salary) = data.overview.salary
    {
        crate::mpf::meta_put(
            pool,
            crate::routes::months::SALARY_KEY,
            Some(&salary.to_string()),
        )
        .await?;
        report.settings_seeded += 1;
    }
    let rate_key = crate::routes::months::pool_rate_key;
    if crate::mpf::meta_get(pool, &rate_key(current_year))
        .await?
        .is_none()
        && let Some(rate) = data.overview.pool_rate
    {
        crate::mpf::meta_put(pool, &rate_key(current_year), Some(&rate.to_string())).await?;
        report.settings_seeded += 1;
    }
    // Past years' rates solve from the sheet's pool chain:
    // M(y) = M(y−1) + H(y)·rate − G(y) + N(y).
    let mut prev_balance = 0.0;
    for year in &data.month_stat.years {
        let Some(balance) = year.pool_balance else {
            continue;
        };
        let interest = year.interest_sum.unwrap_or(0.0);
        if year.year < current_year
            && interest != 0.0
            && crate::mpf::meta_get(pool, &rate_key(year.year))
                .await?
                .is_none()
        {
            let rate = (balance - prev_balance + year.entertainment_sum.unwrap_or(0.0)
                - year.pool_input_sum.unwrap_or(0.0))
                / interest;
            crate::mpf::meta_put(pool, &rate_key(year.year), Some(&rate.to_string())).await?;
            report.settings_seeded += 1;
        }
        prev_balance = balance;
    }

    // The four manual Overview cells seed manual_assets only while the table
    // is untouched.
    report.settings_seeded += seed_manual_assets(pool, &data.overview.manual_assets, &now).await?;

    // The 美股 sheet's IBKR account cells seed `app_meta` once, like the salary.
    for (key, value) in [
        (
            crate::routes::overview::IBKR_NOW_VALUE_KEY,
            data.us_account.now_value,
        ),
        (
            crate::routes::overview::IBKR_HKD_CASH_KEY,
            data.us_account.hkd_cash,
        ),
        (
            crate::routes::overview::IBKR_USD_CASH_KEY,
            data.us_account.usd_cash,
        ),
    ] {
        if let Some(value) = value
            && crate::mpf::meta_get(pool, key).await?.is_none()
        {
            crate::mpf::meta_put(pool, key, Some(&value.to_string())).await?;
            report.settings_seeded += 1;
        }
    }

    // The cumulative B1 becomes the transfer log's first entry, dated to the
    // first US trade — funding precedes the first buy — so each year's 轉入
    // derives from the log.
    if let Some(total) = data
        .us_account
        .transferred_hkd
        .filter(|total| *total != 0.0)
    {
        let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ibkr_transfers")
            .fetch_one(pool)
            .await?;
        if stored == 0 {
            let first_trade: Option<String> = sqlx::query_scalar(
                "SELECT MIN(t.trade_date) FROM trades t \
                 JOIN stocks s ON s.id = t.stock_id WHERE s.market = 'US'",
            )
            .fetch_one(pool)
            .await?;
            sqlx::query(
                "INSERT INTO ibkr_transfers (transfer_date, amount_hkd, created_at) \
                 VALUES (?, ?, ?)",
            )
            .bind(first_trade.unwrap_or_else(|| crate::routes::today().to_string()))
            .bind(total)
            .bind(&now)
            .execute(pool)
            .await?;
            report.settings_seeded += 1;
        }
    }

    Ok(report)
}

/// The four manual Overview cells seed `manual_assets` — with the sheet's
/// 策略 liquidity (B7 Irene `short`, B8 HS人壽 `long`) — only while the table
/// is untouched; a user reclassification survives re-import. Returns the
/// number of rows seeded.
async fn seed_manual_assets(
    pool: &SqlitePool,
    assets: &[SheetManualAsset],
    now: &str,
) -> anyhow::Result<usize> {
    let stored_assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM manual_assets")
        .fetch_one(pool)
        .await?;
    if stored_assets != 0 {
        return Ok(0);
    }
    let mut seeded = 0;
    for (index, asset) in assets.iter().enumerate() {
        sqlx::query(
            "INSERT INTO manual_assets (label, kind, liquidity, amount, sort_order, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&asset.label)
        .bind(asset.kind.as_str())
        .bind(asset.liquidity.as_str())
        .bind(asset.amount)
        .bind((index + 1) as i64)
        .bind(now)
        .execute(pool)
        .await?;
        seeded += 1;
    }
    Ok(seeded)
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

/// 債券 registry rows dedupe by 發行編號 (falling back to the row's natural
/// fields when the sheet has none); coupons dedupe by (bond, pay_date). A
/// coupon already past its pay date imports as received — the sheet's
/// interest value is what was actually paid — while future ones stay
/// unreceived, the same heuristic dividends use.
async fn import_bonds(pool: &SqlitePool, bonds: &[SheetBond]) -> anyhow::Result<BondReport> {
    let mut report = BondReport::default();
    let today = crate::routes::today();
    let mut bond_ids = existing_bond_ids(pool).await?;
    let mut coupon_keys = existing_coupon_keys(pool).await?;

    for bond in bonds {
        let existing_id = bond
            .issue_no
            .as_deref()
            .and_then(|issue_no| bond_ids.by_issue.get(issue_no).copied())
            .or_else(|| bond_ids.by_natural.get(&BondNaturalKey::new(bond)).copied());

        let bond_id = match existing_id {
            Some(id) => {
                report.bonds_skipped += 1;
                id
            }
            None => {
                let validated = validate_bond(BondInput {
                    label: &bond.label,
                    issue_no: bond.issue_no.as_deref(),
                    principal: bond.principal,
                    maturity_date: &bond.maturity_date,
                })
                .map_err(|errors| {
                    anyhow!(
                        "row {} of the {} sheet is not valid: {}",
                        bond.source_row,
                        crate::xlsx::BOND_SHEET,
                        errors
                            .iter()
                            .map(|e| format!("{}: {}", e.field, e.message))
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                })?;
                let id = crate::routes::bonds::insert_bond(
                    pool,
                    &validated,
                    bond.sort_order,
                    &crate::models::NewBond {
                        label: validated.label.clone(),
                        issue_no: None,
                        principal: validated.principal,
                        maturity_date: validated.maturity_date.clone(),
                        note: None,
                    },
                )
                .await
                .with_context(|| format!("inserting bond row {} of 債券", bond.source_row))?;
                bond_ids.register(id, bond);
                report.bonds_imported += 1;
                id
            }
        };

        for coupon in &bond.coupons {
            if !coupon_keys.insert((bond_id, coupon.pay_date.clone())) {
                report.coupons_skipped += 1;
                continue;
            }
            let pay_date = chrono::NaiveDate::parse_from_str(&coupon.pay_date, "%Y-%m-%d")
                .map_err(|_| {
                    anyhow!(
                        "row {} of the {} sheet has no usable 付息日",
                        coupon.source_row,
                        crate::xlsx::BOND_SHEET
                    )
                })?;
            // Past-dated coupons auto-credited: the sheet's interest value is
            // the amount received. A future or amountless row stays unreceived.
            let received_amount = coupon.interest.filter(|_| pay_date <= today);
            let validated = validate_coupon(CouponInput {
                pay_date: &coupon.pay_date,
                fixing_date: coupon.fixing_date.as_deref(),
                annual_rate: coupon.annual_rate,
                per_10k: coupon.per_10k,
                received_amount,
            })
            .map_err(|errors| {
                anyhow!(
                    "row {} of the {} sheet is not valid: {}",
                    coupon.source_row,
                    crate::xlsx::BOND_SHEET,
                    errors
                        .iter()
                        .map(|e| format!("{}: {}", e.field, e.message))
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            })?;
            crate::routes::bonds::insert_coupon(
                pool,
                &validated,
                &crate::models::NewBondCoupon {
                    bond_id,
                    pay_date: validated.pay_date.clone(),
                    fixing_date: None,
                    annual_rate: None,
                    per_10k: None,
                    received_amount: None,
                    note: None,
                },
            )
            .await
            .with_context(|| format!("inserting coupon row {} of 債券", coupon.source_row))?;
            report.coupons_imported += 1;
        }
    }
    Ok(report)
}

/// Which rows the sheet's USD totals leave out: compare Σ premium/Σ value
/// over parsed rows to the cached `buy usd`/`now usd` cells. The row whose
/// removal reconciles both is the share the sheet subtracts (today: `irene
/// 20%`). Ambiguous or missing evidence flags nothing and reports a warning —
/// the flag stays editable in the app.
fn reconcile_aia_excluded(
    policies: &[SheetAiaPolicy],
    cached: &crate::xlsx::AiaSheetCached,
) -> (Vec<bool>, Option<String>) {
    let (Some(buy), Some(now)) = (cached.buy_usd, cached.now_usd) else {
        return (
            vec![false; policies.len()],
            Some(
                "AIA sheet's cached buy usd/now usd cells are missing; no row flagged excluded"
                    .to_string(),
            ),
        );
    };
    let sum_premium: f64 = policies.iter().map(|p| p.premium_usd).sum();
    let sum_value: f64 = policies.iter().map(|p| p.value_usd).sum();
    if approx_eq(sum_premium, buy) && approx_eq(sum_value, now) {
        return (vec![false; policies.len()], None);
    }
    let matches: Vec<usize> = (0..policies.len())
        .filter(|&i| {
            approx_eq(sum_premium - policies[i].premium_usd, buy)
                && approx_eq(sum_value - policies[i].value_usd, now)
        })
        .collect();
    match matches.as_slice() {
        [i] => {
            let mut excluded = vec![false; policies.len()];
            excluded[*i] = true;
            (excluded, None)
        }
        _ => (
            vec![false; policies.len()],
            Some(format!(
                "could not reconcile the AIA excluded row against cached totals \
                 ({} candidate(s) out of {} rows); no row flagged excluded",
                matches.len(),
                policies.len()
            )),
        ),
    }
}

/// A row already stored imports as a skip: `policy_no` is the natural key when
/// both sides carry one, else the row's own figures identify it.
async fn import_aia(pool: &SqlitePool, data: &WorkbookData) -> anyhow::Result<AiaReport> {
    let mut report = AiaReport::default();

    // The manual rate seeds once from Overview!N3 and is never overwritten —
    // after first import the user owns it (PATCH /api/aia/rate).
    if crate::mpf::meta_get(pool, crate::routes::aia::RATE_KEY)
        .await?
        .is_none()
        && let Some(rate) = data.aia_cached.usd_hkd_rate
    {
        crate::mpf::meta_put(pool, crate::routes::aia::RATE_KEY, Some(&rate.to_string())).await?;
        report.rate_seeded = 1;
    }

    let (excluded, warning) = reconcile_aia_excluded(&data.aia, &data.aia_cached);
    if let Some(warning) = warning {
        report.warnings.push(warning);
    }

    let existing: Vec<(i64, Option<String>, String, f64, f64)> =
        sqlx::query_as("SELECT id, policy_no, label, premium_usd, value_usd FROM aia_policies")
            .fetch_all(pool)
            .await?;
    let by_policy_no: HashMap<&str, i64> = existing
        .iter()
        .filter_map(|(id, policy_no, _, _, _)| {
            policy_no.as_deref().map(|policy_no| (policy_no, *id))
        })
        .collect();

    for (index, policy) in data.aia.iter().enumerate() {
        let already = policy
            .policy_no
            .as_deref()
            .and_then(|policy_no| by_policy_no.get(policy_no))
            .is_some()
            || existing.iter().any(|(_, _, label, premium, value)| {
                *label == policy.label
                    && approx_eq(*premium, policy.premium_usd)
                    && approx_eq(*value, policy.value_usd)
            });
        if already {
            report.policies_skipped += 1;
            continue;
        }
        let validated = validate_aia_policy(AiaPolicyInput {
            label: &policy.label,
            policy_no: policy.policy_no.as_deref(),
            next_pay_date: policy.next_pay_date.as_deref(),
            premium_usd: policy.premium_usd,
            value_usd: policy.value_usd,
            remaining_years: policy.remaining_years,
            withdrew_usd: policy.withdrew_usd,
            note: policy.note.as_deref(),
            link: None,
            excluded: excluded[index],
            in_account: policy.in_account,
        })
        .map_err(|errors| {
            anyhow!(
                "row {} of the {} sheet is not valid: {}",
                policy.source_row,
                crate::xlsx::AIA_SHEET,
                errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;
        crate::routes::aia::insert_policy(pool, &validated, policy.sort_order)
            .await
            .with_context(|| {
                format!(
                    "inserting policy row {} of {}",
                    policy.source_row,
                    crate::xlsx::AIA_SHEET
                )
            })?;
        report.policies_imported += 1;
    }
    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BondNaturalKey {
    label: String,
    principal: u64,
    maturity_date: String,
}

impl BondNaturalKey {
    fn new(bond: &SheetBond) -> Self {
        Self {
            label: bond.label.clone(),
            principal: bond.principal.to_bits(),
            maturity_date: bond.maturity_date.clone(),
        }
    }
}

#[derive(Debug, Default)]
struct ExistingBonds {
    by_issue: HashMap<String, i64>,
    by_natural: HashMap<BondNaturalKey, i64>,
}

impl ExistingBonds {
    fn register(&mut self, id: i64, bond: &SheetBond) {
        if let Some(issue_no) = &bond.issue_no {
            self.by_issue.insert(issue_no.clone(), id);
        }
        self.by_natural.insert(BondNaturalKey::new(bond), id);
    }
}

async fn existing_bond_ids(pool: &SqlitePool) -> anyhow::Result<ExistingBonds> {
    let rows = sqlx::query("SELECT id, label, issue_no, principal, maturity_date FROM bonds")
        .fetch_all(pool)
        .await?;

    let mut existing = ExistingBonds::default();
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        let issue_no: Option<String> = row.try_get("issue_no")?;
        if let Some(issue_no) = issue_no {
            existing.by_issue.insert(issue_no, id);
        }
        existing.by_natural.insert(
            BondNaturalKey {
                label: row.try_get("label")?,
                principal: row.try_get::<f64, _>("principal")?.to_bits(),
                maturity_date: row.try_get("maturity_date")?,
            },
            id,
        );
    }
    Ok(existing)
}

async fn existing_coupon_keys(
    pool: &SqlitePool,
) -> anyhow::Result<std::collections::HashSet<(i64, String)>> {
    let rows = sqlx::query("SELECT bond_id, pay_date FROM bond_coupons")
        .fetch_all(pool)
        .await?;
    let mut keys = std::collections::HashSet::new();
    for row in &rows {
        keys.insert((row.try_get("bond_id")?, row.try_get("pay_date")?));
    }
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::models::{ManualAssetKind, ManualAssetLiquidity};

    fn seed_rows() -> Vec<SheetManualAsset> {
        vec![
            SheetManualAsset {
                label: "Irene".to_string(),
                kind: ManualAssetKind::Asset,
                liquidity: ManualAssetLiquidity::Short,
                amount: 20000.0,
            },
            SheetManualAsset {
                label: "HS人壽".to_string(),
                kind: ManualAssetKind::Asset,
                liquidity: ManualAssetLiquidity::Long,
                amount: 69440.47,
            },
            SheetManualAsset {
                label: "HS".to_string(),
                kind: ManualAssetKind::Cash,
                liquidity: ManualAssetLiquidity::Long,
                amount: 30538.78,
            },
        ]
    }

    #[tokio::test]
    async fn manual_assets_seed_liquidity_once() {
        let pool = connect_memory().await.expect("memory db");
        let seeded = seed_manual_assets(&pool, &seed_rows(), "2026-09-01T00:00:00")
            .await
            .expect("seed");
        assert_eq!(seeded, 3);

        let stored = crate::routes::months::load_assets(&pool)
            .await
            .expect("load");
        let by_label = |label: &str| {
            stored
                .iter()
                .find(|asset| asset.label == label)
                .expect(label)
        };
        assert_eq!(by_label("Irene").liquidity, ManualAssetLiquidity::Short);
        assert_eq!(by_label("HS人壽").liquidity, ManualAssetLiquidity::Long);
        assert_eq!(by_label("HS").liquidity, ManualAssetLiquidity::Long);

        // A reclassification survives re-import: the seed only runs on an
        // empty table.
        sqlx::query("UPDATE manual_assets SET liquidity = 'long' WHERE label = 'Irene'")
            .execute(&pool)
            .await
            .unwrap();
        let seeded = seed_manual_assets(&pool, &seed_rows(), "2026-10-01T00:00:00")
            .await
            .expect("re-seed");
        assert_eq!(seeded, 0);
        let stored = crate::routes::months::load_assets(&pool)
            .await
            .expect("load");
        assert_eq!(
            stored
                .iter()
                .find(|asset| asset.label == "Irene")
                .unwrap()
                .liquidity,
            ManualAssetLiquidity::Long
        );
    }
}
