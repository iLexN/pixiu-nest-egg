use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use super::{now_timestamp, today, AppState};
use crate::calc::{
    mpf_figures, mpf_last_month, mpf_max, mpf_totals, validate_mpf_account, MpfAccountFacts,
    MpfAccountInput, MpfPoint, MpfTotals,
};
use crate::error::ApiError;
use crate::models::{MpfAccount, MpfAccountPatch, MpfHistoryRow, MpfNotePatch, NewMpfAccount};
use crate::mpf;

const MPF_COLUMNS: &str = "id, label, trustee, contributions, balance, plan_name, \
     member_no, sort_order, seed_max_rate, seed_max_gain, created_at";

/// The stored row, before derived figures are attached.
#[derive(Debug, Clone)]
pub struct StoredAccount {
    id: i64,
    label: String,
    trustee: Option<String>,
    contributions: f64,
    balance: f64,
    plan_name: Option<String>,
    member_no: Option<String>,
    sort_order: i64,
    seed_max_rate: Option<f64>,
    seed_max_gain: Option<f64>,
    created_on: chrono::NaiveDate,
}

pub fn row_to_stored(row: &SqliteRow) -> Result<StoredAccount, ApiError> {
    let created_at: String = row.try_get("created_at")?;
    let created_on = chrono::DateTime::parse_from_rfc3339(&created_at)
        .map(|stamp| stamp.date_naive())
        .map_err(|_| ApiError::Conflict(format!("stored created_at {created_at} is not valid")))?;
    Ok(StoredAccount {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        trustee: row.try_get("trustee")?,
        contributions: row.try_get("contributions")?,
        balance: row.try_get("balance")?,
        plan_name: row.try_get("plan_name")?,
        member_no: row.try_get("member_no")?,
        sort_order: row.try_get("sort_order")?,
        seed_max_rate: row.try_get("seed_max_rate")?,
        seed_max_gain: row.try_get("seed_max_gain")?,
        created_on,
    })
}

fn row_to_history(row: &SqliteRow) -> Result<MpfHistoryRow, ApiError> {
    let contributions: f64 = row.try_get("contributions")?;
    let balance: f64 = row.try_get("balance")?;
    let figures = mpf_figures(contributions, balance);
    Ok(MpfHistoryRow {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        recorded_on: row.try_get("recorded_on")?,
        contributions,
        balance,
        synthetic: row.try_get::<i64, _>("synthetic")? != 0,
        rate: figures.rate,
        gain: figures.gain,
    })
}

