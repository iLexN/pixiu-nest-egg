//! The 預測 grid read (`Overview!A20:H36`): loads the window's inputs,
//! derives the seven month columns, and buckets each column's lines into its
//! plan-row cells — the whole grid is derived on read; only
//! `forecast_items` is stored.

use serde::Serialize;
use sqlx::SqlitePool;
use utoipa::ToSchema;

use crate::calc;
use crate::store::StoreError;

/// `GET /api/forecast`: seven columns from the current month.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ForecastResponse {
    /// The effective quarterly bill amount (`forecast.bill_amount` or 2158).
    pub bill_amount: f64,
    pub months: Vec<calc::ForecastMonth>,
}

/// The seven-month 預測 grid, fully derived on read. A stored item month
/// that does not parse is `Err(Corrupt)`.
pub async fn build(pool: &SqlitePool) -> Result<ForecastResponse, StoreError> {
    let ctx = super::wealth::LiveContext::open(pool).await?;
    let input = &ctx.input;
    let totals = ctx.totals;
    let stored = &ctx.stored;
    let salary = stored.salary;
    let bill_amount = stored
        .bill_amount
        .unwrap_or(crate::store::months::DEFAULT_BILL_AMOUNT);
    let semi_liquid_target = ctx.semi_liquid_target();
    let first = super::months::live_from();
    // The first column anchors on the current month's 月初(出糧後); while it is
    // unset the live 活期 sum stands in.
    let start = crate::store::months::load_month(pool, &first.to_string())
        .await?
        .and_then(|stored| stored.start_cash)
        .or(Some(input.cash_sum));

    let trailing = ctx.book.trailing_averages();
    let spend = trailing.living_budget.map(|budget| -budget);
    let events = super::months::load_interest_events(pool).await?;
    // HK trades inside the window move 活期 in their month; US trades settle
    // inside IBKR and stay out (the 月結 `trade:` suggestion rule).
    let window_end = first
        .checked_add_months(chrono::Months::new(calc::FORECAST_MONTHS))
        .expect("forecast window overflow")
        .to_string();
    let trades =
        crate::store::trades::hk_suggestion_trades(pool, &first.to_string(), &window_end).await?;
    // Every `ibkr_transfers` row in the window is a 活期 flow in its month —
    // log-only legacy rows moved cash the same way.
    let ibkr_transfers =
        crate::store::ibkr::transfers_in_range(pool, &first.to_string(), &window_end, false)
            .await?;

    let items = crate::store::forecast::load_all_items(pool).await?;
    let facts = items
        .iter()
        .map(crate::store::forecast::facts_of)
        .collect::<Result<Vec<_>, _>>()?;

    let months = calc::forecast_grid(&calc::ForecastInput {
        first_month: first,
        start,
        salary,
        spend,
        liquid_assets: totals.liquid_assets,
        semi_liquid_target,
        locked_base: calc::forecast_locked_anchor(&events.deposits, first),
        bill_amount,
        deposits: &events.deposits,
        events: &events,
        items: &facts,
        trades: &trades,
        ibkr_transfers: &ibkr_transfers,
    });
    Ok(ForecastResponse {
        bill_amount,
        months,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::{ForecastCell, ForecastIbkrTransfer, ForecastMonth};
    use crate::db::connect_memory;
    use crate::models::ForecastItemKind;

    fn stock_cell(month: &ForecastMonth) -> &ForecastCell {
        month
            .cells
            .iter()
            .find(|cell| cell.kind == ForecastItemKind::Stock)
            .expect("stock cell")
    }

    /// An in-window transfer lands in its column's `stock` cell `ibkr_lines`
    /// with the signed −amount; a pre-window one appears nowhere.
    #[tokio::test]
    async fn forecast_lists_the_windows_ibkr_transfers() {
        let pool = connect_memory().await.expect("memory db");
        let first = super::super::months::live_from();
        let in_window = format!("{}-05", &first.to_string()[..7]);
        let pre_window = first
            .checked_sub_months(chrono::Months::new(2))
            .expect("pre-window month")
            .to_string();
        sqlx::query(
            "INSERT INTO ibkr_transfers (transfer_date, amount_hkd, created_at) \
             VALUES (?, 50000, '2026-01-01T00:00:00'), \
                    (?, 90000, '2026-01-01T00:00:00')",
        )
        .bind(&in_window)
        .bind(&pre_window)
        .execute(&pool)
        .await
        .unwrap();

        let response = build(&pool).await.expect("forecast");
        let lines: Vec<&ForecastIbkrTransfer> = response
            .months
            .iter()
            .flat_map(|month| month.cells.iter().flat_map(|cell| cell.ibkr_lines.iter()))
            .collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].transfer_date, in_window);
        assert_eq!(lines[0].amount, -50000.0);
        assert_eq!(stock_cell(&response.months[0]).figure, Some(-50000.0));
    }

    /// A real deposit placed in a window month lands in its bank's cell, and
    /// the empty sibling cell shows no figure.
    #[tokio::test]
    async fn forecast_routes_a_placement_to_its_bank_cell() {
        let pool = connect_memory().await.expect("memory db");
        let first = super::super::months::live_from();
        let start = format!("{}-10", &first.to_string()[..7]);
        let end = first
            .checked_add_months(chrono::Months::new(12))
            .expect("end")
            .to_string();
        sqlx::query(
            "INSERT INTO deposits (label, bank, principal, interest, start_date, end_date, \
             sort_order, created_at, updated_at) VALUES ('HS-1', 'HS', 130000, 900, ?, ?, 1, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00')",
        )
        .bind(&start)
        .bind(&end)
        .execute(&pool)
        .await
        .unwrap();

        let response = build(&pool).await.expect("forecast");
        let cells = &response.months[0].cells;
        let hs = cells
            .iter()
            .find(|cell| cell.kind == ForecastItemKind::HsDeposit)
            .expect("hs cell");
        assert_eq!(hs.placements.len(), 1);
        assert_eq!(hs.figure, Some(-130000.0));
        let sc = cells
            .iter()
            .find(|cell| cell.kind == ForecastItemKind::ScDeposit)
            .expect("sc cell");
        assert!(sc.placements.is_empty());
        assert_eq!(sc.figure, None);
    }
}
