use sqlx::SqlitePool;

use crate::models::{Bond, BondStatus};
use crate::store::StoreError;

/// 債券!B1: Σ principal over active bonds, over already-loaded rows.
pub fn active_principal_of(bonds: &[Bond]) -> f64 {
    bonds
        .iter()
        .filter(|bond| bond.status == BondStatus::Active)
        .map(|bond| bond.principal)
        .sum()
}

/// 債券!B1 for callers that have not loaded the bonds (live month totals).
pub async fn active_principal(pool: &SqlitePool) -> Result<f64, StoreError> {
    Ok(active_principal_of(
        &crate::store::bonds::load_all_bonds(pool).await?,
    ))
}