pub async fn load_stored(pool: &SqlitePool, id: i64) -> Result<StoredAccount, ApiError> {
    let row = sqlx::query(&format!(
        "SELECT {MPF_COLUMNS} FROM mpf_accounts WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("mpf account {id} not found")))?;
    row_to_stored(&row)
}

pub async fn load_all_stored(pool: &SqlitePool) -> Result<Vec<StoredAccount>, ApiError> {
    let rows = sqlx::query(&format!(
        "SELECT {MPF_COLUMNS} FROM mpf_accounts ORDER BY sort_order, id"
    ))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_stored).collect()
}

pub async fn load_history(
    pool: &SqlitePool,
    account_id: Option<i64>,
) -> Result<Vec<MpfHistoryRow>, ApiError> {
    let sql = "SELECT id, account_id, recorded_on, contributions, balance, synthetic \
         FROM mpf_history";
    let rows = match account_id {
        Some(id) => {
            sqlx::query(&format!(
                "{sql} WHERE account_id = ? ORDER BY recorded_on, id"
            ))
            .bind(id)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query(&format!("{sql} ORDER BY recorded_on, id"))
                .fetch_all(pool)
                .await?
        }
    };
    rows.iter().map(row_to_history).collect()
}

pub fn facts_of(account: &StoredAccount, history: &[MpfHistoryRow]) -> MpfAccountFacts {
    MpfAccountFacts {
        created_on: account.created_on,
        contributions: account.contributions,
        balance: account.balance,
        seed_max_rate: account.seed_max_rate,
        seed_max_gain: account.seed_max_gain,
        history: history
            .iter()
            .filter(|row| row.account_id == account.id)
            .map(|row| MpfPoint {
                recorded_on: chrono::NaiveDate::parse_from_str(&row.recorded_on, "%Y-%m-%d")
                    .unwrap_or(account.created_on),
                contributions: row.contributions,
                balance: row.balance,
            })
            .collect(),
    }
}

pub fn present_account(
    account: &StoredAccount,
    history: &[MpfHistoryRow],
    today: chrono::NaiveDate,
) -> MpfAccount {
    let facts = facts_of(account, history);
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

pub async fn meta_f64(pool: &SqlitePool, key: &str) -> Result<Option<f64>, ApiError> {
    Ok(mpf::meta_get(pool, key)
        .await?
        .and_then(|raw| raw.parse::<f64>().ok()))
}

#[derive(Debug, Serialize)]
pub struct MpfResponse {
    pub today: String,
    pub accounts: Vec<MpfAccount>,
    pub totals: MpfTotals,
    pub note: Option<String>,
    pub history: Vec<MpfHistoryRow>,
}

pub async fn overview(State(state): State<AppState>) -> Result<Json<MpfResponse>, ApiError> {
    let today = today();
    let stored = load_all_stored(&state.pool).await?;
    let history = load_history(&state.pool, None).await?;
    let accounts: Vec<MpfAccount> = stored
        .iter()
        .map(|account| present_account(account, &history, today))
        .collect();
    let facts: Vec<MpfAccountFacts> = stored
        .iter()
        .map(|account| facts_of(account, &history))
        .collect();
    let totals = mpf_totals(
        &facts,
        today,
        meta_f64(&state.pool, mpf::SEED_MAX_RATE_KEY).await?,
        meta_f64(&state.pool, mpf::SEED_MAX_GAIN_KEY).await?,
    );
    let note = mpf::meta_get(&state.pool, mpf::NOTE_KEY).await?;
    Ok(Json(MpfResponse {
        today: today.to_string(),
        accounts,
        totals,
        note,
        history,
    }))
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewMpfAccount>,
) -> Result<(StatusCode, Json<MpfAccount>), ApiError> {
    let contributions = body.contributions.unwrap_or(0.0);
    let balance = body.balance.unwrap_or(0.0);
    validate_mpf_account(MpfAccountInput {
        label: &body.label,
        contributions,
        balance,
    })
    .map_err(ApiError::Validation)?;

    let sort_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM mpf_accounts")
            .fetch_one(&state.pool)
            .await?;
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO mpf_accounts (label, trustee, contributions, balance, plan_name, \
         member_no, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(body.label.trim())
    .bind(body.trustee.as_deref())
    .bind(contributions)
    .bind(balance)
    .bind(body.plan_name.as_deref())
    .bind(body.member_no.as_deref())
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(&state.pool)
    .await?;

    let stored = load_stored(&state.pool, id).await?;
    Ok((
        StatusCode::CREATED,
        Json(present_account(&stored, &[], today())),
    ))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<MpfAccountPatch>,
) -> Result<Json<MpfAccount>, ApiError> {
    let existing = load_stored(&state.pool, id).await?;

    let merge_text = |patch: Option<Option<String>>, existing: Option<String>| match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    };
    let merge_num = |patch: Option<Option<f64>>, existing: f64| match patch {
        Some(Some(value)) => value,
        Some(None) => existing,
        None => existing,
    };

    let label = patch.label.unwrap_or(existing.label);
    let trustee = merge_text(patch.trustee, existing.trustee);
    let plan_name = merge_text(patch.plan_name, existing.plan_name);
    let member_no = merge_text(patch.member_no, existing.member_no);
    let contributions = merge_num(patch.contributions, existing.contributions);
    let balance = merge_num(patch.balance, existing.balance);

    validate_mpf_account(MpfAccountInput {
        label: &label,
        contributions,
        balance,
    })
    .map_err(ApiError::Validation)?;

    let values_changed = contributions != existing.contributions || balance != existing.balance;

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE mpf_accounts SET label = ?, trustee = ?, contributions = ?, balance = ?, \
         plan_name = ?, member_no = ?, updated_at = ? WHERE id = ?",
    )
    .bind(label.trim())
    .bind(&trustee)
    .bind(contributions)
    .bind(balance)
    .bind(&plan_name)
    .bind(&member_no)
    .bind(now_timestamp())
    .bind(id)
    .execute(&mut *tx)
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
    tx.commit().await?;

    let stored = load_stored(&state.pool, id).await?;
    let history = load_history(&state.pool, Some(id)).await?;
    Ok(Json(present_account(&stored, &history, today())))
}

/// An account with history rows is refused so the record never loses its past.
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    load_stored(&state.pool, id).await?;
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mpf_history WHERE account_id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    if rows > 0 {
        return Err(ApiError::Conflict(format!(
            "mpf account {id} has history records"
        )));
    }
    sqlx::query("DELETE FROM mpf_accounts WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct NoteResponse {
    pub note: Option<String>,
}

pub async fn update_note(
    State(state): State<AppState>,
    Json(body): Json<MpfNotePatch>,
) -> Result<Json<NoteResponse>, ApiError> {
    mpf::meta_put(&state.pool, mpf::NOTE_KEY, body.note.as_deref()).await?;
    Ok(Json(NoteResponse {
        note: mpf::meta_get(&state.pool, mpf::NOTE_KEY).await?,
    }))
}

/// Deleting a stale row recomputes last-month/max without it on next read.
pub async fn remove_history(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM mpf_history WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "mpf history row {id} not found"
        )));
    }
    Ok(StatusCode::NO_CONTENT)
}
