use sqlx::SqlitePool;

use crate::calc::{
    MpfAccountInput, MpfPoint, mpf_figures, mpf_last_month, mpf_max, validate_mpf_account,
};
use crate::models::{MpfAccount, MpfAccountPatch, MpfHistoryRow};
use crate::service::ServiceError;
use crate::store::mpf::{self, StoredAccount};
use crate::store::{StoreError, today};

/// `None` keeps `existing`; a present value is trimmed and `null`/blank
/// clears it — the shared PATCH-merge rule for nullable text.
fn merge_text(patch: Option<Option<String>>, existing: Option<String>) -> Option<String> {
    match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    }
}

/// Patch an account in one transaction: `record_history` only when
/// contributions or balance actually changed. Returns the presented account.
/// `Err(NotFound)` while `id` names no account.
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    patch: &MpfAccountPatch,
) -> Result<MpfAccount, ServiceError> {
    let existing = mpf::load_stored(pool, id).await?;

    let label = patch.label.clone().unwrap_or(existing.label);
    let trustee = merge_text(patch.trustee.clone(), existing.trustee);
    let plan_name = merge_text(patch.plan_name.clone(), existing.plan_name);
    let member_no = merge_text(patch.member_no.clone(), existing.member_no);
    let contributions = patch
        .contributions
        .flatten()
        .unwrap_or(existing.contributions);
    let balance = patch.balance.flatten().unwrap_or(existing.balance);

    validate_mpf_account(MpfAccountInput {
        label: &label,
        contributions,
        balance,
    })
    .map_err(ServiceError::Validation)?;

    let values_changed = contributions != existing.contributions || balance != existing.balance;

    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    mpf::update_account(
        &mut tx,
        id,
        label.trim(),
        &trustee,
        contributions,
        balance,
        &plan_name,
        &member_no,
    )
    .await?;
    if values_changed {
        mpf::record_history(
            &mut tx,
            id,
            (existing.contributions, existing.balance),
            (contributions, balance),
            today(),
            existing.created_on,
        )
        .await?;
    }
    tx.commit().await.map_err(StoreError::from)?;

    let stored = mpf::load_stored(pool, id).await?;
    let history = mpf::load_history(pool, Some(id)).await?;
    Ok(present_account(&stored, &history, today()))
}

/// A stored MPF account plus its derived figures and history cells.
pub fn present_account(
    account: &StoredAccount,
    history: &[MpfHistoryRow],
    today: chrono::NaiveDate,
) -> MpfAccount {
    let facts = mpf::facts_of(account, history);
    let current = MpfPoint {
        recorded_on: today,
        contributions: account.contributions,
        balance: account.balance,
    };
    let figures = mpf_figures(account.contributions, account.balance);
    MpfAccount {
        id: account.id,
        label: account.label.clone(),
        trustee: account.trustee.clone(),
        contributions: account.contributions,
        balance: account.balance,
        plan_name: account.plan_name.clone(),
        member_no: account.member_no.clone(),
        sort_order: account.sort_order,
        rate: figures.rate,
        gain: figures.gain,
        last_month: mpf_last_month(&facts.history, today),
        max: mpf_max(
            &facts.history,
            Some(current),
            account.seed_max_rate,
            account.seed_max_gain,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balance_patch(balance: f64) -> MpfAccountPatch {
        MpfAccountPatch {
            label: None,
            trustee: None,
            contributions: None,
            balance: Some(Some(balance)),
            plan_name: None,
            member_no: None,
        }
    }

    async fn history_count(pool: &SqlitePool, id: i64) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM mpf_history WHERE account_id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("count")
    }

    #[tokio::test]
    async fn update_records_history_only_when_values_change() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let id = mpf::insert_account(&pool, "MPF", None, 1000.0, 2000.0, None, None, 0)
            .await
            .expect("account");

        update(&pool, id, &balance_patch(2000.0))
            .await
            .expect("same");
        assert_eq!(history_count(&pool, id).await, 0);

        update(&pool, id, &balance_patch(2500.0))
            .await
            .expect("changed");
        assert!(history_count(&pool, id).await >= 1);
    }
}
