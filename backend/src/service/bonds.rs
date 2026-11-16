use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{CandidateKey, ValidatedCoupon, bond_end, coupon};
use crate::models::{Bond, BondCoupon, BondStatus, Currency};
use crate::service::{ServiceError, receipts};
use crate::store::{StoreError, bonds, months, now_timestamp, today};

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
    Ok(active_principal_of(&bonds::load_all_bonds(pool).await?))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BondTotals {
    /// The sheet's `Total` cell: Σ principal over active bonds.
    pub active_principal: f64,
}

/// A bond with its coupon schedule attached.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BondWithCoupons {
    #[serde(flatten)]
    pub bond: Bond,
    pub coupons: Vec<BondCoupon>,
}

/// A coupon shown under 即將付息, carrying its bond's label.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UpcomingCoupon {
    pub bond_label: String,
    #[serde(flatten)]
    pub coupon: BondCoupon,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BondSummaryResponse {
    /// The date the summary derived against; an active bond matured on or
    /// before it is 已到期未收.
    pub today: String,
    pub totals: BondTotals,
    /// Active bonds with their coupons, earliest maturity first.
    pub active: Vec<BondWithCoupons>,
    /// Matured bonds with their coupons, latest maturity first.
    pub matured: Vec<BondWithCoupons>,
    /// Non-received coupons across active bonds, earliest pay_date first.
    pub upcoming_coupons: Vec<UpcomingCoupon>,
}

/// The 債券 page read model — every figure derived from the stored rows on
/// each read.
pub async fn summary(pool: &SqlitePool) -> Result<BondSummaryResponse, StoreError> {
    let today = today();
    let bonds = bonds::load_all_bonds(pool).await?;

    let active_principal = active_principal_of(&bonds);
    let mut active = Vec::new();
    let mut matured = Vec::new();
    let mut upcoming_coupons = Vec::new();

    for bond in bonds {
        let coupons = bonds::load_bond_coupons(pool, bond.id).await?;
        if bond.status == BondStatus::Active {
            for coupon in &coupons {
                if coupon.received_amount.is_none() {
                    upcoming_coupons.push(UpcomingCoupon {
                        bond_label: bond.label.clone(),
                        coupon: coupon.clone(),
                    });
                }
            }
            active.push(BondWithCoupons { bond, coupons });
        } else {
            matured.push(BondWithCoupons { bond, coupons });
        }
    }

    active.sort_by(|a, b| {
        a.bond
            .maturity_date
            .cmp(&b.bond.maturity_date)
            .then(a.bond.sort_order.cmp(&b.bond.sort_order))
    });
    matured.sort_by(|a, b| {
        b.bond
            .maturity_date
            .cmp(&a.bond.maturity_date)
            .then(b.bond.sort_order.cmp(&a.bond.sort_order))
    });
    upcoming_coupons.sort_by(|a, b| a.coupon.pay_date.cmp(&b.coupon.pay_date));

    Ok(BondSummaryResponse {
        today: today.to_string(),
        totals: BondTotals { active_principal },
        active,
        matured,
        upcoming_coupons,
    })
}

/// What a `POST /api/bonds/{id}/receive` body resolves to once the route
/// maps it.
pub struct ReceiveInput {
    pub received_at: String,
    /// Whether the principal banks into a cash account.
    pub bank_in: bool,
    /// The chosen account; `None` falls back to the currency default.
    pub account_id: Option<i64>,
    /// The credited amount; the route defaults it to the principal.
    pub credit_amount: f64,
}

