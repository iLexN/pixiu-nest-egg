use std::path::PathBuf;

use wealth_backend::models::{InputMode, Market};
use wealth_backend::parity::Outcome;
use wealth_backend::xlsx::{MarketSheets, SheetStock, SheetTrade, WorkbookData};
use wealth_backend::{db, import, parity, xlsx};

fn workbook_path() -> PathBuf {
    PathBuf::from("../財富分析報告.xlsx")
}

async fn count(pool: &sqlx::SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
        .fetch_one(pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn imports_every_trade_row_then_skips_them_on_a_second_run() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");

    let first = import::import(&pool, &data).await.expect("first import");
    assert_eq!(first.hk.trades_imported, 24);
    assert_eq!(first.us.trades_imported, 10);
    assert_eq!(first.hk.trades_skipped, 0);
    assert_eq!(count(&pool, "trades").await, 34);

    // 港股 lists 11 stocks (10 traded + ＦＧ恆生紅利) plus 香港寬頻, which only
    // appears in the 派息 block; 美股 lists 4.
    assert_eq!(first.hk.stocks_created, 12);
    assert_eq!(first.us.stocks_created, 4);
    assert_eq!(count(&pool, "stocks").await, 16);

    let second = import::import(&pool, &data).await.expect("second import");
    assert_eq!(second.hk.trades_imported, 0);
    assert_eq!(second.us.trades_imported, 0);
    assert_eq!(second.hk.trades_skipped, 24);
    assert_eq!(second.us.trades_skipped, 10);
    assert_eq!(second.hk.stocks_created, 0);
    assert_eq!(second.us.stocks_created, 0);
    assert_eq!(count(&pool, "trades").await, 34);
    assert_eq!(count(&pool, "stocks").await, 16);
}

#[tokio::test]
async fn imports_every_deposit_row_then_skips_them_on_a_second_run() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");

    let first = import::import(&pool, &data).await.expect("first import");
    assert_eq!(first.deposits.deposits_imported, 23);
    assert_eq!(first.deposits.deposits_skipped, 0);
    assert_eq!(count(&pool, "deposits").await, 23);

    // Workbook row order is preserved as sort_order.
    let first_label: Option<String> =
        sqlx::query_scalar("SELECT label FROM deposits ORDER BY sort_order LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("first label");
    assert_eq!(first_label.as_deref(), Some("HS-74"));

    let second = import::import(&pool, &data).await.expect("second import");
    assert_eq!(second.deposits.deposits_imported, 0);
    assert_eq!(second.deposits.deposits_skipped, 23);
    assert_eq!(count(&pool, "deposits").await, 23);
}

#[tokio::test]
async fn imports_every_dividend_row_then_skips_them_on_a_second_run() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");

    let first = import::import(&pool, &data).await.expect("first import");
    assert_eq!(first.hk.dividends_imported, 43);
    assert_eq!(first.us.dividends_imported, 0);
    assert_eq!(count(&pool, "dividends").await, 43);

    // The earliest 中國銀行 row carries the sheet's frozen denominators:
    // 6767.35 ÷ 0.0701… ≈ 96523.15 buy cost over the 30000 shares held then.
    let row = sqlx::query(
        "SELECT d.shares_held, d.buy_cost, d.received_amount, d.received_price \
         FROM dividends d JOIN stocks s ON s.id = d.stock_id \
         WHERE s.code = '中國銀行' ORDER BY d.pay_date LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("first dividend");
    use sqlx::Row;
    let shares: f64 = row.try_get(0).expect("shares");
    let cost: f64 = row.try_get(1).expect("cost");
    let received: f64 = row.try_get(2).expect("received");
    assert!((shares - 30000.0).abs() < 0.01, "shares = {shares}");
    assert!((cost - 96523.15).abs() < 0.01, "cost = {cost}");
    assert!((received - 6767.35).abs() < 0.01, "received = {received}");
    assert!(row.try_get::<Option<f64>, _>(3).expect("price").is_none());

    // A later row recovered the price snapshot from M ÷ (N × O).
    let price: Option<f64> = sqlx::query_scalar(
        "SELECT d.received_price FROM dividends d JOIN stocks s ON s.id = d.stock_id \
         WHERE s.code = '中國銀行' AND d.pay_date = '2026-08-19'",
    )
    .fetch_one(&pool)
    .await
    .expect("priced dividend");
    assert!((price.expect("price") - 5.41).abs() < 0.01);

    let second = import::import(&pool, &data).await.expect("second import");
    assert_eq!(second.hk.dividends_imported, 0);
    assert_eq!(second.hk.dividends_skipped, 43);
    assert_eq!(count(&pool, "dividends").await, 43);
}

#[tokio::test]
async fn imports_the_bond_and_its_coupons_then_skips_them_on_a_second_run() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");

    let first = import::import(&pool, &data).await.expect("first import");
    assert_eq!(first.bonds.bonds_imported, 1);
    assert_eq!(first.bonds.coupons_imported, 6);
    assert_eq!(count(&pool, "bonds").await, 1);
    assert_eq!(count(&pool, "bond_coupons").await, 6);

    // The silver bond registry row and its serial-date maturity.
    let bond: (String, String, f64, String) =
        sqlx::query_as("SELECT label, issue_no, principal, maturity_date FROM bonds")
            .fetch_one(&pool)
            .await
            .expect("the bond");
    assert_eq!(bond.0, "silver bond");
    assert_eq!(bond.1, "03GB2710R");
    assert!((bond.2 - 50000.0).abs() < 0.01);
    assert_eq!(bond.3, "2027-10-25");

    // Past-dated coupons imported as received with the sheet's cached
    // interest; the 待定 rows imported with null rate/per_10k.
    let received: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bond_coupons WHERE received_amount IS NOT NULL")
            .fetch_one(&pool)
            .await
            .expect("received count");
    assert_eq!(received, 3);
    let unfixed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bond_coupons WHERE annual_rate IS NULL AND per_10k IS NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("unfixed count");
    assert_eq!(unfixed, 3);
    // The sheet's hand-entered 1000 (not 199.45 × 5) is what imported.
    let amount: f64 = sqlx::query_scalar(
        "SELECT received_amount FROM bond_coupons WHERE pay_date = '2026-04-23'",
    )
    .fetch_one(&pool)
    .await
    .expect("april coupon");
    assert!((amount - 1000.0).abs() < 0.01);

    let second = import::import(&pool, &data).await.expect("second import");
    assert_eq!(second.bonds.bonds_imported, 0);
    assert_eq!(second.bonds.bonds_skipped, 1);
    assert_eq!(second.bonds.coupons_imported, 0);
    assert_eq!(second.bonds.coupons_skipped, 6);
    assert_eq!(count(&pool, "bonds").await, 1);
    assert_eq!(count(&pool, "bond_coupons").await, 6);
}

