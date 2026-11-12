use super::AppState;
use super::patch::{merge_num, merge_text};
use crate::calc::{BondInput, CouponInput, validate_bond, validate_coupon};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    Bond, BondCoupon, BondCouponPatch, BondPatch, BondStatus, NewBond, NewBondCoupon,
};
use crate::store::bonds as store;
use crate::store::today;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    /// `active` (principal unreceived) or `matured` (principal 收訖'd).
    pub status: Option<String>,
    /// `asc` (default) or `desc`, by maturity date.
    pub order: Option<String>,
}

#[utoipa::path(
    operation_id = "bonds_list",
    get,
    path = "/api/bonds",
    tag = "bonds",
    params(ListQuery),
    responses(
        (status = 200, description = "List bonds", body = [Bond]),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Bond>>, ApiError> {
    let matured = match query.status.as_deref() {
        Some(status) => match status.to_ascii_lowercase().as_str() {
            "active" => Some(false),
            "matured" => Some(true),
            _ => {
                return Err(ApiError::field(
                    "status",
                    "status must be active or matured",
                ));
            }
        },
        None => None,
    };
    let descending =
        matches!(query.order.as_deref(), Some(order) if order.eq_ignore_ascii_case("desc"));
    store::list(&state.pool, matured, descending)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

#[utoipa::path(
    operation_id = "bonds_create",
    post,
    path = "/api/bonds",
    tag = "bonds",
    request_body = NewBond,
    responses(
        (status = 201, description = "Bond created", body = Bond),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewBond>,
) -> Result<(StatusCode, Json<Bond>), ApiError> {
    let validated = validate_bond(BondInput {
        label: &body.label,
        issue_no: body.issue_no.as_deref(),
        principal: body.principal,
        maturity_date: &body.maturity_date,
    })
    .map_err(ApiError::Validation)?;

    let sort_order = store::next_sort_order(&state.pool).await?;
    let id = store::insert_bond(&state.pool, &validated, sort_order, &body).await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_bond(&state.pool, id).await?),
    ))
}

#[utoipa::path(
    operation_id = "bonds_update",
    patch,
    path = "/api/bonds/{id}",
    tag = "bonds",
    params(("id" = i64, Path, description = "Bond id")),
    request_body = BondPatch,
    responses(
        (status = 200, description = "Updated bond", body = Bond),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Bond not found", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<BondPatch>,
) -> Result<Json<Bond>, ApiError> {
    let existing = store::load_bond(&state.pool, id).await?;

    let label = patch.label.unwrap_or(existing.label);
    let issue_no = merge_text(patch.issue_no, existing.issue_no);
    let note = merge_text(patch.note, existing.note);
    let principal = patch.principal.unwrap_or(existing.principal);
    let maturity_date = patch.maturity_date.unwrap_or(existing.maturity_date);

    let validated = validate_bond(BondInput {
        label: &label,
        issue_no: issue_no.as_deref(),
        principal,
        maturity_date: &maturity_date,
    })
    .map_err(ApiError::Validation)?;

    store::update_bond(&state.pool, id, &validated, &note).await?;

    Ok(Json(store::load_bond(&state.pool, id).await?))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ReceiveBond {
    /// 收訖日 (`YYYY-MM-DD`); defaults to today.
    pub received_at: Option<String>,
    /// Bank the returned principal into a cash account. Defaults to true.
    #[serde(default)]
    pub bank_in: Option<bool>,
    /// The chosen receipt account; falls back to the HKD default.
    #[serde(default)]
    pub account_id: Option<i64>,
    /// Amount credited; defaults to the bond's principal.
    pub credit_amount: Option<f64>,
}

/// 收訖 the bond's principal return: mark received, optionally credit the
/// principal to a cash manual asset, and record the maturity month's
/// `bond-end` adjustment item. Coupons keep their own 收訖 — this is only
/// about the principal.
#[utoipa::path(
    operation_id = "bonds_receive",
    post,
    path = "/api/bonds/{id}/receive",
    tag = "bonds",
    params(("id" = i64, Path, description = "Bond id")),
    request_body = ReceiveBond,
    responses(
        (status = 200, description = "Matured principal marked 收訖 with optional bank-in", body = Bond),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Bond not found", body = ErrorBody),
        (status = 409, description = "Bond already received", body = ErrorBody),
    )
)]
pub async fn receive(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ReceiveBond>,
) -> Result<Json<Bond>, ApiError> {
    let existing = store::load_bond(&state.pool, id).await?;
    let received_at = match body.received_at.as_deref() {
        Some(date) => chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| ApiError::field("received_at", "received_at must be YYYY-MM-DD"))?
            .to_string(),
        None => today().to_string(),
    };
    let credit_amount = match body.credit_amount {
        Some(value) if !value.is_finite() => {
            return Err(ApiError::field(
                "credit_amount",
                "credit_amount must be a number",
            ));
        }
        Some(value) => value,
        None => existing.principal,
    };
    crate::service::bonds::receive(
        &state.pool,
        id,
        &crate::service::bonds::ReceiveInput {
            received_at,
            bank_in: body.bank_in.unwrap_or(true),
            account_id: body.account_id,
            credit_amount,
        },
    )
    .await?;

    Ok(Json(store::load_bond(&state.pool, id).await?))
}

