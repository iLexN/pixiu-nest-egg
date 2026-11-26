//! The analysis export read (`GET /api/export/analysis`): the figures the
//! 總覽, 預測 and 股票 pages show, reshaped into the sections an external AI
//! analysis consumes. Derived on read like every service read model —
//! nothing here is stored.

use serde::Serialize;
use sqlx::SqlitePool;
use utoipa::ToSchema;

use crate::models::{ForecastItemKind, Market};
use crate::store::StoreError;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AnalysisExport {
    pub meta: ExportMeta,
    pub liquid_assets_summary: LiquidAssetsSummary,
    pub cash_flow_momentum: CashFlowMomentum,
    pub upcoming_cash_flow_and_rigid_expenses: Vec<ExportMonth>,
    pub equity_holdings: EquityHoldings,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExportMeta {
    /// The overview's today (`YYYY-MM-DD`).
    pub as_of_date: String,
    /// Always `"HKD"` — the sheet's base currency.
    pub currency: String,
    /// `aia.usd_hkd_rate`; absent while unset.
    pub usd_hkd_rate: Option<f64>,
    /// The goal-3 生活保障 target; absent while its inputs are unset.
    pub target_liquid_assets_hkd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LiquidAssetsSummary {
    /// 港股 market value (Overview 港股 row).
    pub hk_stocks_hkd: f64,
    /// The IBKR HKD total (Overview IBKR row).
    pub us_stocks_ibkr_hkd: Option<f64>,
    /// Active bond principal (Overview 債券 row).
    pub bonds_hkd: f64,
    /// 半流動資金 = 已定期 + 活期.
    pub semi_liquid_total_hkd: f64,
    pub semi_liquid_breakdown: SemiLiquidBreakdown,
    pub deductions: Deductions,
    /// H1 流動資產 — identical to the overview's liquid_assets.
    pub total_liquid_assets_hkd: f64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SemiLiquidBreakdown {
    /// 活期 = Σ bank cash accounts.
    pub cash_current_hkd: f64,
    /// 已定期 = Σ principal over active deposits.
    pub time_deposits_hkd: f64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Deductions {
    /// The live 開心Pool balance.
    pub happy_pool_hkd: f64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CashFlowMomentum {
    /// The stored 薪金; absent while unset.
    pub monthly_salary_hkd: Option<f64>,
    /// Trailing 12-month 存 average; absent without history.
    pub avg_monthly_saving_hkd: Option<f64>,
    /// Trailing 12-month 利息 average; absent without history.
    pub avg_monthly_interest_hkd: Option<f64>,
    /// Trailing 12-month 生活支出 average; absent without history.
    pub normal_monthly_living_expense_hkd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExportMonth {
    /// `YYYY-MM`.
    pub month: String,
    /// The 預測 row-23 活期 figure; absent while its chain is broken.
    pub projected_cash_current_hkd: Option<f64>,
    /// The 預測 半流動資金 figure; absent while its chain is broken.
    pub projected_deposits_and_sc_hkd: Option<f64>,
    /// Deposits ending this month (deposit_finish).
    pub maturing_deposits_hkd: f64,
    pub rigid_expenses: Vec<RigidExpense>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RigidExpense {
    /// The plan item's note; `null` when it has none. `差餉` marks the
    /// synthesized quarterly bill.
    pub item: Option<String>,
    /// Signed amount (expenses are negative).
    pub amount_hkd: f64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EquityHoldings {
    pub hk_stocks: Vec<ExportHolding>,
    pub us_stocks: Vec<ExportHolding>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExportHolding {
    pub code: String,
    pub ticker: Option<String>,
    pub exchange: Option<String>,
    pub sector: Option<String>,
    pub shares_held: f64,
    /// Market value in the market's own currency side; `*_hkd` is absent
    /// while a rate is needed and unset.
    pub market_value_hkd: Option<f64>,
    pub market_value_usd: Option<f64>,
    pub total_buy_cost_hkd: Option<f64>,
    pub total_buy_cost_usd: Option<f64>,
    pub dividends_received_hkd: Option<f64>,
    pub dividends_received_usd: Option<f64>,
    /// 累計派息 ÷ 總買入成本 as a percent; absent while cost is 0.
    pub dividend_yield_pct: Option<f64>,
}

/// `GET /api/export/analysis`: the snapshot, fully derived on read.
pub async fn build(pool: &SqlitePool) -> Result<AnalysisExport, StoreError> {
    let overview = super::overview::build(pool).await?;

    let asset_amount = |key: &str| {
        overview
            .assets
            .iter()
            .find(|row| row.key == key)
            .and_then(|row| row.amount)
            .unwrap_or(0.0)
    };

    Ok(AnalysisExport {
        meta: ExportMeta {
            as_of_date: overview.today.clone(),
            currency: "HKD".to_string(),
            usd_hkd_rate: overview.rate,
            target_liquid_assets_hkd: overview.goals.third.target,
        },
        liquid_assets_summary: LiquidAssetsSummary {
            hk_stocks_hkd: asset_amount("hk"),
            us_stocks_ibkr_hkd: overview.ibkr.computed_total_hkd,
            bonds_hkd: asset_amount("bonds"),
            semi_liquid_total_hkd: overview.semi_liquid.total,
            semi_liquid_breakdown: SemiLiquidBreakdown {
                cash_current_hkd: overview.semi_liquid.cash_sum,
                time_deposits_hkd: overview.semi_liquid.deposits,
            },
            deductions: Deductions {
                happy_pool_hkd: overview.averages.pool_balance,
            },
            total_liquid_assets_hkd: overview.liquid_assets,
        },
        cash_flow_momentum: CashFlowMomentum {
            monthly_salary_hkd: overview.salary,
            avg_monthly_saving_hkd: overview.averages.saved,
            avg_monthly_interest_hkd: overview.averages.interest,
            normal_monthly_living_expense_hkd: overview.averages.living_spend,
        },
        upcoming_cash_flow_and_rigid_expenses: export_months(super::forecast::build(pool).await?),
        equity_holdings: EquityHoldings {
            hk_stocks: holdings(pool, Market::Hk, overview.rate).await?,
            us_stocks: holdings(pool, Market::Us, overview.rate).await?,
        },
    })
}

/// Active stocks of one market as export rows. US figures stay in USD and
/// are also converted at `rate` (absent while the rate is unset); HK rows
/// only fill the `_hkd` side.
async fn holdings(
    pool: &SqlitePool,
    market: Market,
    rate: Option<f64>,
) -> Result<Vec<ExportHolding>, StoreError> {
    let (rows, _sectors, _totals) = super::summary::summarize_market(pool, market).await?;
    let is_us = market == Market::Us;
    let to_hkd = |value: Option<f64>| value.zip(rate).map(|(v, r)| v * r);
    let money = |value: Option<f64>| -> (Option<f64>, Option<f64>) {
        if is_us {
            (to_hkd(value), value)
        } else {
            (value, None)
        }
    };
    Ok(rows
        .into_iter()
        .filter(|row| row.stock.is_active)
        .map(|row| {
            let (market_value_hkd, market_value_usd) = money(row.summary.market_value);
            let (total_buy_cost_hkd, total_buy_cost_usd) = money(Some(row.summary.total_buy_cost));
            let (dividends_received_hkd, dividends_received_usd) =
                money(Some(row.summary.dividends_received));
            ExportHolding {
                code: row.stock.code,
                ticker: row.stock.ticker,
                exchange: row.stock.exchange,
                sector: row.stock.sector,
                shares_held: row.summary.shares_held,
                market_value_hkd,
                market_value_usd,
                total_buy_cost_hkd,
                total_buy_cost_usd,
                dividends_received_hkd,
                dividends_received_usd,
                dividend_yield_pct: row.summary.dividend_return.map(|r| r * 100.0),
            }
        })
        .collect())
}

/// One 預測 column per window month. Rigid expenses are the month's Tax,
/// Bill and Other plan items as `{item, amount_hkd}`; a quarter-month `bill`
/// that came from the recurring-bill default (not a stored Bill item) is
/// synthesized as 差餉 — stored Bill items replace the default rather than
/// stack on it.
fn export_months(forecast: super::forecast::ForecastResponse) -> Vec<ExportMonth> {
    forecast
        .months
        .iter()
        .map(|month| {
            let mut rigid_expenses: Vec<RigidExpense> = month
                .cells
                .iter()
                .filter(|cell| {
                    matches!(
                        cell.kind,
                        ForecastItemKind::Tax | ForecastItemKind::Bill | ForecastItemKind::Other
                    )
                })
                .flat_map(|cell| cell.plan_items.iter())
                .map(|item| RigidExpense {
                    item: item.note.clone(),
                    amount_hkd: item.amount,
                })
                .collect();
            let bill_from_items = month
                .cells
                .iter()
                .any(|cell| cell.kind == ForecastItemKind::Bill && !cell.plan_items.is_empty());
            if !bill_from_items && let Some(bill) = month.bill {
                rigid_expenses.push(RigidExpense {
                    item: Some("差餉".to_string()),
                    amount_hkd: bill,
                });
            }
            ExportMonth {
                month: month_label(&month.month),
                projected_cash_current_hkd: month.cash,
                projected_deposits_and_sc_hkd: month.semi_liquid,
                maturing_deposits_hkd: month.deposit_finish,
                rigid_expenses,
            }
        })
        .collect()
}

/// A forecast month's `YYYY-MM` label. The grid emits full `YYYY-MM-DD`
/// dates (day 01); a string that does not parse passes through whole rather
/// than being silently truncated.
fn month_label(month: &str) -> String {
    chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d")
        .map(|date| date.format("%Y-%m").to_string())
        .unwrap_or_else(|_| month.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn liquid_summary_matches_the_overview_figures() {
        let pool = connect_memory().await.expect("memory db");
        sqlx::query(
            "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
             sort_order, updated_at) VALUES ('HS', 'HKD', 'bank', 10000.0, 1, 1, \
             '2026-01-01T00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO deposits (label, bank, principal, interest, start_date, end_date, \
             sort_order, created_at, updated_at) VALUES ('TD-1', 'HS', 50000, 900, \
             '2025-01-01', '2030-01-01', 1, '2026-01-01T00:00:00', '2026-01-01T00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let export = build(&pool).await.expect("export");
        assert!(chrono::NaiveDate::parse_from_str(&export.meta.as_of_date, "%Y-%m-%d").is_ok());
        assert_eq!(export.meta.currency, "HKD");
        // Nothing configured yet — both optional meta fields are null, not absent.
        assert_eq!(export.meta.usd_hkd_rate, None);
        assert_eq!(export.meta.target_liquid_assets_hkd, None);
        let summary = &export.liquid_assets_summary;
        assert_eq!(summary.semi_liquid_breakdown.cash_current_hkd, 10000.0);
        assert_eq!(summary.semi_liquid_breakdown.time_deposits_hkd, 50000.0);
        assert_eq!(summary.semi_liquid_total_hkd, 60000.0);
        assert_eq!(summary.total_liquid_assets_hkd, 60000.0);
    }

    #[tokio::test]
    async fn rigid_expenses_list_tax_bill_other_items_only() {
        let pool = connect_memory().await.expect("memory db");
        let first = crate::service::months::live_from().to_string();
        insert_plan_item(&pool, &first, "tax", -100.0, Some("AIA 醫療".to_string())).await;
        insert_plan_item(&pool, &first, "other", -50.0, None).await;
        insert_plan_item(&pool, &first, "sc_deposit", -20000.0, None).await;
        insert_plan_item(
            &pool,
            &first,
            "interest",
            300.0,
            Some("bonus interest".to_string()),
        )
        .await;
        insert_plan_item(
            &pool,
            &first,
            "stock",
            -5000.0,
            Some("planned buy".to_string()),
        )
        .await;

        let export = build(&pool).await.expect("export");
        assert_eq!(export.upcoming_cash_flow_and_rigid_expenses.len(), 7);
        let month = &export.upcoming_cash_flow_and_rigid_expenses[0];
        assert_eq!(month.month.len(), 7);
        let items = &month.rigid_expenses;
        assert!(
            items
                .iter()
                .any(|e| e.item.as_deref() == Some("AIA 醫療") && e.amount_hkd == -100.0)
        );
        assert!(
            items
                .iter()
                .any(|e| e.item.is_none() && e.amount_hkd == -50.0)
        );
        assert!(
            !items.iter().any(|e| e.amount_hkd == -20000.0
                || e.amount_hkd == 300.0
                || e.amount_hkd == -5000.0)
        );
    }

    #[tokio::test]
    async fn a_quarter_month_synthesizes_the_bill_unless_items_replace_it() {
        let pool = connect_memory().await.expect("memory db");
        let export = build(&pool).await.expect("export");
        let quarter = export
            .upcoming_cash_flow_and_rigid_expenses
            .iter()
            .find(|m| matches!(&m.month[5..7], "01" | "04" | "07" | "10"))
            .expect("a quarter month in the window");
        let bill = quarter
            .rigid_expenses
            .iter()
            .find(|e| e.item.as_deref() == Some("差餉"))
            .expect("synthesized 差餉");
        assert_eq!(bill.amount_hkd, -crate::store::months::DEFAULT_BILL_AMOUNT);

        // A stored Bill item in a quarter month replaces the default — the
        // synthesized 差餉 must not double it up.
        let pool = connect_memory().await.expect("memory db");
        let first = crate::service::months::live_from();
        let quarter_month = (0..crate::calc::FORECAST_MONTHS)
            .filter_map(|offset| {
                first
                    .checked_add_months(chrono::Months::new(offset))
                    .map(|date| date.to_string())
            })
            .find(|date| matches!(&date[5..7], "01" | "04" | "07" | "10"))
            .expect("quarter month");
        crate::store::forecast::insert_item(
            &pool,
            &quarter_month,
            "bill",
            -2223.0,
            &None,
            &Some("rates".to_string()),
            1,
        )
        .await
        .unwrap();
        let export = build(&pool).await.expect("export");
        let month = export
            .upcoming_cash_flow_and_rigid_expenses
            .iter()
            .find(|m| quarter_month.starts_with(&m.month))
            .expect("the seeded quarter month");
        assert!(
            month
                .rigid_expenses
                .iter()
                .any(|e| e.item.as_deref() == Some("rates"))
        );
        assert!(
            !month
                .rigid_expenses
                .iter()
                .any(|e| e.item.as_deref() == Some("差餉"))
        );
    }

    async fn add_stock(
        pool: &SqlitePool,
        market: &str,
        code: &str,
        price: Option<f64>,
        active: i64,
    ) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO stocks (market, code, manual_price, is_active, sort_order) \
             VALUES (?, ?, ?, ?, 1) RETURNING id",
        )
        .bind(market)
        .bind(code)
        .bind(price)
        .bind(active)
        .fetch_one(pool)
        .await
        .expect("insert stock")
    }

    async fn insert_plan_item(
        pool: &SqlitePool,
        month: &str,
        kind: &str,
        amount: f64,
        note: Option<String>,
    ) {
        crate::store::forecast::insert_item(pool, month, kind, amount, &None, &note, 1)
            .await
            .unwrap();
    }

    async fn add_buy(pool: &SqlitePool, stock_id: i64, shares: f64, total: f64) {
        sqlx::query(
            "INSERT INTO trades (stock_id, trade_type, trade_date, shares, unit_price, fee, \
             total, input_mode, created_at, updated_at) \
             VALUES (?, 'BUY', '2026-01-01', ?, ?, 0.0, ?, 'HK_TOTAL', 'x', 'x')",
        )
        .bind(stock_id)
        .bind(shares)
        .bind(total / shares)
        .bind(total)
        .execute(pool)
        .await
        .expect("insert trade");
    }

    #[tokio::test]
    async fn equity_holdings_list_active_stocks_with_yields() {
        let pool = connect_memory().await.expect("memory db");
        sqlx::query(
            "INSERT INTO fx_rates (currency, rate_to_hkd, updated_at) VALUES ('USD', 8.0, 'x')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let hk = add_stock(&pool, "HK", "03988", Some(12.0), 1).await;
        add_buy(&pool, hk, 1000.0, 10030.0).await;
        sqlx::query(
            "INSERT INTO dividends (stock_id, pay_date, received_amount, created_at, \
             updated_at) VALUES (?, '2026-08-01', 500.0, 'x', 'x')",
        )
        .bind(hk)
        .execute(&pool)
        .await
        .unwrap();

        let inactive = add_stock(&pool, "HK", "99999", None, 0).await;
        add_buy(&pool, inactive, 100.0, 1000.0).await;

        let us = add_stock(&pool, "US", "VOO", Some(600.0), 1).await;
        add_buy(&pool, us, 10.0, 5000.0).await;

        // A stock with no recorded buy cost yields `null`.
        add_stock(&pool, "US", "GOOG", None, 1).await;

        let export = build(&pool).await.expect("export");

        let hk_row = export
            .equity_holdings
            .hk_stocks
            .iter()
            .find(|h| h.code == "03988")
            .expect("hk row");
        assert_eq!(hk_row.shares_held, 1000.0);
        assert_eq!(hk_row.market_value_hkd, Some(12000.0));
        assert_eq!(hk_row.total_buy_cost_hkd, Some(10030.0));
        assert_eq!(hk_row.dividends_received_hkd, Some(500.0));
        assert_eq!(hk_row.dividend_yield_pct, Some(500.0 / 10030.0 * 100.0));
        assert_eq!(hk_row.market_value_usd, None);

        assert!(
            !export
                .equity_holdings
                .hk_stocks
                .iter()
                .any(|h| h.code == "99999")
        );

        let us_row = export
            .equity_holdings
            .us_stocks
            .iter()
            .find(|h| h.code == "VOO")
            .expect("us row");
        assert_eq!(us_row.market_value_usd, Some(6000.0));
        assert_eq!(us_row.market_value_hkd, Some(48000.0));
        assert_eq!(us_row.total_buy_cost_usd, Some(5000.0));

        let goog = export
            .equity_holdings
            .us_stocks
            .iter()
            .find(|h| h.code == "GOOG")
            .expect("goog row");
        assert_eq!(goog.dividend_yield_pct, None);
    }

    #[tokio::test]
    async fn momentum_carries_the_salary_and_leaves_absent_averages_null() {
        let pool = connect_memory().await.expect("memory db");
        let mut conn = pool.acquire().await.unwrap();
        crate::store::settings::SettingsPatch {
            salary: Some(Some(52700.0)),
            ..Default::default()
        }
        .apply(&mut conn)
        .await
        .unwrap();
        drop(conn);

        let export = build(&pool).await.expect("export");
        let momentum = &export.cash_flow_momentum;
        assert_eq!(momentum.monthly_salary_hkd, Some(52700.0));
        assert_eq!(momentum.avg_monthly_saving_hkd, None);
        assert_eq!(momentum.avg_monthly_interest_hkd, None);
        assert_eq!(momentum.normal_monthly_living_expense_hkd, None);
    }
}