#[tokio::test]
async fn imports_mpf_accounts_then_skips_them_on_a_second_run() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");

    let first = import::import(&pool, &data).await.expect("first import");
    assert_eq!(first.mpf.accounts_created, 2);
    assert_eq!(first.mpf.accounts_skipped, 0);
    assert_eq!(count(&pool, "mpf_accounts").await, 2);
    // Each account seeded one synthetic last-month history row.
    assert_eq!(count(&pool, "mpf_history").await, 2);

    // The seeded max rate reproduces the sheet's cached max column.
    let seed: Option<f64> =
        sqlx::query_scalar("SELECT seed_max_rate FROM mpf_accounts WHERE label = '強積金個人帳戶'")
            .fetch_one(&pool)
            .await
            .expect("seed max rate");
    assert!((seed.expect("seed") - 0.5273351333828047).abs() < 1e-9);

    // With two accounts the frozen last-month figures recover the actual
    // month-end contributions: the 'new type' seed backs out the 3,000
    // added in September, while the other account is unchanged.
    let past: f64 = sqlx::query_scalar(
        "SELECT h.contributions FROM mpf_history h \
         JOIN mpf_accounts a ON a.id = h.account_id \
         WHERE a.label = 'new type' AND h.synthetic = 1",
    )
    .fetch_one(&pool)
    .await
    .expect("seeded contributions");
    assert!((past - 122_171.43).abs() < 0.01);

    let second = import::import(&pool, &data).await.expect("second import");
    assert_eq!(second.mpf.accounts_created, 0);
    assert_eq!(second.mpf.accounts_skipped, 2);
    assert_eq!(count(&pool, "mpf_accounts").await, 2);
    assert_eq!(count(&pool, "mpf_history").await, 2);
}

#[tokio::test]
async fn imported_stock_order_matches_the_summary_sheets() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");
    import::import(&pool, &data).await.expect("import");

    for market in [Market::Hk, Market::Us] {
        let mut expected: Vec<String> = data
            .market(market)
            .stocks
            .iter()
            .map(|stock| stock.code.clone())
            .collect();
        // Stocks that only appear in the 派息 block are appended after the
        // summary-sheet order.
        if market == Market::Hk {
            expected.push("香港寬頻".to_string());
        }
        let actual: Vec<String> = sqlx::query_scalar(
            "SELECT code FROM stocks WHERE market = ? ORDER BY sort_order, code",
        )
        .bind(market.as_str())
        .fetch_all(&pool)
        .await
        .expect("ordered stocks");
        assert_eq!(actual, expected);
    }
}

