use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::AppState;
use crate::calc::{BondInput, CouponInput, validate_bond, validate_coupon};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    Bond, BondCoupon, BondCouponPatch, BondPatch, BondStatus, NewBond, NewBondCoupon,
};
use crate::store::bonds as store;
use crate::store::months as months_store;
use crate::store::{now_timestamp, today};

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    /// `active` (maturity in the future) or `matured`.
    pub status: Option<String>,
    /// `asc` (default) or `desc`, by maturity date.
    pub order: Option<String>,
}

#[utoipa::path(
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

    let merge_text = |patch: Option<Option<String>>, existing: Option<String>| match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    };

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
    /// Cash `manual_assets` row to credit the returned principal into.
    pub credit_asset_id: Option<i64>,
    /// Amount credited; defaults to the bond's principal.
    pub credit_amount: Option<f64>,
}

/// 收訖 the bond's principal return: mark received, optionally credit the
/// principal to a cash manual asset, and record the maturity month's
/// `bond-end` adjustment item. Coupons keep their own 收訖 — this is only
/// about the principal.
#[utoipa::path(
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
    if existing.received_at.is_some() {
        return Err(ApiError::Conflict(format!("bond {id} is already received")));
    }
    let received_at = match body.received_at.as_deref() {
        Some(date) => chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| ApiError::field("received_at", "received_at must be YYYY-MM-DD"))?
            .to_string(),
        None => today().to_string(),
    };
    let credit_amount = body.credit_amount.unwrap_or(existing.principal);
    if !credit_amount.is_finite() {
        return Err(ApiError::field(
            "credit_amount",
            "credit_amount must be a number",
        ));
    }
    if let Some(asset_id) = body.credit_asset_id {
        let kind = months_store::manual_asset_kind(&state.pool, asset_id).await?;
        match kind.as_deref() {
            Some("cash") => {}
            Some(_) => {
                return Err(ApiError::field(
                    "credit_asset_id",
                    "credit target must be a 活期 cash row",
                ));
            }
            None => {
                return Err(ApiError::NotFound(format!(
                    "manual asset {asset_id} not found"
                )));
            }
        }
    }

    let month = format!("{}-01", &existing.maturity_date[..7]);
    let auto_key = format!("bond-end:{id}");
    let now = now_timestamp();
    let mut tx = state.pool.begin().await?;
    store::mark_received(
        &mut tx,
        id,
        &received_at,
        body.credit_asset_id,
        body.credit_asset_id.map(|_| credit_amount),
        &now,
    )
    .await?;
    if let Some(asset_id) = body.credit_asset_id {
        months_store::add_manual_asset_amount(&mut tx, credit_amount, &now, asset_id).await?;
    }
    months_store::record_receipt_item(
        &mut tx,
        &month,
        &auto_key,
        &existing.label,
        existing.principal,
    )
    .await?;
    tx.commit().await?;

    Ok(Json(store::load_bond(&state.pool, id).await?))
}

/// Deleting a bond removes its coupons: nothing outside the bond references
/// them, so delete is always permitted.
#[utoipa::path(
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
    let mut tx = state.pool.begin().await?;
    if store::delete_with_coupons(&mut tx, id).await? == 0 {
        return Err(ApiError::NotFound(format!("bond {id} not found")));
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BondTotals {
    /// The sheet's `Total` cell: Σ principal over active bonds.
    pub active_principal: f64,
}

/// 債券!B1: Σ principal over active bonds, over already-loaded rows.
pub fn active_principal_of(bonds: &[Bond]) -> f64 {
    bonds
        .iter()
        .filter(|bond| bond.status == BondStatus::Active)
        .map(|bond| bond.principal)
        .sum()
}

/// 債券!B1 for callers that have not loaded the bonds (live month totals).
pub async fn active_principal(pool: &SqlitePool) -> Result<f64, ApiError> {
    Ok(active_principal_of(&store::load_all_bonds(pool).await?))
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
    /// The date `status` was derived against.
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

    let active_principal = active_principal_of(&bonds);
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

    let merge = |patch: Option<Option<f64>>, existing: Option<f64>| match patch {
        Some(value) => value,
        None => existing,
    };
    let merge_text = |patch: Option<Option<String>>, existing: Option<String>| match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    };

    let bond_id = patch.bond_id.unwrap_or(existing.bond_id);
    store::load_bond(&state.pool, bond_id).await?;
    let pay_date = patch.pay_date.unwrap_or(existing.pay_date);
    let fixing_date = merge_text(patch.fixing_date, existing.fixing_date);
    let annual_rate = merge(patch.annual_rate, existing.annual_rate);
    let per_10k = merge(patch.per_10k, existing.per_10k);
    let received_amount = merge(patch.received_amount, existing.received_amount);
    let note = merge_text(patch.note, existing.note);

    let validated = validate_coupon(CouponInput {
        pay_date: &pay_date,
        fixing_date: fixing_date.as_deref(),
        annual_rate,
        per_10k,
        received_amount,
    })
    .map_err(ApiError::Validation)?;

    let now = now_timestamp();
    let mut tx = state.pool.begin().await?;
    store::update_coupon(&mut tx, id, bond_id, &validated, &note, &now).await?;

    // 收訖: received_amount NULL → set banks the amount into HS and records
    // the coupon:<id> month item; clearing it reverses both.
    let credited_before = store::coupon_credited_amount(&mut tx, id).await?;
    let auto_key = format!("coupon:{id}");
    let month = format!("{}-01", &validated.pay_date[..7]);
    match (existing.received_amount, validated.received_amount) {
        (None, Some(amount)) => {
            let mut credited = None;
            if patch.bank_in.unwrap_or(true)
                && let Some(asset_id) = months_store::hs_cash_asset_id(&mut tx).await?
            {
                months_store::add_manual_asset_amount(&mut tx, amount, &now, asset_id).await?;
                credited = Some(amount);
            }
            store::set_coupon_credited(&mut tx, id, credited).await?;
            let bond_label = store::bond_label(&mut tx, bond_id).await?;
            months_store::record_receipt_item(&mut tx, &month, &auto_key, &bond_label, amount)
                .await?;
        }
        (Some(_), None) => {
            if let Some(amount) = credited_before
                && let Some(asset_id) = months_store::hs_cash_asset_id(&mut tx).await?
            {
                months_store::add_manual_asset_amount(&mut tx, -amount, &now, asset_id).await?;
            }
            store::set_coupon_credited(&mut tx, id, None).await?;
            months_store::delete_auto_item(&mut tx, &month, &auto_key).await?;
        }
        _ => {}
    }
    tx.commit().await?;

    Ok(Json(store::load_coupon(&state.pool, id).await?))
}

#[utoipa::path(
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
