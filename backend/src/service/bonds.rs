use sqlx::SqlitePool;

use crate::calc::ValidatedCoupon;
use crate::models::{Bond, BondStatus, Currency};
use crate::service::{ServiceError, receipts};
use crate::store::{StoreError, bonds, months, now_timestamp};

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

/// What a `POST /api/bonds/{id}/receive` body resolves to.
pub struct ReceiveInput {
    pub received_at: String,
    /// Whether the principal banks into a cash account.
    pub bank_in: bool,
    /// The chosen account; `None` falls back to the currency default.
    pub account_id: Option<i64>,
    /// The credited amount; defaults to the principal.
    pub credit_amount: Option<f64>,
}

/// 收訖 the bond's principal return — one transaction: mark received,
/// credit the amount into a cash account, and record the maturity month's
/// `bond-end` item. `Err(Conflict)` on an already-received bond.
pub async fn receive(pool: &SqlitePool, id: i64, input: &ReceiveInput) -> Result<(), ServiceError> {
    let bond = bonds::load_bond(pool, id).await?;
    if bond.received_at.is_some() {
        return Err(ServiceError::Conflict(format!(
            "bond {id} is already received"
        )));
    }
    let credit_amount = input.credit_amount.unwrap_or(bond.principal);
    // The bond-end item lands in the maturity month and carries the
    // principal, as the suggestion would have.
    let month = format!("{}-01", &bond.maturity_date[..7]);
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let credit = receipts::credit(
        &mut tx,
        Currency::Hkd,
        credit_amount,
        input.bank_in,
        input.account_id,
        &now,
    )
    .await?;
    bonds::mark_received(&mut tx, id, &input.received_at, credit, &now).await?;
    months::record_receipt_item(
        &mut tx,
        &month,
        &format!("bond-end:{id}"),
        &bond.label,
        bond.principal,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// The coupon PATCH transition inputs — the route keeps merge+validate and
/// passes the resolved values here.
pub struct CouponUpdate<'a> {
    pub bond_id: i64,
    pub validated: &'a ValidatedCoupon,
    pub note: &'a Option<String>,
    /// The coupon's `received_amount` before the update.
    pub existing_received: Option<f64>,
    pub bank_in: bool,
    /// The chosen receipt account; `None` falls back to the currency default.
    pub account_id: Option<i64>,
}

/// The coupon PATCH transition — this owns the row update, the receipt
/// credit/reversal, and the `coupon:<id>` month item.
pub async fn update_coupon(
    pool: &SqlitePool,
    id: i64,
    input: &CouponUpdate<'_>,
) -> Result<(), ServiceError> {
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    bonds::update_coupon(
        &mut tx,
        id,
        input.bond_id,
        input.validated,
        input.note,
        &now,
    )
    .await?;

    // 收訖: received_amount NULL → set banks the amount into the chosen or
    // default HKD account and records the coupon:<id> month item; clearing it
    // reverses both through the recorded credit.
    let credited_before = bonds::coupon_credit(&mut tx, id).await?;
    let auto_key = format!("coupon:{id}");
    let month = format!("{}-01", &input.validated.pay_date[..7]);
    match (input.existing_received, input.validated.received_amount) {
        (None, Some(amount)) => {
            let credit = receipts::credit(
                &mut tx,
                Currency::Hkd,
                amount,
                input.bank_in,
                input.account_id,
                &now,
            )
            .await?;
            bonds::set_coupon_credit(&mut tx, id, credit).await?;
            let bond_label = bonds::bond_label(&mut tx, input.bond_id).await?;
            months::record_receipt_item(&mut tx, &month, &auto_key, &bond_label, amount).await?;
        }
        (Some(_), None) => {
            receipts::reverse(&mut tx, credited_before, &now).await?;
            bonds::set_coupon_credit(&mut tx, id, None).await?;
            months::delete_auto_item(&mut tx, &month, &auto_key).await?;
        }
        _ => {}
    }
    tx.commit().await?;
    Ok(())
}
