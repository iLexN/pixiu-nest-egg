use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{
    AppState, BOND_SELECT, COUPON_SELECT, now_timestamp, record_receipt_item, row_to_bond,
    row_to_coupon, today,
};
use crate::calc::{
    BondInput, CouponInput, ValidatedBond, ValidatedCoupon, validate_bond, validate_coupon,
};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    Bond, BondCoupon, BondCouponPatch, BondPatch, BondStatus, NewBond, NewBondCoupon,
};

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
    let mut sql = BOND_SELECT.to_string();
    let mut conditions: Vec<&str> = Vec::new();
    if let Some(status) = query.status.as_deref() {
        match status.to_ascii_lowercase().as_str() {
            "active" => conditions.push("b.maturity_date > ?"),
            "matured" => conditions.push("b.maturity_date <= ?"),
            _ => {
                return Err(ApiError::field(
                    "status",
                    "status must be active or matured",
                ));
            }
        }
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    let descending =
        matches!(query.order.as_deref(), Some(order) if order.eq_ignore_ascii_case("desc"));
    sql.push_str(if descending {
        " ORDER BY b.maturity_date DESC, b.sort_order DESC, b.id DESC"
    } else {
        " ORDER BY b.maturity_date ASC, b.sort_order ASC, b.id ASC"
    });

    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if query.status.is_some() {
        statement = statement.bind(today().to_string());
    }
    let today = today();
    statement
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(|row| row_to_bond(row, today))
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
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

    let sort_order: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM bonds")
        .fetch_one(&state.pool)
        .await?;
    let id = insert_bond(&state.pool, &validated, sort_order, &body).await?;
    Ok((StatusCode::CREATED, Json(load_bond(&state.pool, id).await?)))
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
    let existing = load_bond(&state.pool, id).await?;

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

    sqlx::query(
        "UPDATE bonds SET label = ?, issue_no = ?, principal = ?, maturity_date = ?, \
         note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&validated.label)
    .bind(validated.issue_no.as_deref())
    .bind(validated.principal)
    .bind(&validated.maturity_date)
    .bind(&note)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_bond(&state.pool, id).await?))
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
    let existing = load_bond(&state.pool, id).await?;
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
        let kind: Option<String> =
            sqlx::query_scalar("SELECT kind FROM manual_assets WHERE id = ?")
                .bind(asset_id)
                .fetch_optional(&state.pool)
                .await?;
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
    sqlx::query(
        "UPDATE bonds SET received_at = ?, credited_asset_id = ?, credited_amount = ?, \
         updated_at = ? WHERE id = ?",
    )
    .bind(&received_at)
    .bind(body.credit_asset_id)
    .bind(body.credit_asset_id.map(|_| credit_amount))
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if let Some(asset_id) = body.credit_asset_id {
        sqlx::query("UPDATE manual_assets SET amount = amount + ?, updated_at = ? WHERE id = ?")
            .bind(credit_amount)
            .bind(&now)
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;
    }
    record_receipt_item(
        &mut tx,
        &month,
        &auto_key,
        &existing.label,
        existing.principal,
    )
    .await?;
    tx.commit().await?;

    Ok(Json(load_bond(&state.pool, id).await?))
}

