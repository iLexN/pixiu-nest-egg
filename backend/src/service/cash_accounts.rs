//! Cash-account CRUD validation: every field's rules live here; the store
//! enforces the unique constraints (label, one broker per currency, one
//! default per currency) that surface as 409s.

use sqlx::SqlitePool;

use crate::calc::FieldError;
use crate::models::{CashAccount, CashAccountPatch, CashRole, Currency, NewCashAccount};
use crate::service::ServiceError;
use crate::store::cash_accounts;

fn validate(
    label: &str,
    currency: Currency,
    role: CashRole,
    amount: f64,
) -> Result<String, ServiceError> {
    let mut errors = Vec::new();
    let label = label.trim();
    if label.is_empty() {
        errors.push(FieldError::new("label", "label is required"));
    }
    if !amount.is_finite() || amount < 0.0 {
        errors.push(FieldError::new(
            "amount",
            "amount must be a non-negative number",
        ));
    }
    if role == CashRole::Bank && currency != Currency::Hkd {
        errors.push(FieldError::new("currency", "bank accounts must be HKD"));
    }
    if errors.is_empty() {
        Ok(label.to_string())
    } else {
        Err(ServiceError::Validation(errors))
    }
}

pub async fn create(pool: &SqlitePool, body: &NewCashAccount) -> Result<CashAccount, ServiceError> {
    let amount = body.amount.unwrap_or(0.0);
    let label = validate(&body.label, body.currency, body.role, amount)?;
    let sort_order = cash_accounts::next_sort_order(pool).await?;
    let id = cash_accounts::insert(
        pool,
        &label,
        body.currency,
        body.role,
        amount,
        body.is_default_receipt.unwrap_or(false),
        sort_order,
    )
    .await?;
    Ok(cash_accounts::load_one(pool, id).await?)
}

/// `Err(NotFound)` while `id` names no account.
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    patch: &CashAccountPatch,
) -> Result<CashAccount, ServiceError> {
    let existing = cash_accounts::load_one(pool, id).await?;
    let currency = patch.currency.unwrap_or(existing.currency);
    let role = patch.role.unwrap_or(existing.role);
    let amount = patch.amount.unwrap_or(existing.amount);
    let label = validate(
        patch.label.as_deref().unwrap_or(&existing.label),
        currency,
        role,
        amount,
    )?;
    cash_accounts::update(
        pool,
        id,
        &label,
        currency,
        role,
        amount,
        patch
            .is_default_receipt
            .unwrap_or(existing.is_default_receipt),
        existing.sort_order,
    )
    .await?;
    Ok(cash_accounts::load_one(pool, id).await?)
}

/// `Err(NotFound)` for a missing id; `Err(Conflict)` for a broker account.
pub async fn remove(pool: &SqlitePool, id: i64) -> Result<(), ServiceError> {
    cash_accounts::delete(pool, id).await?;
    Ok(())
}
