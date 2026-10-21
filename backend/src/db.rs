use std::path::Path;
use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub const DEFAULT_DB_PATH: &str = "data/wealth.db";

/// Open (creating if needed) the SQLite database at `path` and run migrations.
pub async fn connect(path: &str) -> anyhow::Result<SqlitePool> {
    if let Some(parent) = Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// An isolated in-memory database with migrations applied, for tests.
pub async fn connect_memory() -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// Applying migrations twice is a no-op: sqlx records applied versions.
pub async fn migrate(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn table_names(pool: &SqlitePool) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
        )
        .fetch_all(pool)
        .await
        .expect("read schema")
    }

    #[tokio::test]
    async fn migrations_create_both_tables_and_rerun_cleanly() {
        let dir = std::env::temp_dir().join(format!("wealth-db-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("wealth.db");
        let path_str = path.to_str().expect("utf-8 path");

        let pool = connect(path_str).await.expect("first run");
        let tables = table_names(&pool).await;
        assert!(tables.contains(&"stocks".to_string()));
        assert!(tables.contains(&"trades".to_string()));
        assert!(tables.contains(&"deposits".to_string()));
        assert!(path.exists());

        // Second run against the same file must not fail or change the schema.
        migrate(&pool).await.expect("re-running migrations");
        let pool2 = connect(path_str).await.expect("second run");
        assert_eq!(table_names(&pool2).await, tables);

        pool.close().await;
        pool2.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn in_memory_database_has_schema() {
        let pool = connect_memory().await.expect("memory db");
        let tables = table_names(&pool).await;
        assert!(tables.contains(&"stocks".to_string()));
        assert!(tables.contains(&"trades".to_string()));
        assert!(tables.contains(&"deposits".to_string()));
        assert!(tables.contains(&"market_history".to_string()));
    }

    /// An in-memory database with no bookkeeping table, so migration steps can
    /// be run one by one around fixture inserts.
    async fn unmigrated_memory() -> SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("memory db")
    }

    async fn apply_versions(pool: &SqlitePool, upto: i64, from: i64) {
        let migrator = sqlx::migrate!("./migrations");
        for migration in migrator
            .migrations
            .iter()
            .filter(|migration| migration.version > from && migration.version <= upto)
        {
            sqlx::raw_sql(migration.sql.clone())
                .execute(pool)
                .await
                .unwrap_or_else(|err| panic!("migration {} failed: {err}", migration.version));
        }
    }

    /// The 0027-state fixtures shaped like the live database: asset id 1,
    /// cash ids 2 `HS` and 3 `渣打`, the IBKR meta keys, the renamed settings
    /// keys, pool-rate keys, plus receipt rows with credited amounts.
    async fn seed_0027_fixtures(pool: &SqlitePool) {
        sqlx::raw_sql(
            "INSERT INTO manual_assets (id, label, kind, liquidity, amount, sort_order, updated_at) VALUES \
             (1, 'HS 人壽', 'asset', 'long', 69681.06, 1, '2026-01-01T00:00:00'), \
             (2, 'HS', 'cash', 'long', 26096.52, 2, '2026-01-01T00:00:00'), \
             (3, '渣打', 'cash', 'long', 1549.01, 3, '2026-01-01T00:00:00');
             INSERT INTO app_meta (key, value) VALUES \
             ('ibkr.hkd_cash', '765.419'), ('ibkr.usd_cash', '20.019'), \
             ('aia.usd_hkd_rate', '7.843045'), ('ibkr.now_value', '133238.04'), \
             ('overview.salary', '52700'), ('overview.semi_liquid_target', '0.25'), \
             ('overview.goal1_amount', '1550000'), ('overview.goal2_amount', '2500000'), \
             ('overview.goal3_inflation', '1.07'), ('overview.goal3_years', '30'), \
             ('overview.final_replacement', '0.7'), \
             ('overview.pool_rate.2024', '0.53'), \
             ('overview.pool_rate.2025', '0.42500000000000004'), \
             ('overview.pool_rate.2026', '0.337'), \
             ('aia.note', 'keep me');
             INSERT INTO deposits (label, bank, principal, rate, interest, start_date, end_date, \
             received_at, credited_asset_id, credited_amount, sort_order, created_at, updated_at) \
             VALUES ('定期A', 'HS', 100000, 0.03, 1000, '2026-01-01', '2026-10-01', \
             '2026-10-01', 3, 101000, 1, '2026-01-01T00:00:00', '2026-01-01T00:00:00');
             INSERT INTO stocks (code, market, sort_order) VALUES \
             ('0005', 'HK', 1), ('AAPL', 'US', 2);
             INSERT INTO dividends (stock_id, pay_date, received_amount, credited_amount, created_at, updated_at) VALUES \
             (1, '2026-03-01', 720.0, 720.0, '2026-01-01T00:00:00', '2026-01-01T00:00:00'), \
             (2, '2026-03-02', 50.0, 50.0, '2026-01-01T00:00:00', '2026-01-01T00:00:00'), \
             (1, '2026-06-01', NULL, NULL, '2026-01-01T00:00:00', '2026-01-01T00:00:00');
             INSERT INTO bonds (label, issue_no, principal, maturity_date, sort_order, created_at, updated_at) VALUES \
             ('債券A', 'X1', 50000, '2027-01-01', 1, '2026-01-01T00:00:00', '2026-01-01T00:00:00');
             INSERT INTO bond_coupons (bond_id, pay_date, received_amount, credited_amount, created_at, updated_at) VALUES \
             (1, '2026-07-01', 875.0, 875.0, '2026-01-01T00:00:00', '2026-01-01T00:00:00'), \
             (1, '2027-01-01', NULL, NULL, '2026-01-01T00:00:00', '2026-01-01T00:00:00');",
        )
        .execute(pool)
        .await
        .expect("0027 fixtures");
    }

    async fn migrate_28_plus(pool: &SqlitePool) {
        apply_versions(pool, i64::MAX, 27).await;
    }

    #[tokio::test]
    async fn migration_0028_moves_the_usd_rate() {
        let pool = unmigrated_memory().await;
        apply_versions(&pool, 27, 0).await;
        seed_0027_fixtures(&pool).await;
        migrate_28_plus(&pool).await;

        let rate: f64 =
            sqlx::query_scalar("SELECT rate_to_hkd FROM fx_rates WHERE currency = 'USD'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(rate, 7.843045);
        let key: Option<String> =
            sqlx::query_scalar("SELECT value FROM app_meta WHERE key = 'aia.usd_hkd_rate'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(key.is_none());
    }

    #[tokio::test]
    async fn migration_0028_tolerates_a_missing_rate() {
        let pool = unmigrated_memory().await;
        apply_versions(&pool, i64::MAX, 0).await;
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM fx_rates")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0);
    }

    #[tokio::test]
    async fn migration_0029_moves_cash_into_cash_accounts() {
        let pool = unmigrated_memory().await;
        apply_versions(&pool, 27, 0).await;
        seed_0027_fixtures(&pool).await;
        migrate_28_plus(&pool).await;

        let rows: Vec<(i64, String, String, String, f64, i64)> = sqlx::query_as(
            "SELECT id, label, currency, role, amount, is_default_receipt \
             FROM cash_accounts ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            rows,
            vec![
                (2, "HS".into(), "HKD".into(), "bank".into(), 26096.52, 1),
                (3, "渣打".into(), "HKD".into(), "bank".into(), 1549.01, 0),
                (
                    4,
                    "IBKR HKD".into(),
                    "HKD".into(),
                    "broker".into(),
                    765.419,
                    0
                ),
                (
                    5,
                    "IBKR USD".into(),
                    "USD".into(),
                    "broker".into(),
                    20.019,
                    1
                ),
            ]
        );

        let keys: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM app_meta WHERE key IN ('ibkr.hkd_cash','ibkr.usd_cash')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(keys, 0);

        let assets: Vec<(i64, String)> =
            sqlx::query_as("SELECT id, label FROM manual_assets ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(assets, vec![(1, "HS 人壽".to_string())]);
        let has_kind: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('manual_assets') WHERE name = 'kind')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!has_kind);

        // New asset ids continue past the cash ids — never reused.
        let next_id: i64 = sqlx::query_scalar(
            "INSERT INTO manual_assets (label, liquidity, amount, sort_order, updated_at) \
             VALUES ('new asset', 'long', 0, 9, '2026-01-01T00:00:00') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(next_id > 3);
    }

    #[tokio::test]
    async fn migration_0029_fresh_db_still_gets_broker_rows() {
        let pool = unmigrated_memory().await;
        apply_versions(&pool, i64::MAX, 0).await;
        let rows: Vec<(String, String, f64, i64)> = sqlx::query_as(
            "SELECT label, currency, amount, is_default_receipt FROM cash_accounts \
             WHERE role = 'broker' ORDER BY currency",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            rows,
            vec![
                ("IBKR HKD".into(), "HKD".into(), 0.0, 0),
                ("IBKR USD".into(), "USD".into(), 0.0, 1),
            ]
        );
    }

    #[tokio::test]
    async fn migration_0030_backfills_credited_accounts() {
        let pool = unmigrated_memory().await;
        apply_versions(&pool, 27, 0).await;
        seed_0027_fixtures(&pool).await;
        migrate_28_plus(&pool).await;

        let deposit: Option<i64> =
            sqlx::query_scalar("SELECT credited_account_id FROM deposits WHERE label = '定期A'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(deposit, Some(3));

        let hs: i64 =
            sqlx::query_scalar("SELECT id FROM cash_accounts WHERE label = 'HS' AND role = 'bank'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let ibkr_usd: i64 = sqlx::query_scalar(
            "SELECT id FROM cash_accounts WHERE role = 'broker' AND currency = 'USD'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let credited: Vec<(i64, Option<i64>)> = sqlx::query_as(
            "SELECT d.id, d.credited_account_id FROM dividends d JOIN stocks s ON s.id = d.stock_id \
             ORDER BY d.id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            credited,
            vec![(1, Some(hs)), (2, Some(ibkr_usd)), (3, None)]
        );

        let coupons: Vec<(i64, Option<i64>)> =
            sqlx::query_as("SELECT id, credited_account_id FROM bond_coupons ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(coupons, vec![(1, Some(hs)), (2, None)]);
    }

    #[tokio::test]
    async fn migration_0031_renames_keys_and_moves_pool_rates() {
        let pool = unmigrated_memory().await;
        apply_versions(&pool, 27, 0).await;
        seed_0027_fixtures(&pool).await;
        migrate_28_plus(&pool).await;

        let renamed: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM app_meta WHERE key IN (\
             'ledger.salary','strategy.semi_liquid_target','goals.goal1_amount',\
             'goals.goal2_amount','goals.goal3_inflation','goals.goal3_years',\
             'goals.final_replacement','ibkr.reported_value') ORDER BY key",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            renamed,
            vec![
                ("goals.final_replacement".into(), "0.7".into()),
                ("goals.goal1_amount".into(), "1550000".into()),
                ("goals.goal2_amount".into(), "2500000".into()),
                ("goals.goal3_inflation".into(), "1.07".into()),
                ("goals.goal3_years".into(), "30".into()),
                ("ibkr.reported_value".into(), "133238.04".into()),
                ("ledger.salary".into(), "52700".into()),
                ("strategy.semi_liquid_target".into(), "0.25".into()),
            ]
        );

        let stale: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM app_meta WHERE key LIKE 'overview.%' OR key = 'ibkr.now_value'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stale, 0);
        let untouched: Option<String> =
            sqlx::query_scalar("SELECT value FROM app_meta WHERE key = 'aia.note'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(untouched.as_deref(), Some("keep me"));

        let rates: Vec<(i32, f64)> =
            sqlx::query_as("SELECT year, rate FROM pool_rates ORDER BY year")
                .fetch_all(&pool)
                .await
                .unwrap();
        // REAL storage preserves the text value's f64 exactly.
        assert_eq!(
            rates,
            vec![(2024, 0.53), (2025, 0.42500000000000004), (2026, 0.337)]
        );
        assert_eq!(
            rates[1].1.to_string(),
            "0.42500000000000004",
            "the parsed rate must round-trip exactly"
        );
    }

    /// `query` selects the single rewritten timestamp; `original` is the
    /// legacy value inserted before migration 0032. The rewrite must land on
    /// the app's `±HH:MM` shape and keep the instant, checked through
    /// julianday so the test is zone-independent.
    async fn assert_ts_normalized(pool: &SqlitePool, query: &'static str, original: &str) {
        let converted: String = sqlx::query_scalar(query).fetch_one(pool).await.unwrap();
        let converted_jd: f64 = sqlx::query_scalar("SELECT julianday(?)")
            .bind(&converted)
            .fetch_one(pool)
            .await
            .unwrap();
        let original_jd: f64 = sqlx::query_scalar("SELECT julianday(?)")
            .bind(original)
            .fetch_one(pool)
            .await
            .unwrap();
        assert!(
            converted.len() == 32
                && matches!(converted.as_bytes()[26], b'+' | b'-')
                && converted.as_bytes()[29] == b':'
                && (converted_jd - original_jd).abs() < 0.00001,
            "{query} -> {converted}"
        );
    }

    #[tokio::test]
    async fn migration_0032_normalizes_legacy_timestamps() {
        let pool = unmigrated_memory().await;
        apply_versions(&pool, 31, 0).await;
        sqlx::raw_sql(
            "INSERT INTO month_stats (month, created_at, updated_at) VALUES \
             ('2024-01-01', '2026-01-01T00:00:00.000000+08:00', '2026-01-01T00:00:00.000000+08:00');
             INSERT INTO month_items (month, category, label, amount, created_at) VALUES \
             ('2024-01-01', 'income', 'legacy', 1, '2026-09-21T07:11:22.897Z'), \
             ('2024-01-01', 'income', 'conforming', 1, '2026-10-20T15:10:11.946813+08:00');
             INSERT INTO fx_rates (currency, rate_to_hkd, updated_at) VALUES \
             ('USD', 7.843045, '2026-10-20T07:01:06.624Z');
             INSERT INTO ibkr_transfers (transfer_date, amount_hkd, created_at) VALUES \
             ('2026-06-02', 131000, '2026-09-23 09:10:36');",
        )
        .execute(&pool)
        .await
        .expect("legacy timestamp fixtures");
        apply_versions(&pool, 32, 31).await;

        assert_ts_normalized(
            &pool,
            "SELECT created_at FROM month_items WHERE label = 'legacy'",
            "2026-09-21T07:11:22.897Z",
        )
        .await;
        assert_ts_normalized(
            &pool,
            "SELECT updated_at FROM fx_rates WHERE currency = 'USD'",
            "2026-10-20T07:01:06.624Z",
        )
        .await;
        assert_ts_normalized(
            &pool,
            "SELECT created_at FROM ibkr_transfers",
            "2026-09-23 09:10:36",
        )
        .await;

        // 0029 stamps the seeded broker rows UTC `Z`; 0032 rewrites them on
        // the same pass, whatever instant 'now' was.
        let odd: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cash_accounts \
             WHERE updated_at NOT GLOB '????-??-??T??:??:??.??????[+-][0-9][0-9]:[0-9][0-9]'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(odd, 0);

        // Rows already in the app format are untouched.
        let kept: String =
            sqlx::query_scalar("SELECT created_at FROM month_items WHERE label = 'conforming'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kept, "2026-10-20T15:10:11.946813+08:00");
    }

    #[tokio::test]
    async fn migration_0032_leaves_fresh_db_timestamps_uniform() {
        let pool = unmigrated_memory().await;
        apply_versions(&pool, i64::MAX, 0).await;
        // 0029 stamps the seeded broker rows UTC `Z`; 0032 must rewrite them.
        let odd: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM cash_accounts \
             WHERE updated_at NOT GLOB '????-??-??T??:??:??.??????[+-][0-9][0-9]:[0-9][0-9]'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(odd, 0);
    }
}