/// 收訖 the bond's principal return — one transaction: mark received,
/// credit the amount into a cash account, and record the maturity month's
/// `bond-end` item at the credited amount (the principal when nothing was
/// credited). `Err(Conflict)` on an already-received bond.
pub async fn receive(pool: &SqlitePool, id: i64, input: &ReceiveInput) -> Result<(), ServiceError> {
    let bond = bonds::load_bond(pool, id).await?;
    if bond.received_at.is_some() {
        return Err(ServiceError::Conflict(format!(
            "bond {id} is already received"
        )));
    }
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let credit = receipts::credit(
        &mut tx,
        Currency::Hkd,
        input.credit_amount,
        input.bank_in,
        input.account_id,
        &now,
    )
    .await?;
    bonds::mark_received(&mut tx, id, &input.received_at, credit, &now).await?;
    // The bond-end Candidate rebuilds from the stored row — same event shape
    // the suggestion read.
    if let Some(event) = bonds::bond_end_event(&mut tx, id).await?
        && let Some(candidate) = bond_end(&event)
    {
        months::record_candidate(&mut tx, &candidate).await?;
    }
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
    // reverses the credit and retires the item in whichever month it sits.
    let credited_before = bonds::coupon_credit(&mut tx, id).await?;
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
            if let Some(event) = bonds::coupon_event(&mut tx, id).await?
                && let Some(candidate) = coupon(&event)
            {
                months::record_candidate(&mut tx, &candidate).await?;
            }
        }
        (Some(_), None) => {
            receipts::reverse(&mut tx, credited_before, &now).await?;
            bonds::set_coupon_credit(&mut tx, id, None).await?;
            months::retire_candidate(&mut tx, &CandidateKey::Coupon(id)).await?;
        }
        _ => {}
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use chrono::Datelike;
    use std::collections::HashSet;

    fn this_month() -> chrono::NaiveDate {
        let today = today();
        chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap()
    }

    /// The suggestion the month read shows for one key.
    async fn suggestion_of(
        pool: &SqlitePool,
        month: chrono::NaiveDate,
        auto_key: &str,
    ) -> crate::models::MonthSuggestion {
        let events = crate::service::months::load_events(pool, month, 0.0)
            .await
            .unwrap();
        let suggestions = crate::calc::build_suggestions(
            &month.to_string(),
            &events,
            &HashSet::new(),
            &HashSet::new(),
        );
        suggestions
            .into_iter()
            .find(|suggestion| suggestion.auto_key == auto_key)
            .unwrap_or_else(|| panic!("no suggestion {auto_key}"))
    }

    /// A maturing bond suggests `bond-end:<id>` at its principal; 收訖 at the
    /// same figure stores the identical item.
    #[tokio::test]
    async fn parity_bond_end() {
        use crate::models::MonthItemCategory;
        let pool = connect_memory().await.expect("memory db");
        let month = this_month();
        seed_month(&pool, &month.to_string()).await;
        let id = seed_bond(
            &pool,
            "silver bond",
            100000.0,
            &format!("{}-15", &month.to_string()[..7]),
        )
        .await;

        let suggestion = suggestion_of(&pool, month, &format!("bond-end:{id}")).await;
        assert_eq!(suggestion.category, MonthItemCategory::Adjustment);
        assert_eq!(suggestion.label.as_deref(), Some("silver bond"));
        assert_eq!(suggestion.amount, 100000.0);

        receive(
            &pool,
            id,
            &ReceiveInput {
                received_at: today().to_string(),
                bank_in: false,
                account_id: None,
                credit_amount: 100000.0,
            },
        )
        .await
        .unwrap();
        let (item_month, category, label, amount): (String, String, String, f64) = sqlx::query_as(
            "SELECT month, category, label, amount FROM month_items WHERE auto_key = ?",
        )
        .bind(format!("bond-end:{id}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            (
                item_month.as_str(),
                category.as_str(),
                label.as_str(),
                amount
            ),
            (
                month.to_string().as_str(),
                "adjustment",
                "silver bond",
                100000.0
            )
        );
    }

    /// A pending coupon suggests at its expected figure; 收訖 at that figure
    /// stores the identical item.
    #[tokio::test]
    async fn parity_coupon() {
        use crate::models::MonthItemCategory;
        let pool = connect_memory().await.expect("memory db");
        let month = this_month();
        seed_month(&pool, &month.to_string()).await;
        let bond_id = seed_bond(&pool, "silver bond", 100000.0, "2100-01-15").await;
        let pay_date = format!("{}-15", &month.to_string()[..7]);
        let coupon_id: i64 = sqlx::query_scalar(
            "INSERT INTO bond_coupons (bond_id, pay_date, annual_rate, per_10k, \
             created_at, updated_at) \
             VALUES (?, ?, 0.04, 200.0, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(bond_id)
        .bind(&pay_date)
        .fetch_one(&pool)
        .await
        .unwrap();

        let suggestion = suggestion_of(&pool, month, &format!("coupon:{coupon_id}")).await;
        assert_eq!(suggestion.category, MonthItemCategory::Adjustment);
        assert_eq!(suggestion.label.as_deref(), Some("silver bond"));
        assert_eq!(suggestion.amount, 2000.0);

        let received = ValidatedCoupon {
            pay_date,
            fixing_date: None,
            annual_rate: Some(0.04),
            per_10k: Some(200.0),
            received_amount: Some(2000.0),
        };
        update_coupon(
            &pool,
            coupon_id,
            &CouponUpdate {
                bond_id,
                validated: &received,
                note: &None,
                existing_received: None,
                bank_in: false,
                account_id: None,
            },
        )
        .await
        .unwrap();
        let (item_month, category, label, amount): (String, String, String, f64) = sqlx::query_as(
            "SELECT month, category, label, amount FROM month_items WHERE auto_key = ?",
        )
        .bind(format!("coupon:{coupon_id}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            (
                item_month.as_str(),
                category.as_str(),
                label.as_str(),
                amount
            ),
            (
                month.to_string().as_str(),
                "adjustment",
                "silver bond",
                2000.0
            )
        );
    }

    /// A matured bond inserted straight into the table stays unreceived —
    /// `insert_bond` would auto-receive the past maturity_date.
    async fn seed_matured_unreceived(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO bonds (label, issue_no, principal, maturity_date, sort_order, \
             created_at, updated_at) \
             VALUES ('silver bond', '03GB2710R', 50000, '2020-01-01', 1, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    /// One unpaid coupon on the bond — it stays in upcoming_coupons while the
    /// bond is active.
    async fn seed_unpaid_coupon(pool: &SqlitePool, bond_id: i64) {
        sqlx::query(
            "INSERT INTO bond_coupons (bond_id, pay_date, annual_rate, per_10k, \
             created_at, updated_at) \
             VALUES (?, '2020-06-01', 0.04, 200.0, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00')",
        )
        .bind(bond_id)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn seed_month(pool: &SqlitePool, month: &str) {
        sqlx::query("INSERT INTO month_stats (month, created_at, updated_at) VALUES (?, ?, ?)")
            .bind(month)
            .bind("2026-01-01T00:00:00")
            .bind("2026-01-01T00:00:00")
            .execute(pool)
            .await
            .unwrap();
    }

    async fn seed_bank(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
             sort_order, updated_at) VALUES ('HS', 'HKD', 'bank', 0.0, 1, 1, \
             '2026-01-01T00:00:00') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_bond(pool: &SqlitePool, label: &str, principal: f64, maturity: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO bonds (label, issue_no, principal, maturity_date, sort_order, \
             created_at, updated_at) VALUES (?, 'x', ?, ?, 1, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(label)
        .bind(principal)
        .bind(maturity)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn bond_end_item(pool: &SqlitePool, id: i64) -> Option<(String, f64)> {
        sqlx::query_as("SELECT month, amount FROM month_items WHERE auto_key = ?")
            .bind(format!("bond-end:{id}"))
            .fetch_optional(pool)
            .await
            .unwrap()
    }

    /// 收訖 records the maturity month's `bond-end:<id>` item at the credited
    /// amount; with `bank_in` off nothing is credited and the item records
    /// the principal instead.
    #[tokio::test]
    async fn receive_records_bond_end_at_the_credited_amount() {
        let pool = connect_memory().await.expect("memory db");
        seed_month(&pool, "2099-10-01").await;
        seed_bank(&pool).await;
        let id = seed_bond(&pool, "silver bond", 100000.0, "2099-10-15").await;

        receive(
            &pool,
            id,
            &ReceiveInput {
                received_at: today().to_string(),
                bank_in: true,
                account_id: None,
                credit_amount: 99500.0,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            bond_end_item(&pool, id).await,
            Some(("2099-10-01".to_string(), 99500.0))
        );

        let id = seed_bond(&pool, "green bond", 100000.0, "2099-10-20").await;
        receive(
            &pool,
            id,
            &ReceiveInput {
                received_at: today().to_string(),
                bank_in: false,
                account_id: None,
                credit_amount: 100000.0,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            bond_end_item(&pool, id).await,
            Some(("2099-10-01".to_string(), 100000.0))
        );
    }

    /// Moving a coupon's `pay_date` and clearing `received_amount` in one
    /// PATCH retires the `coupon:<id>` item in whichever month it sits.
    #[tokio::test]
    async fn clearing_after_the_pay_date_moved_leaves_no_item() {
        let pool = connect_memory().await.expect("memory db");
        seed_month(&pool, "2099-10-01").await;
        seed_month(&pool, "2099-11-01").await;
        let bond_id = seed_bond(&pool, "silver bond", 100000.0, "2100-01-15").await;
        let coupon_id: i64 = sqlx::query_scalar(
            "INSERT INTO bond_coupons (bond_id, pay_date, annual_rate, per_10k, \
             created_at, updated_at) \
             VALUES (?, '2099-10-15', 0.04, 200.0, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(bond_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // 收訖 stores the item in 2099-10.
        let received = ValidatedCoupon {
            pay_date: "2099-10-15".to_string(),
            fixing_date: None,
            annual_rate: Some(0.04),
            per_10k: Some(200.0),
            received_amount: Some(1000.0),
        };
        update_coupon(
            &pool,
            coupon_id,
            &CouponUpdate {
                bond_id,
                validated: &received,
                note: &None,
                existing_received: None,
                bank_in: false,
                account_id: None,
            },
        )
        .await
        .unwrap();
        let (month, amount): (String, f64) =
            sqlx::query_as("SELECT month, amount FROM month_items WHERE auto_key = ?")
                .bind(format!("coupon:{coupon_id}"))
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!((month.as_str(), amount), ("2099-10-01", 1000.0));

        // One PATCH moves the pay date and clears the receipt: the item
        // retires from 2099-10, not the new month.
        let cleared = ValidatedCoupon {
            pay_date: "2099-11-20".to_string(),
            received_amount: None,
            ..received
        };
        update_coupon(
            &pool,
            coupon_id,
            &CouponUpdate {
                bond_id,
                validated: &cleared,
                note: &None,
                existing_received: Some(1000.0),
                bank_in: false,
                account_id: None,
            },
        )
        .await
        .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = ?")
            .bind(format!("coupon:{coupon_id}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn matured_unreceived_bond_counts_until_received() {
        let pool = connect_memory().await.expect("memory db");
        let id = seed_matured_unreceived(&pool).await;
        seed_unpaid_coupon(&pool, id).await;

        // Past maturity but unreceived: still active everywhere.
        let response = summary(&pool).await.expect("summary");
        assert_eq!(response.totals.active_principal, 50000.0);
        assert_eq!(response.active.len(), 1);
        assert_eq!(response.active[0].bond.id, id);
        assert!(response.matured.is_empty());
        assert_eq!(response.upcoming_coupons.len(), 1);
        assert_eq!(response.upcoming_coupons[0].coupon.pay_date, "2020-06-01");

        let listed = bonds::list(&pool, Some(false), false)
            .await
            .expect("active list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, id);

        receive(
            &pool,
            id,
            &ReceiveInput {
                received_at: today().to_string(),
                bank_in: true,
                account_id: None,
                credit_amount: 50000.0,
            },
        )
        .await
        .expect("received");

        let response = summary(&pool).await.expect("summary");
        assert_eq!(response.totals.active_principal, 0.0);
        assert!(response.active.is_empty());
        assert_eq!(response.matured.len(), 1);
        assert!(response.upcoming_coupons.is_empty());

        let listed = bonds::list(&pool, Some(true), false)
            .await
            .expect("matured list");
        assert_eq!(listed.len(), 1);

        // Receipt is final: a second 收訖 conflicts.
        let err = receive(
            &pool,
            id,
            &ReceiveInput {
                received_at: today().to_string(),
                bank_in: true,
                account_id: None,
                credit_amount: 50000.0,
            },
        )
        .await
        .expect_err("second receive");
        assert!(matches!(err, ServiceError::Conflict(_)));
    }
}