#[tokio::test]
async fn stock_listed_without_trades_is_created() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");
    import::import(&pool, &data).await.expect("import");

    let trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trades t JOIN stocks s ON s.id = t.stock_id WHERE s.code = ?",
    )
    .bind("ＦＧ恆生紅利")
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(trades, 0);

    let ticker: Option<String> =
        sqlx::query_scalar("SELECT ticker FROM stocks WHERE market = 'HK' AND code = ?")
            .bind("香港中華煤氣")
            .fetch_one(&pool)
            .await
            .expect("ticker");
    assert_eq!(ticker.as_deref(), Some("0003"));
}

#[tokio::test]
async fn duplicate_source_rows_are_kept_once_and_only_once() {
    let pool = db::connect_memory().await.expect("db");

    let duplicate = |source_row: usize| SheetTrade {
        code: "港燈".to_string(),
        shares: 7000.0,
        unit_price: 4.67,
        total: Some(32895.78),
        fee: None,
        trade_date: "2024-04-05".to_string(),
        trade_type: "BUY".to_string(),
        input_mode: InputMode::HkTotal,
        note: None,
        source_row,
    };
    let data = WorkbookData {
        hk: MarketSheets {
            market: Market::Hk,
            trades: vec![duplicate(3), duplicate(4)],
            stocks: vec![SheetStock {
                code: "港燈".to_string(),
                ticker: Some("2638".to_string()),
                exchange: Some("HKG".to_string()),
                sector: Some("Utilities".to_string()),
                sort_order: 1,
            }],
            summary: Vec::new(),
            dividends: Vec::new(),
            cached: Default::default(),
        },
        us: MarketSheets {
            market: Market::Us,
            trades: Vec::new(),
            stocks: Vec::new(),
            summary: Vec::new(),
            dividends: Vec::new(),
            cached: Default::default(),
        },
        deposits: Vec::new(),
        deposit_cached: Default::default(),
        year_figures: Vec::new(),
        year_review: Vec::new(),
        mpf: Vec::new(),
        mpf_cached: Default::default(),
        bonds: Vec::new(),
        bond_cached: Default::default(),
        aia: Vec::new(),
        aia_cached: Default::default(),
        month_stat: Default::default(),
        overview: Default::default(),
        us_account: Default::default(),
    };

    let first = import::import(&pool, &data).await.expect("first import");
    assert_eq!(first.hk.trades_imported, 2);
    assert_eq!(count(&pool, "trades").await, 2);

    let second = import::import(&pool, &data).await.expect("second import");
    assert_eq!(second.hk.trades_imported, 0);
    assert_eq!(second.hk.trades_skipped, 2);
    assert_eq!(count(&pool, "trades").await, 2);
}

#[tokio::test]
async fn parity_is_clean_after_import_and_fails_when_a_trade_changes() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");
    import::import(&pool, &data).await.expect("import");

    let report = parity::check(&pool, &data).await.expect("parity");
    assert!(
        report.is_clean(),
        "unexpected differences: {:?}",
        report.problems().collect::<Vec<_>>()
    );
    // ＦＧ恆生紅利 has no figures on either side and is reported as skipped.
    assert!(
        report
            .rows
            .iter()
            .any(|row| row.code == "ＦＧ恆生紅利" && row.outcome == Outcome::SkippedNoData)
    );

    sqlx::query("UPDATE trades SET shares = shares + 100 WHERE id = 1")
        .execute(&pool)
        .await
        .expect("alter a trade");

    let report = parity::check(&pool, &data).await.expect("parity");
    assert!(!report.is_clean());
    assert!(
        report
            .problems()
            .any(|row| matches!(row.outcome, Outcome::Difference { .. }))
    );
}

#[tokio::test]
async fn parity_flags_a_stock_with_trades_but_no_cached_figures() {
    let pool = db::connect_memory().await.expect("db");
    let mut data = xlsx::read(&workbook_path()).expect("workbook");
    import::import(&pool, &data).await.expect("import");

    // Blank out the cached figures for a stock that does have trades.
    let entry = data
        .hk
        .summary
        .iter_mut()
        .find(|row| row.code == "中國銀行")
        .expect("中國銀行");
    entry.shares_held = None;
    entry.total_buy_cost = None;
    entry.weighted_avg_buy_price = None;

    let report = parity::check(&pool, &data).await.expect("parity");
    assert!(report.problems().any(
        |row| matches!(row.outcome, Outcome::MissingSheetValue { .. }) && row.code == "中國銀行"
    ));
}

