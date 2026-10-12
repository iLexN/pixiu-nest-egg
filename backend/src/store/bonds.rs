use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::calc::{
    BondYearFacts, SuggestionReceipt, ValidatedBond, ValidatedCoupon, bond_active, coupon_expected,
    coupon_status, coupon_variance,
};
use crate::models::{Bond, BondCoupon, BondStatus, NewBond, NewBondCoupon};
use crate::store::{StoreError, now_timestamp, today};

/// `next_pay_date` comes from a scalar subquery on non-received coupons.
pub const BOND_SELECT: &str = "SELECT b.id, b.label, b.issue_no, b.principal, b.maturity_date, \
     b.received_at, b.note, b.sort_order, (SELECT MIN(c.pay_date) FROM bond_coupons c \
     WHERE c.bond_id = b.id AND c.received_amount IS NULL) AS next_pay_date FROM bonds b";

pub fn row_to_bond(row: &SqliteRow, today: chrono::NaiveDate) -> Result<Bond, StoreError> {
    let maturity_date: String = row.try_get("maturity_date")?;
    let parsed = chrono::NaiveDate::parse_from_str(&maturity_date, "%Y-%m-%d").map_err(|_| {
        StoreError::Corrupt(format!("stored maturity_date {maturity_date} is not valid"))
    })?;
    Ok(Bond {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        issue_no: row.try_get("issue_no")?,
        principal: row.try_get("principal")?,
        maturity_date,
        received_at: row.try_get("received_at")?,
        note: row.try_get("note")?,
        sort_order: row.try_get("sort_order")?,
        status: if bond_active(parsed, today) {
            BondStatus::Active
        } else {
            BondStatus::Matured
        },
        next_pay_date: row.try_get("next_pay_date")?,
    })
}

/// `principal` is joined from the parent bond so `expected` can be derived.
pub const COUPON_SELECT: &str = "SELECT c.id, c.bond_id, c.pay_date, c.fixing_date, \
     c.annual_rate, c.per_10k, c.received_amount, c.note, b.principal \
     FROM bond_coupons c JOIN bonds b ON b.id = c.bond_id";

pub fn row_to_coupon(row: &SqliteRow) -> Result<BondCoupon, StoreError> {
    let annual_rate: Option<f64> = row.try_get("annual_rate")?;
    let per_10k: Option<f64> = row.try_get("per_10k")?;
    let received_amount: Option<f64> = row.try_get("received_amount")?;
    let principal: f64 = row.try_get("principal")?;
    let expected = coupon_expected(per_10k, principal);
    Ok(BondCoupon {
        id: row.try_get("id")?,
        bond_id: row.try_get("bond_id")?,
        pay_date: row.try_get("pay_date")?,
        fixing_date: row.try_get("fixing_date")?,
        annual_rate,
        per_10k,
        received_amount,
        note: row.try_get("note")?,
        status: coupon_status(received_amount, annual_rate, per_10k),
        expected,
        variance: coupon_variance(received_amount, expected),
    })
}

/// `matured`: Some(false) lists only bonds still active today
/// (maturity_date > today), Some(true) only matured.
pub async fn list(
    pool: &SqlitePool,
    matured: Option<bool>,
    descending: bool,
) -> Result<Vec<Bond>, StoreError> {
    let mut sql = BOND_SELECT.to_string();
    let mut conditions: Vec<&str> = Vec::new();
    if let Some(matured) = matured {
        if matured {
            conditions.push("b.maturity_date <= ?");
        } else {
            conditions.push("b.maturity_date > ?");
        }
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(if descending {
        " ORDER BY b.maturity_date DESC, b.sort_order DESC, b.id DESC"
    } else {
        " ORDER BY b.maturity_date ASC, b.sort_order ASC, b.id ASC"
    });

    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if matured.is_some() {
        statement = statement.bind(today().to_string());
    }
    let today = today();
    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(|row| row_to_bond(row, today))
        .collect::<Result<Vec<_>, _>>()
}

pub async fn next_sort_order(pool: &SqlitePool) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM bonds")
            .fetch_one(pool)
            .await?,
    )
}

