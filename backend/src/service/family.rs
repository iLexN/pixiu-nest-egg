//! Family deposits (家人 定期) read side: the per-holder summary. Writes stay
//! store-direct (`store::family`) per the two-tier seam — the domain has no
//! cross-table workflows to compose.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{DepositFacts, active_totals, history_years};
use crate::models::FamilyDeposit;
use crate::store::{StoreError, family, settings, today};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FamilyHolderSummary {
    pub holder: String,
    pub note: Option<String>,
    /// Unreceived deposits, earliest maturity first.
    pub upcoming: Vec<FamilyDeposit>,
    /// Σ principal over this holder's unreceived deposits.
    pub active_principal: f64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FamilyDepositSummary {
    /// The date `status`/`active_principal` were derived against.
    pub today: String,
    /// Every holder with deposits or a stored note, sorted by name.
    pub holders: Vec<FamilyHolderSummary>,
    /// Distinct end years in the data plus the current year, for the filter.
    pub history_years: Vec<i32>,
}

/// The 家人 → 定期 page read model — every figure derived from the stored
/// family deposits on each read.
pub async fn summary(pool: &SqlitePool) -> Result<FamilyDepositSummary, StoreError> {
    let today = today();
    let deposits = family::load_all(pool).await?;

    // Holders = distinct deposit holders ∪ holders with a stored note.
    let mut holders: Vec<String> = deposits
        .iter()
        .map(|deposit| deposit.holder.clone())
        .collect();
    holders.extend(settings::family_note_holders(pool).await?);
    holders.sort();
    holders.dedup();

    let mut summaries = Vec::with_capacity(holders.len());
    for holder in holders {
        let rows: Vec<&FamilyDeposit> = deposits
            .iter()
            .filter(|deposit| deposit.holder == holder)
            .collect();
        let mut upcoming: Vec<FamilyDeposit> = rows
            .iter()
            .filter(|deposit| deposit.received_at.is_none())
            .map(|deposit| (*deposit).clone())
            .collect();
        upcoming.sort_by(|a, b| {
            a.end_date
                .cmp(&b.end_date)
                .then(a.sort_order.cmp(&b.sort_order))
        });
        let facts: Vec<DepositFacts<'_>> = rows
            .iter()
            .map(|deposit| family::facts_of(deposit))
            .collect::<Result<_, _>>()?;
        summaries.push(FamilyHolderSummary {
            note: settings::family_note(pool, &holder).await?,
            holder,
            upcoming,
            active_principal: active_totals(&facts).principal,
        });
    }

    Ok(FamilyDepositSummary {
        today: today.to_string(),
        holders: summaries,
        history_years: history_years(deposits.iter().map(|deposit| deposit.end_year), today),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::{DepositInput, validate_deposit};
    use crate::db::connect_memory;

    #[tokio::test]
    async fn matured_unreceived_deposit_counts_in_the_holders_active_principal() {
        let pool = connect_memory().await.expect("memory db");
        let validated = validate_deposit(DepositInput {
            label: Some("SC-9179"),
            bank: Some("SC"),
            principal: Some(100000.0),
            rate: None,
            interest: Some(500.0),
            start_date: None,
            end_date: "2020-01-01",
        })
        .expect("valid");
        family::insert_family_deposit(&pool, "媽媽", &validated, None, 1)
            .await
            .unwrap();

        let response = summary(&pool).await.expect("summary");
        let holder = response
            .holders
            .iter()
            .find(|holder| holder.holder == "媽媽")
            .expect("holder");
        assert_eq!(holder.active_principal, 100000.0);
        assert_eq!(holder.upcoming.len(), 1);
    }
}