#[tokio::test]
async fn reimport_refreshes_the_synthetic_last_month_seed_but_never_a_real_row() {
    use chrono::Datelike;

    let pool = db::connect_memory().await.expect("db");
    let mut data = xlsx::read(&workbook_path()).expect("workbook");
    import::import(&pool, &data).await.expect("first import");

    let today = wealth_backend::routes::today();
    let first_of_month = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let seed_date = (first_of_month - chrono::Days::new(1)).to_string();

    let seeded: Option<(f64, i64)> = sqlx::query_as(
        "SELECT market_value, synthetic FROM market_history \
         WHERE market = 'HK' AND recorded_on = ?",
    )
    .bind(&seed_date)
    .fetch_optional(&pool)
    .await
    .expect("seed row");
    let (value_before, synthetic) = seeded.expect("HK last-month seed");
    assert_eq!(synthetic, 1);

    // A changed cached rate refreshes the synthetic seed on re-import.
    let rate = data.hk.cached.last_month_percent.expect("cached rate");
    data.hk.cached.last_month_percent = Some(rate + 0.01);
    import::import(&pool, &data).await.expect("second import");

    let (cost, refreshed): (f64, f64) = sqlx::query_as(
        "SELECT buy_cost_priced, market_value FROM market_history \
         WHERE market = 'HK' AND recorded_on = ?",
    )
    .bind(&seed_date)
    .fetch_one(&pool)
    .await
    .expect("refreshed row");
    assert_ne!(refreshed, value_before);
    assert!((refreshed - cost * (1.0 + rate + 0.01)).abs() < 0.01);

    // A real record on the seed date is never overwritten.
    sqlx::query(
        "UPDATE market_history SET synthetic = 0, market_value = 12345.0 \
         WHERE market = 'HK' AND recorded_on = ?",
    )
    .bind(&seed_date)
    .execute(&pool)
    .await
    .expect("mark real");
    import::import(&pool, &data).await.expect("third import");

    let value: f64 = sqlx::query_scalar(
        "SELECT market_value FROM market_history \
         WHERE market = 'HK' AND recorded_on = ?",
    )
    .bind(&seed_date)
    .fetch_one(&pool)
    .await
    .expect("row");
    assert_eq!(value, 12345.0);
}

#[tokio::test]
async fn parity_treats_the_seeded_max_marks_as_floors() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");
    import::import(&pool, &data).await.expect("import");

    // A recorded value far above the sheet's max net raises the derived
    // 最高金額 — beating the mark is the expected outcome, not a difference.
    let huge = data.us.cached.max_amount.expect("us max net") * 10.0;
    sqlx::query(
        "INSERT INTO market_history \
         (market, recorded_on, buy_cost_priced, market_value, synthetic) \
         VALUES ('US', '2025-06-30', 1.0, ?, 0)",
    )
    .bind(huge)
    .execute(&pool)
    .await
    .expect("insert history");

    let report = parity::check(&pool, &data).await.expect("parity");
    let row = report
        .market_figures
        .iter()
        .find(|row| row.name == "US 最高金額")
        .expect("US 最高金額 row");
    assert_eq!(row.outcome, Outcome::Match);
    assert!(
        report.is_clean(),
        "unexpected differences: {:?}",
        report.market_figure_problems().collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn parity_flags_a_shortfall_below_the_seeded_max_mark() {
    let pool = db::connect_memory().await.expect("db");
    let data = xlsx::read(&workbook_path()).expect("workbook");
    import::import(&pool, &data).await.expect("import");

    // Unpriced US leaves no derived 最高金額 to compare — a real shortfall
    // against the sheet's mark, unlike exceeding it.
    sqlx::query("UPDATE stocks SET manual_price = NULL WHERE market = 'US'")
        .execute(&pool)
        .await
        .expect("unprice");
    sqlx::query("DELETE FROM market_history WHERE market = 'US'")
        .execute(&pool)
        .await
        .expect("clear history");
    sqlx::query("DELETE FROM app_meta WHERE key = 'market.US.seed_max_amount'")
        .execute(&pool)
        .await
        .expect("clear mark");

    let report = parity::check(&pool, &data).await.expect("parity");
    let row = report
        .market_figures
        .iter()
        .find(|row| row.name == "US 最高金額")
        .expect("US 最高金額 row");
    assert!(matches!(row.outcome, Outcome::Difference { .. }));
}

#[tokio::test]
async fn missing_sheet_or_file_is_reported_clearly() {
    let err = xlsx::read(&PathBuf::from("../does-not-exist.xlsx")).expect_err("must fail");
    assert!(err.to_string().contains("does-not-exist.xlsx"));

    // A workbook without the expected sheets names the missing sheet.
    let path = std::env::temp_dir().join(format!("wealth-empty-{}.xlsx", std::process::id()));
    std::fs::write(&path, b"not a workbook").expect("write file");
    let err = xlsx::read(&path).expect_err("must fail");
    assert!(err.to_string().contains(&path.display().to_string()));
    let _ = std::fs::remove_file(&path);
}