/// Deleting a bond removes its coupons: nothing outside the bond references
/// them, so delete is always permitted.
#[utoipa::path(
    operation_id = "bonds_remove",
    delete,
    path = "/api/bonds/{id}",
    tag = "bonds",
    params(("id" = i64, Path, description = "Bond id")),
    responses(
        (status = 204, description = "Bond and its coupons deleted"),
        (status = 404, description = "Bond not found", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if store::delete_with_coupons(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!("bond {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
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

/// Every figure here is derived from the stored rows on each read.
#[utoipa::path(
    operation_id = "bonds_summary",
    get,
    path = "/api/bonds/summary",
    tag = "bonds",
    responses(
        (status = 200, description = "Active/matured bonds with totals and upcoming coupons", body = BondSummaryResponse),
    )
)]
pub async fn summary(State(state): State<AppState>) -> Result<Json<BondSummaryResponse>, ApiError> {
    let today = today();
    let bonds = store::load_all_bonds(&state.pool).await?;

    let active_principal = crate::service::bonds::active_principal_of(&bonds);
    let mut active = Vec::new();
    let mut matured = Vec::new();
    let mut upcoming_coupons = Vec::new();

    for bond in bonds {
        let coupons = store::load_bond_coupons(&state.pool, bond.id).await?;
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

    Ok(Json(BondSummaryResponse {
        today: today.to_string(),
        totals: BondTotals { active_principal },
        active,
        matured,
        upcoming_coupons,
    }))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct CouponListQuery {
    pub bond_id: Option<i64>,
}

#[utoipa::path(
    operation_id = "bonds_list_coupons",
    get,
    path = "/api/coupons",
    tag = "bonds",
    params(CouponListQuery),
    responses(
        (status = 200, description = "List bond coupons", body = [BondCoupon]),
    )
)]
pub async fn list_coupons(
    State(state): State<AppState>,
    Query(query): Query<CouponListQuery>,
) -> Result<Json<Vec<BondCoupon>>, ApiError> {
    store::list_coupons(&state.pool, query.bond_id)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

#[utoipa::path(
    operation_id = "bonds_create_coupon",
    post,
    path = "/api/coupons",
    tag = "bonds",
    request_body = NewBondCoupon,
    responses(
        (status = 201, description = "Coupon created", body = BondCoupon),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Bond not found", body = ErrorBody),
    )
)]
pub async fn create_coupon(
    State(state): State<AppState>,
    Json(body): Json<NewBondCoupon>,
) -> Result<(StatusCode, Json<BondCoupon>), ApiError> {
    store::load_bond(&state.pool, body.bond_id).await?;
    let validated = validate_coupon(CouponInput {
        pay_date: &body.pay_date,
        fixing_date: body.fixing_date.as_deref(),
        annual_rate: body.annual_rate,
        per_10k: body.per_10k,
        received_amount: body.received_amount,
    })
    .map_err(ApiError::Validation)?;

    let id = store::insert_coupon(&state.pool, &validated, &body).await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_coupon(&state.pool, id).await?),
    ))
}

