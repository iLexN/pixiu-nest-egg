//! Compares the app's computed summary against the workbook's cached values.

use anyhow::anyhow;
use sqlx::SqlitePool;

use crate::calc::approx_eq;
use crate::models::Market;
use crate::xlsx::WorkbookData;

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Match,
    /// The computed and cached figures disagree beyond tolerance.
    Difference {
        field: &'static str,
        computed: f64,
        sheet: f64,
    },
    /// The sheet holds no cached value for a stock that has trades.
    MissingSheetValue {
        field: &'static str,
    },
    /// The sheet lists the stock but neither side has figures to compare.
    SkippedNoData,
    /// The sheet lists a stock the database does not know about.
    MissingStock,
}

#[derive(Debug, Clone)]
pub struct ParityRow {
    pub market: Market,
    pub code: String,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Default)]
pub struct ParityReport {
    pub rows: Vec<ParityRow>,
}

impl ParityReport {
    pub fn problems(&self) -> impl Iterator<Item = &ParityRow> {
        self.rows
            .iter()
            .filter(|row| !matches!(row.outcome, Outcome::Match | Outcome::SkippedNoData))
    }

    pub fn is_clean(&self) -> bool {
        self.problems().next().is_none()
    }
}

pub async fn check(pool: &SqlitePool, data: &WorkbookData) -> anyhow::Result<ParityReport> {
    let mut report = ParityReport::default();
    for market in [Market::Hk, Market::Us] {
        check_market(pool, data, market, &mut report).await?;
    }
    Ok(report)
}

async fn check_market(
    pool: &SqlitePool,
    data: &WorkbookData,
    market: Market,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    let summary = crate::routes::summary::build(pool, market)
        .await
        .map_err(|err| anyhow!("computing the {} summary failed: {err}", market.as_str()))?;

    for sheet in &data.market(market).summary {
        let Some(computed) = summary
            .stocks
            .iter()
            .find(|row| row.stock.code == sheet.code)
        else {
            report.rows.push(ParityRow {
                market,
                code: sheet.code.clone(),
                outcome: Outcome::MissingStock,
            });
            continue;
        };

        let has_trades = computed.trade_count > 0;
        let comparisons: [(&'static str, Option<f64>, f64); 3] = [
            ("股數", sheet.shares_held, computed.summary.shares_held),
            (
                "總買入成本",
                sheet.total_buy_cost,
                computed.summary.total_buy_cost,
            ),
            (
                "加權平均買入單價",
                sheet.weighted_avg_buy_price,
                computed.summary.weighted_avg_buy_price,
            ),
        ];

        let mut outcome = Outcome::Match;
        let mut sheet_values = 0;
        for (field, sheet_value, computed_value) in comparisons {
            match sheet_value {
                None if has_trades => {
                    outcome = Outcome::MissingSheetValue { field };
                    break;
                }
                None => continue,
                Some(sheet_value) => {
                    sheet_values += 1;
                    if !approx_eq(computed_value, sheet_value) {
                        outcome = Outcome::Difference {
                            field,
                            computed: computed_value,
                            sheet: sheet_value,
                        };
                        break;
                    }
                }
            }
        }
        if outcome == Outcome::Match && sheet_values == 0 && !has_trades {
            outcome = Outcome::SkippedNoData;
        }

        report.rows.push(ParityRow {
            market,
            code: sheet.code.clone(),
            outcome,
        });
    }

    Ok(())
}