/// Undo a bond 收訖: reverse the stored cash credit, drop the bond-end item,
/// and clear the received flag.
#[utoipa::path(
    post,
    path = "/api/bonds/{id}/unreceive",
    tag = "bonds",
    params(("id" = i64, Path, description = "Bond id")),
    responses(
        (status = 200, description = "收訖 cleared and bank-in reversed", body = Bond),
        (status = 404, description = "Bond not found", body = ErrorBody),
        (status = 409, description = "Bond is not received", body = ErrorBody),
    )
)]
pub async fn unreceive(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Bond>, ApiError> {
    let existing = load_bond(&state.pool, id).await?;
    if existing.received_at.is_none() {
        return Err(ApiError::Conflict(format!("bond {id} is not received")));
    }
    let credited: Option<(Option<i64>, Option<f64>)> =
        sqlx::query_as("SELECT credited_asset_id, credited_amount FROM bonds WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;

    let month = format!("{}-01", &existing.maturity_date[..7]);
    let now = now_timestamp();
    let mut tx = state.pool.begin().await?;
    if let Some((Some(asset_id), Some(amount))) = credited {
        sqlx::query("UPDATE manual_assets SET amount = amount - ?, updated_at = ? WHERE id = ?")
            .bind(amount)
            .bind(&now)
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM month_items WHERE month = ? AND auto_key = ?")
        .bind(&month)
        .bind(format!("bond-end:{id}"))
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE bonds SET received_at = NULL, credited_asset_id = NULL, \
         credited_amount = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(load_bond(&state.pool, id).await?))
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
    sqlx::query("DELETE FROM bond_coupons WHERE bond_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("DELETE FROM bonds WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
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
    Ok(active_principal_of(&load_all_bonds(pool).await?))
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
    let bonds = load_all_bonds(&state.pool).await?;

    let active_principal = active_principal_of(&bonds);
    let mut active = Vec::new();
    let mut matured = Vec::new();
    let mut upcoming_coupons = Vec::new();

    for bond in bonds {
        let coupons = load_bond_coupons(&state.pool, bond.id).await?;
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

pub async fn insert_bond(
    pool: &SqlitePool,
    bond: &ValidatedBond,
    sort_order: i64,
    body: &NewBond,
) -> Result<i64, ApiError> {
    let now = now_timestamp();
    // A bond entered already past maturity is recorded as received — its
    // principal already came back. 收訖 only applies to live bonds.
    let received_at = (bond.maturity_date.as_str() <= today().to_string().as_str())
        .then(|| bond.maturity_date.clone());
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO bonds (label, issue_no, principal, maturity_date, received_at, note, \
         sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&bond.label)
    .bind(bond.issue_no.as_deref())
    .bind(bond.principal)
    .bind(&bond.maturity_date)
    .bind(&received_at)
    .bind(body.note.as_deref())
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn insert_coupon(
    pool: &SqlitePool,
    coupon: &ValidatedCoupon,
    body: &NewBondCoupon,
) -> Result<i64, ApiError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO bond_coupons (bond_id, pay_date, fixing_date, annual_rate, per_10k, \
         received_amount, note, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(body.bond_id)
    .bind(&coupon.pay_date)
    .bind(coupon.fixing_date.as_deref())
    .bind(coupon.annual_rate)
    .bind(coupon.per_10k)
    .bind(coupon.received_amount)
    .bind(body.note.as_deref())
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn load_bond(pool: &SqlitePool, id: i64) -> Result<Bond, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{BOND_SELECT} WHERE b.id = ?")))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("bond {id} not found")))?;
    row_to_bond(&row, today())
}

pub async fn load_all_bonds(pool: &SqlitePool) -> Result<Vec<Bond>, ApiError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{BOND_SELECT} ORDER BY b.sort_order, b.id"
    )))
    .fetch_all(pool)
    .await?;
    let today = today();
    rows.iter()
        .map(|row| row_to_bond(row, today))
        .collect::<Result<Vec<_>, _>>()
}

pub async fn load_bond_coupons(
    pool: &SqlitePool,
    bond_id: i64,
) -> Result<Vec<BondCoupon>, ApiError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{COUPON_SELECT} WHERE c.bond_id = ? ORDER BY c.pay_date, c.id"
    )))
    .bind(bond_id)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(row_to_coupon)
        .collect::<Result<Vec<_>, _>>()
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
    let mut sql = COUPON_SELECT.to_string();
    if query.bond_id.is_some() {
        sql.push_str(" WHERE c.bond_id = ?");
    }
    sql.push_str(" ORDER BY c.pay_date, c.id");
    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(bond_id) = query.bond_id {
        statement = statement.bind(bond_id);
    }
    statement
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(row_to_coupon)
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
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
    load_bond(&state.pool, body.bond_id).await?;
    let validated = validate_coupon(CouponInput {
        pay_date: &body.pay_date,
        fixing_date: body.fixing_date.as_deref(),
        annual_rate: body.annual_rate,
        per_10k: body.per_10k,
        received_amount: body.received_amount,
    })
    .map_err(ApiError::Validation)?;

    let id = insert_coupon(&state.pool, &validated, &body).await?;
    Ok((
        StatusCode::CREATED,
        Json(load_coupon(&state.pool, id).await?),
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
    let existing = load_coupon(&state.pool, id).await?;

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
    load_bond(&state.pool, bond_id).await?;
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
    sqlx::query(
        "UPDATE bond_coupons SET bond_id = ?, pay_date = ?, fixing_date = ?, annual_rate = ?, \
         per_10k = ?, received_amount = ?, note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(bond_id)
    .bind(&validated.pay_date)
    .bind(validated.fixing_date.as_deref())
    .bind(validated.annual_rate)
    .bind(validated.per_10k)
    .bind(validated.received_amount)
    .bind(&note)
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    // 收訖: received_amount NULL → set banks the amount into HS and records
    // the coupon:<id> month item; clearing it reverses both.
    let credited_before: Option<f64> =
        sqlx::query_scalar("SELECT credited_amount FROM bond_coupons WHERE id = ?")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    let auto_key = format!("coupon:{id}");
    let month = format!("{}-01", &validated.pay_date[..7]);
    match (existing.received_amount, validated.received_amount) {
        (None, Some(amount)) => {
            let mut credited = None;
            if patch.bank_in.unwrap_or(true)
                && let Some(asset_id) = super::hs_cash_asset_id(&mut tx).await?
            {
                sqlx::query(
                    "UPDATE manual_assets SET amount = amount + ?, updated_at = ? WHERE id = ?",
                )
                .bind(amount)
                .bind(&now)
                .bind(asset_id)
                .execute(&mut *tx)
                .await?;
                credited = Some(amount);
            }
            sqlx::query("UPDATE bond_coupons SET credited_amount = ? WHERE id = ?")
                .bind(credited)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            let bond_label: String = sqlx::query_scalar("SELECT label FROM bonds WHERE id = ?")
                .bind(bond_id)
                .fetch_one(&mut *tx)
                .await?;
            record_receipt_item(&mut tx, &month, &auto_key, &bond_label, amount).await?;
        }
        (Some(_), None) => {
            if let Some(amount) = credited_before
                && let Some(asset_id) = super::hs_cash_asset_id(&mut tx).await?
            {
                sqlx::query(
                    "UPDATE manual_assets SET amount = amount - ?, updated_at = ? WHERE id = ?",
                )
                .bind(amount)
                .bind(&now)
                .bind(asset_id)
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query("UPDATE bond_coupons SET credited_amount = NULL WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM month_items WHERE month = ? AND auto_key = ?")
                .bind(&month)
                .bind(&auto_key)
                .execute(&mut *tx)
                .await?;
        }
        _ => {}
    }
    tx.commit().await?;

    Ok(Json(load_coupon(&state.pool, id).await?))
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
    let result = sqlx::query("DELETE FROM bond_coupons WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("coupon {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn load_coupon(pool: &SqlitePool, id: i64) -> Result<BondCoupon, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{COUPON_SELECT} WHERE c.id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("coupon {id} not found")))?;
    row_to_coupon(&row)
}
