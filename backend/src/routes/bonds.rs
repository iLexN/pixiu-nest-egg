use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{
    now_timestamp, row_to_bond, row_to_coupon, today, AppState, BOND_SELECT, COUPON_SELECT,
};
use crate::calc::{
    validate_bond, validate_coupon, BondInput, CouponInput, ValidatedBond, ValidatedCoupon,
};
use crate::error::ApiError;
use crate::models::{
    Bond, BondCoupon, BondCouponPatch, BondPatch, BondStatus, NewBond, NewBondCoupon,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// `active` (maturity in the future) or `matured`.
    pub status: Option<String>,
    /// `asc` (default) or `desc`, by maturity date.
    pub order: Option<String>,
}

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
                ))
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

    let mut statement = sqlx::query(&sql);
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

/// Deleting a bond removes its coupons: nothing outside the bond references
/// them, so delete is always permitted.
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

#[derive(Debug, Serialize)]
pub struct BondTotals {
    /// The sheet's `Total` cell: Σ principal over active bonds.
    pub active_principal: f64,
}

/// A bond with its coupon schedule attached.
#[derive(Debug, Serialize)]
pub struct BondWithCoupons {
    #[serde(flatten)]
    pub bond: Bond,
    pub coupons: Vec<BondCoupon>,
}

/// A coupon shown under 即將付息, carrying its bond's label.
#[derive(Debug, Serialize)]
pub struct UpcomingCoupon {
    pub bond_label: String,
    #[serde(flatten)]
    pub coupon: BondCoupon,
}

#[derive(Debug, Serialize)]
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
pub async fn summary(State(state): State<AppState>) -> Result<Json<BondSummaryResponse>, ApiError> {
    let today = today();
    let bonds = load_all_bonds(&state.pool).await?;

    let mut active = Vec::new();
    let mut matured = Vec::new();
    let mut active_principal = 0.0;
    let mut upcoming_coupons = Vec::new();

    for bond in bonds {
        let coupons = load_bond_coupons(&state.pool, bond.id).await?;
        if bond.status == BondStatus::Active {
            active_principal += bond.principal;
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
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO bonds (label, issue_no, principal, maturity_date, note, sort_order, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&bond.label)
    .bind(bond.issue_no.as_deref())
    .bind(bond.principal)
    .bind(&bond.maturity_date)
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
    let row = sqlx::query(&format!("{BOND_SELECT} WHERE b.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("bond {id} not found")))?;
    row_to_bond(&row, today())
}

pub async fn load_all_bonds(pool: &SqlitePool) -> Result<Vec<Bond>, ApiError> {
    let rows = sqlx::query(&format!("{BOND_SELECT} ORDER BY b.sort_order, b.id"))
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
    let rows = sqlx::query(&format!(
        "{COUPON_SELECT} WHERE c.bond_id = ? ORDER BY c.pay_date, c.id"
    ))
    .bind(bond_id)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(row_to_coupon)
        .collect::<Result<Vec<_>, _>>()
}

#[derive(Debug, Deserialize)]
pub struct CouponListQuery {
    pub bond_id: Option<i64>,
}

pub async fn list_coupons(
    State(state): State<AppState>,
    Query(query): Query<CouponListQuery>,
) -> Result<Json<Vec<BondCoupon>>, ApiError> {
    let mut sql = COUPON_SELECT.to_string();
    if query.bond_id.is_some() {
        sql.push_str(" WHERE c.bond_id = ?");
    }
    sql.push_str(" ORDER BY c.pay_date, c.id");
    let mut statement = sqlx::query(&sql);
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
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_coupon(&state.pool, id).await?))
}

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
    let row = sqlx::query(&format!("{COUPON_SELECT} WHERE c.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("coupon {id} not found")))?;
    row_to_coupon(&row)
}
