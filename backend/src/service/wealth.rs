//! The 總資產/流動資產 aggregate (`Overview!B1`/`H1`): what every holding is
//! worth right now, in one input struct for [`crate::calc::live_totals`].

use chrono::Datelike;
use sqlx::SqlitePool;

use crate::calc::{LiveTotals, LiveTotalsInput, active_totals, aia_totals, mpf_totals};
use crate::store::{StoreError, today};

/// The components of `Overview!B1`/`H1`, resolved from the current DB — the
/// same aggregation the later Overview endpoint reuses.
pub async fn totals_input(pool: &SqlitePool) -> Result<LiveTotalsInput, StoreError> {
    let today = today();
    let hk = super::summary::market_value_only(pool, crate::models::Market::Hk)
        .await?
        .unwrap_or(0.0);
    let us = super::summary::market_value_only(pool, crate::models::Market::Us)
        .await?
        .unwrap_or(0.0);
    let rate = crate::store::fx::usd_rate(pool).await?;

    let facts = crate::store::deposits::active_facts(pool).await?;
    // 定期!B1 counts principal only; the interest lands in 活期 at maturity.
    let deposits_active_principal = active_totals(&facts).principal;

    let bonds_active_principal = super::bonds::active_principal(pool).await?;

    let policies = crate::store::aia::load_all_policies(pool).await?;
    let aia_facts: Vec<crate::calc::AiaPolicyFacts> =
        policies.iter().map(crate::store::aia::facts_of).collect();
    let aia_value_usd = aia_totals(&aia_facts, rate).value;

    let accounts = crate::store::mpf::load_all_stored(pool).await?;
    let history = crate::store::mpf::load_history(pool, None).await?;
    let mpf_facts: Vec<crate::calc::MpfAccountFacts> = accounts
        .iter()
        .map(|account| crate::store::mpf::facts_of(account, &history))
        .collect();
    let mpf_balance = mpf_totals(&mpf_facts, today, None, None).now;

    let manual_assets_sum = crate::store::months::manual_asset_sum(pool).await?;
    let cash_sum = crate::store::cash_accounts::bank_sum(pool).await?;

    // The 開心Pool chain reads stored figures only — no live fill (which is
    // what this very function is computing).
    let (stat_rows, items) = super::months::stat_rows_no_live(pool).await?;
    let rates = crate::store::months::pool_rates(pool).await?;
    let pool_balance = match stat_rows.last() {
        Some(latest) => crate::calc::pool_balances(&stat_rows, &items, &rates)
            .get(&latest.month.year())
            .copied()
            .unwrap_or(0.0),
        None => 0.0,
    };

    Ok(LiveTotalsInput {
        hk_market_value: hk,
        us_market_value: us,
        usd_hkd_rate: rate,
        deposits_active_principal,
        bonds_active_principal,
        aia_value_usd,
        mpf_balance,
        manual_assets_sum,
        cash_sum,
        ibkr_hkd_cash: crate::store::cash_accounts::broker(pool, crate::models::Currency::Hkd)
            .await?
            .map(|account| account.amount)
            .unwrap_or(0.0),
        ibkr_usd_cash: crate::store::cash_accounts::broker(pool, crate::models::Currency::Usd)
            .await?
            .map(|account| account.amount)
            .unwrap_or(0.0),
        pool_balance,
    })
}

/// Compute the live totals once; callers check their own gate first.
pub async fn live_totals(pool: &SqlitePool) -> Result<LiveTotals, StoreError> {
    let input = totals_input(pool).await?;
    Ok(crate::calc::live_totals(&input))
}