#[utoipa::path(
    operation_id = "bonds_update_coupon",
    patch,
    path = "/api/coupons/{id}",
    tag = "bonds",
    params(("id" = i64, Path, description = "Coupon id")),
    request_body = BondCouponPatch,
    responses(
        (status = 200, description = "Updated coupon", body = BondCoupon),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Coupon not found", body = ErrorBody),
    )
)]
pub async fn update_coupon(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<BondCouponPatch>,
) -> Result<Json<BondCoupon>, ApiError> {
    let existing = store::load_coupon(&state.pool, id).await?;

    let bond_id = patch.bond_id.unwrap_or(existing.bond_id);
    store::load_bond(&state.pool, bond_id).await?;
    let pay_date = patch.pay_date.unwrap_or(existing.pay_date);
    let fixing_date = merge_text(patch.fixing_date, existing.fixing_date);
    let annual_rate = merge_num(patch.annual_rate, existing.annual_rate);
    let per_10k = merge_num(patch.per_10k, existing.per_10k);
    let received_amount = merge_num(patch.received_amount, existing.received_amount);
    let note = merge_text(patch.note, existing.note);

    let validated = validate_coupon(CouponInput {
        pay_date: &pay_date,
        fixing_date: fixing_date.as_deref(),
        annual_rate,
        per_10k,
        received_amount,
    })
    .map_err(ApiError::Validation)?;

    crate::service::bonds::update_coupon(
        &state.pool,
        id,
        &crate::service::bonds::CouponUpdate {
            bond_id,
            validated: &validated,
            note: &note,
            existing_received: existing.received_amount,
            bank_in: patch.bank_in.unwrap_or(true),
            account_id: patch.account_id,
        },
    )
    .await?;

    Ok(Json(store::load_coupon(&state.pool, id).await?))
}