pub async fn insert_bond(
    pool: &SqlitePool,
    bond: &ValidatedBond,
    sort_order: i64,
    body: &NewBond,
) -> Result<i64, StoreError> {
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
) -> Result<i64, StoreError> {
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

pub async fn update_bond(
    pool: &SqlitePool,
    id: i64,
    validated: &ValidatedBond,
    note: &Option<String>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE bonds SET label = ?, issue_no = ?, principal = ?, maturity_date = ?, \
         note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&validated.label)
    .bind(validated.issue_no.as_deref())
    .bind(validated.principal)
    .bind(&validated.maturity_date)
    .bind(note)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_received(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    received_at: &str,
    credit_asset_id: Option<i64>,
    credited_amount: Option<f64>,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE bonds SET received_at = ?, credited_asset_id = ?, credited_amount = ?, \
         updated_at = ? WHERE id = ?",
    )
    .bind(received_at)
    .bind(credit_asset_id)
    .bind(credited_amount)
    .bind(now)
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The stored bank-in fields of a received bond: (credited_asset_id, credited_amount).
pub async fn credited_of(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<(Option<i64>, Option<f64>)>, StoreError> {
    Ok(
        sqlx::query_as("SELECT credited_asset_id, credited_amount FROM bonds WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn clear_receipt(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE bonds SET received_at = NULL, credited_asset_id = NULL, \
         credited_amount = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Deleting a bond removes its coupons: nothing outside the bond references
/// them. Returns the bond rows deleted, so the handler can tell "not found"
/// apart from success.
pub async fn delete_with_coupons(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
) -> Result<u64, StoreError> {
    sqlx::query("DELETE FROM bond_coupons WHERE bond_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(sqlx::query("DELETE FROM bonds WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?
        .rows_affected())
}

pub async fn update_coupon(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    bond_id: i64,
    validated: &ValidatedCoupon,
    note: &Option<String>,
    now: &str,
) -> Result<(), StoreError> {
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
    .bind(note)
    .bind(now)
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The stored credited_amount of a coupon, inside the caller's transaction.
pub async fn coupon_credited_amount(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
) -> Result<Option<f64>, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT credited_amount FROM bond_coupons WHERE id = ?")
            .bind(id)
            .fetch_one(&mut *conn)
            .await?,
    )
}

pub async fn set_coupon_credited(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    amount: Option<f64>,
) -> Result<(), StoreError> {
    match amount {
        Some(_) => {
            sqlx::query("UPDATE bond_coupons SET credited_amount = ? WHERE id = ?")
                .bind(amount)
                .bind(id)
                .execute(&mut *conn)
                .await?;
        }
        None => {
            sqlx::query("UPDATE bond_coupons SET credited_amount = NULL WHERE id = ?")
                .bind(id)
                .execute(&mut *conn)
                .await?;
        }
    }
    Ok(())
}

/// The bond's label, inside the caller's transaction (the coupon receipt's
/// month item carries it).
pub async fn bond_label(conn: &mut sqlx::SqliteConnection, id: i64) -> Result<String, StoreError> {
    Ok(sqlx::query_scalar("SELECT label FROM bonds WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *conn)
        .await?)
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub async fn delete_coupon(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM bond_coupons WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

pub async fn load_bond(pool: &SqlitePool, id: i64) -> Result<Bond, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{BOND_SELECT} WHERE b.id = ?")))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| StoreError::NotFound(format!("bond {id} not found")))?;
    row_to_bond(&row, today())
}

pub async fn load_all_bonds(pool: &SqlitePool) -> Result<Vec<Bond>, StoreError> {
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
) -> Result<Vec<BondCoupon>, StoreError> {
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

pub async fn list_coupons(
    pool: &SqlitePool,
    bond_id: Option<i64>,
) -> Result<Vec<BondCoupon>, StoreError> {
    let mut sql = COUPON_SELECT.to_string();
    if bond_id.is_some() {
        sql.push_str(" WHERE c.bond_id = ?");
    }
    sql.push_str(" ORDER BY c.pay_date, c.id");
    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(bond_id) = bond_id {
        statement = statement.bind(bond_id);
    }
    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_coupon)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn load_coupon(pool: &SqlitePool, id: i64) -> Result<BondCoupon, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{COUPON_SELECT} WHERE c.id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("coupon {id} not found")))?;
    row_to_coupon(&row)
}

/// Bond coupons paid in `range`, or all of them when `None` — received ones
/// carry their amount, pending ones the expected figure (None while 待定).
pub async fn suggestion_receipts(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionReceipt>, StoreError> {
    let mut sql = "SELECT c.id, c.pay_date, c.received_amount, c.per_10k, b.principal, b.label \
         FROM bond_coupons c JOIN bonds b ON b.id = c.bond_id"
        .to_string();
    if range.is_some() {
        sql.push_str(" WHERE c.pay_date >= ? AND c.pay_date < ?");
    }
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some((first, next)) = &range {
        query = query.bind(first).bind(next);
    }
    let rows = query.fetch_all(pool).await?;
    rows.iter()
        .map(|row| {
            let received_amount: Option<f64> = row.try_get("received_amount")?;
            let per_10k: Option<f64> = row.try_get("per_10k")?;
            let principal: f64 = row.try_get("principal")?;
            let pay_date: String = row.try_get("pay_date")?;
            Ok(SuggestionReceipt {
                id: row.try_get("id")?,
                pay_date: chrono::NaiveDate::parse_from_str(&pay_date, "%Y-%m-%d").map_err(
                    |_| StoreError::Corrupt(format!("stored date {pay_date} is not valid")),
                )?,
                amount: received_amount.or(crate::calc::coupon_expected(per_10k, principal)),
                label: row.try_get("label")?,
                received: received_amount.is_some(),
            })
        })
        .collect()
}

/// Bonds reduced to what the year review needs: principal, maturity, the
/// coupon schedule's first pay year (the purchase proxy), and each received
/// coupon's (pay year, amount).
pub async fn year_facts(pool: &SqlitePool) -> Result<Vec<BondYearFacts>, StoreError> {
    let rows = sqlx::query(
        "SELECT b.id, b.principal, b.maturity_date, c.pay_date, c.received_amount \
         FROM bonds b LEFT JOIN bond_coupons c ON c.bond_id = b.id \
         ORDER BY b.sort_order, b.id, c.pay_date",
    )
    .fetch_all(pool)
    .await?;

    let mut facts: Vec<BondYearFacts> = Vec::new();
    let mut last_id: Option<i64> = None;
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        if last_id != Some(id) {
            let maturity_date: String = row.try_get("maturity_date")?;
            facts.push(BondYearFacts {
                principal: row.try_get("principal")?,
                first_coupon_year: None,
                maturity_date: chrono::NaiveDate::parse_from_str(&maturity_date, "%Y-%m-%d")
                    .map_err(|_| {
                        StoreError::Corrupt(format!(
                            "stored maturity_date {maturity_date} is not valid"
                        ))
                    })?,
                received: Vec::new(),
            });
            last_id = Some(id);
        }
        let pay_date: Option<String> = row.try_get("pay_date")?;
        let Some(pay_date) = pay_date else { continue };
        let parsed = chrono::NaiveDate::parse_from_str(&pay_date, "%Y-%m-%d")
            .map_err(|_| StoreError::Corrupt(format!("stored pay_date {pay_date} is not valid")))?;
        let fact = facts.last_mut().expect("just pushed");
        let year = chrono::Datelike::year(&parsed);
        fact.first_coupon_year = Some(fact.first_coupon_year.map_or(year, |first| first.min(year)));
        if let Some(amount) = row.try_get::<Option<f64>, _>("received_amount")? {
            fact.received.push((year, amount));
        }
    }
    Ok(facts)
}