#[utoipa::path(
    operation_id = "bonds_remove_coupon",
    delete,
    path = "/api/coupons/{id}",
    tag = "bonds",
    params(("id" = i64, Path, description = "Coupon id")),
    responses(
        (status = 204, description = "Coupon deleted"),
        (status = 404, description = "Coupon not found", body = ErrorBody),
    )
)]
pub async fn remove_coupon(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if store::delete_coupon(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!("coupon {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use sqlx::SqlitePool;

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

    fn receive_body() -> ReceiveBond {
        ReceiveBond {
            received_at: None,
            bank_in: None,
            account_id: None,
            credit_amount: None,
        }
    }

    #[tokio::test]
    async fn matured_unreceived_bond_counts_until_received() {
        let pool = connect_memory().await.expect("memory db");
        let id = seed_matured_unreceived(&pool).await;
        seed_unpaid_coupon(&pool, id).await;
        let state = AppState { pool: pool.clone() };

        // Past maturity but unreceived: still active everywhere.
        let response = summary(State(state.clone())).await.expect("summary").0;
        assert_eq!(response.totals.active_principal, 50000.0);
        assert_eq!(response.active.len(), 1);
        assert_eq!(response.active[0].bond.id, id);
        assert!(response.matured.is_empty());
        assert_eq!(response.upcoming_coupons.len(), 1);
        assert_eq!(response.upcoming_coupons[0].coupon.pay_date, "2020-06-01");

        let listed = list(
            State(state.clone()),
            Query(ListQuery {
                status: Some("active".to_string()),
                order: None,
            }),
        )
        .await
        .expect("active list")
        .0;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, id);

        let _received = receive(State(state.clone()), Path(id), Json(receive_body()))
            .await
            .expect("received");

        let response = summary(State(state.clone())).await.expect("summary").0;
        assert_eq!(response.totals.active_principal, 0.0);
        assert!(response.active.is_empty());
        assert_eq!(response.matured.len(), 1);
        assert!(response.upcoming_coupons.is_empty());

        let listed = list(
            State(state.clone()),
            Query(ListQuery {
                status: Some("matured".to_string()),
                order: None,
            }),
        )
        .await
        .expect("matured list")
        .0;
        assert_eq!(listed.len(), 1);

        // Receipt is final: a second 收訖 conflicts.
        let err = receive(State(state), Path(id), Json(receive_body()))
            .await
            .expect_err("second receive");
        assert!(matches!(err, ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn early_received_bond_is_matured() {
        let pool = connect_memory().await.expect("memory db");
        // Future maturity, already 收訖 — receipt, not the date, ends it.
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO bonds (label, principal, maturity_date, received_at, sort_order, \
             created_at, updated_at) \
             VALUES ('silver bond', 50000, '2999-01-01', '2026-11-12', 1, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let bond = store::load_bond(&pool, id).await.expect("bond");
        assert_eq!(bond.status, BondStatus::Matured);
    }

    #[tokio::test]
    async fn unreceived_matured_bond_counts_in_live_totals() {
        let pool = connect_memory().await.expect("memory db");
        let id = seed_matured_unreceived(&pool).await;

        // The live totals read active bond principal — it counts until 收訖.
        assert_eq!(
            crate::service::bonds::active_principal(&pool)
                .await
                .unwrap(),
            50000.0
        );
        let input = crate::service::wealth::totals_input(&pool).await.unwrap();
        let liquid_before = crate::calc::live_totals(&input).liquid_assets;

        crate::service::bonds::receive(
            &pool,
            id,
            &crate::service::bonds::ReceiveInput {
                received_at: "2026-11-12".to_string(),
                bank_in: false,
                account_id: None,
                credit_amount: 50000.0,
            },
        )
        .await
        .expect("received");

        assert_eq!(
            crate::service::bonds::active_principal(&pool)
                .await
                .unwrap(),
            0.0
        );
        // 不存入 was picked, so the principal leaves liquid_assets whole.
        let input = crate::service::wealth::totals_input(&pool).await.unwrap();
        let liquid_after = crate::calc::live_totals(&input).liquid_assets;
        assert!((liquid_before - liquid_after - 50000.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn received_bond_banked_into_hs_leaves_liquid_unchanged() {
        let pool = connect_memory().await.expect("memory db");
        let id = seed_matured_unreceived(&pool).await;
        let hs_id: i64 = sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
             sort_order, updated_at) \
             VALUES ('HS', 'HKD', 'bank', 0, 1, 1, '2026-01-01T00:00:00') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let input = crate::service::wealth::totals_input(&pool).await.unwrap();
        let liquid_before = crate::calc::live_totals(&input).liquid_assets;

        crate::service::bonds::receive(
            &pool,
            id,
            &crate::service::bonds::ReceiveInput {
                received_at: "2026-11-12".to_string(),
                bank_in: true,
                account_id: Some(hs_id),
                credit_amount: 50000.0,
            },
        )
        .await
        .expect("received");

        assert_eq!(
            crate::service::bonds::active_principal(&pool)
                .await
                .unwrap(),
            0.0
        );
        let hs_amount: f64 = sqlx::query_scalar("SELECT amount FROM cash_accounts WHERE id = ?")
            .bind(hs_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(hs_amount, 50000.0);

        // 收訖 moves the principal from the bond to HS — 流動資產 unchanged.
        let input = crate::service::wealth::totals_input(&pool).await.unwrap();
        let liquid_after = crate::calc::live_totals(&input).liquid_assets;
        assert!((liquid_after - liquid_before).abs() < 1e-6);
    }
}
