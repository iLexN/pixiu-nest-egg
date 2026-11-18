use sqlx::SqlitePool;

use crate::calc::FieldError;
use crate::models::{
    MonthItem, MonthItemCategory, MonthItemPatch, MonthStat, MonthStatPatch, NewMonthItem,
};
use crate::patch::merge_text;
use crate::service::ServiceError;
use crate::store::{StoreError, now_timestamp};

use super::book::MonthBook;

/// Upsert a month row. Creating snapshots the live totals and the current
/// salary; explicit `null` on a total keeps it live, `recapture` re-snapshots
/// the totals plus 月初 (the live 活期 cash sum). Returns the row with its
/// derived columns.
pub async fn upsert(
    pool: &SqlitePool,
    month: chrono::NaiveDate,
    patch: &MonthStatPatch,
) -> Result<MonthStat, ServiceError> {
    let month_key = month.to_string();
    let existing = crate::store::months::load_month(pool, &month_key).await?;
    let now = now_timestamp();

    match existing {
        None => {
            // Body-supplied totals win; otherwise snapshot the live values.
            let live = crate::calc::live_totals(&crate::service::wealth::totals_input(pool).await?);
            let total_assets = patch.total_assets.unwrap_or(Some(live.total_assets));
            let liquid_assets = patch.liquid_assets.unwrap_or(Some(live.liquid_assets));
            let salary = match patch.salary {
                Some(salary) => Some(salary),
                None => crate::store::settings::Settings::load(pool).await?.salary,
            };
            crate::store::months::insert_month(
                pool,
                &month_key,
                patch.start_cash,
                salary,
                total_assets,
                liquid_assets,
                patch.pool_input.unwrap_or(0.0),
                patch.end_cash_override.flatten(),
                patch.note.clone().flatten(),
                &now,
            )
            .await?;
        }
        Some(existing) => {
            // A present field wins over recapture, recapture wins over the
            // stored value, and `null` stores NULL (live).
            let input = if patch.recapture == Some(true) {
                Some(crate::service::wealth::totals_input(pool).await?)
            } else {
                None
            };
            let live = input.map(|input| crate::calc::live_totals(&input));
            let start_cash = patch
                .start_cash
                .or_else(|| input.map(|input| input.cash_sum))
                .or(existing.start_cash);
            let total_assets = match (patch.total_assets, live) {
                (Some(value), _) => value,
                (None, Some(live)) => Some(live.total_assets),
                (None, None) => existing.total_assets,
            };
            let liquid_assets = match (patch.liquid_assets, live) {
                (Some(value), _) => value,
                (None, Some(live)) => Some(live.liquid_assets),
                (None, None) => existing.liquid_assets,
            };
            crate::store::months::update_month(
                pool,
                &month_key,
                start_cash,
                patch.salary.or(existing.salary),
                total_assets,
                liquid_assets,
                patch.pool_input.unwrap_or(existing.pool_input),
                patch
                    .end_cash_override
                    .unwrap_or(existing.end_cash_override),
                merge_text(patch.note.clone(), existing.note),
                &now,
            )
            .await?;
        }
    }

    MonthBook::open(pool)
        .await?
        .month(&month_key)
        .ok_or_else(|| ServiceError::NotFound(format!("month {month_key} not found")))
}

/// Insert a month item: accepting a suggestion (any `auto_key`) clears its
/// dismissal tombstone in the same transaction, so it can't resurface.
/// `Err(NotFound)` while `month_key` names no month; `Err(Validation)` when
/// `exclude_from_living` rides a non-娛樂支出 item. A duplicate
/// (month, auto_key) hits the partial unique index → `Err(Conflict)`.
pub async fn create_item(
    pool: &SqlitePool,
    month_key: &str,
    body: &NewMonthItem,
) -> Result<MonthItem, ServiceError> {
    if crate::store::months::load_month(pool, month_key)
        .await?
        .is_none()
    {
        return Err(ServiceError::NotFound(format!(
            "month {month_key} not found"
        )));
    }
    if body.exclude_from_living && body.category != MonthItemCategory::Entertainment {
        return Err(ServiceError::Validation(vec![FieldError::new(
            "exclude_from_living",
            "exclude_from_living only applies to 娛樂支出 items",
        )]));
    }
    let label = body
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_string);
    let auto_key = body
        .auto_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string);
    let note = body
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
        .map(str::to_string);

    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    if let Some(auto_key) = &auto_key {
        crate::store::months::clear_dismissal(&mut tx, month_key, auto_key).await?;
    }
    let id = crate::store::months::insert_item(
        &mut tx,
        month_key,
        body.category.as_str(),
        &label,
        body.amount,
        body.exclude_from_living,
        &auto_key,
        &note,
    )
    .await?;
    tx.commit().await.map_err(StoreError::from)?;

    Ok(crate::store::months::item_row(pool, id).await?)
}

/// Update a month item. The `exclude_from_living` flag is entertainment-only:
/// moving an item to another category clears it, while setting it on a
/// non-entertainment item is `Err(Validation)`.
pub async fn update_item(
    pool: &SqlitePool,
    id: i64,
    patch: &MonthItemPatch,
) -> Result<MonthItem, ServiceError> {
    let existing = crate::store::months::load_item(pool, id).await?;

    let category = patch.category.unwrap_or(existing.category);
    let label = merge_text(patch.label.clone(), existing.label);
    let amount = patch.amount.unwrap_or(existing.amount);
    let note = merge_text(patch.note.clone(), existing.note);
    if !amount.is_finite() {
        return Err(ServiceError::Validation(vec![FieldError::new(
            "amount",
            "amount must be a number",
        )]));
    }
    let exclude_from_living = match (patch.exclude_from_living, category) {
        (Some(true), MonthItemCategory::Entertainment) => true,
        (Some(true), _) => {
            return Err(ServiceError::Validation(vec![FieldError::new(
                "exclude_from_living",
                "exclude_from_living only applies to 娛樂支出 items",
            )]));
        }
        (Some(false), _) => false,
        (None, MonthItemCategory::Entertainment) => existing.exclude_from_living,
        (None, _) => false,
    };

    crate::store::months::update_item(
        pool,
        id,
        category.as_str(),
        &label,
        amount,
        exclude_from_living,
        &note,
    )
    .await?;

    Ok(crate::store::months::item_row(pool, id).await?)
}

#[cfg(test)]
mod tests {
    use super::super::book::tests::{insert, month_key};
    use super::super::live_from;
    use super::*;

    fn empty_patch() -> MonthStatPatch {
        MonthStatPatch {
            start_cash: None,
            salary: None,
            total_assets: None,
            liquid_assets: None,
            pool_input: None,
            end_cash_override: None,
            note: None,
            recapture: None,
        }
    }

    fn item(category: MonthItemCategory, exclude: bool, auto_key: Option<&str>) -> NewMonthItem {
        NewMonthItem {
            category,
            label: None,
            amount: 5.0,
            exclude_from_living: exclude,
            auto_key: auto_key.map(str::to_string),
            note: None,
        }
    }

    #[tokio::test]
    async fn upsert_create_snapshots_the_live_totals() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let stat = upsert(&pool, live_from(), &empty_patch())
            .await
            .expect("create");
        // Empty DB → live totals are 0.0, snapshotted as stored values.
        assert_eq!(stat.total_assets, Some(0.0));
        assert_eq!(stat.liquid_assets, Some(0.0));
    }

    #[tokio::test]
    async fn create_item_clears_the_dismissal_and_rejects_flag_on_non_entertainment() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let first = live_from();
        insert(&pool, first, Some(1.0), Some(1.0)).await;
        let month_key = month_key(first);
        crate::store::months::dismiss_item(&pool, &month_key, "dep-start:1")
            .await
            .expect("dismiss");

        let reject = create_item(
            &pool,
            &month_key,
            &item(MonthItemCategory::Income, true, None),
        )
        .await;
        assert!(matches!(reject, Err(ServiceError::Validation(_))));

        create_item(
            &pool,
            &month_key,
            &item(MonthItemCategory::ExtraSpend, false, Some("dep-start:1")),
        )
        .await
        .expect("accept");
        let dismissed = crate::store::months::dismissed_keys(&pool, &month_key)
            .await
            .expect("keys");
        assert!(!dismissed.contains("dep-start:1"));
    }

    #[tokio::test]
    async fn update_item_category_change_clears_the_entertainment_flag() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let first = live_from();
        insert(&pool, first, Some(1.0), Some(1.0)).await;
        let month_key = month_key(first);
        let item = create_item(
            &pool,
            &month_key,
            &item(MonthItemCategory::Entertainment, true, None),
        )
        .await
        .expect("item");
        assert!(item.exclude_from_living);

        let moved = update_item(
            &pool,
            item.id,
            &MonthItemPatch {
                category: Some(MonthItemCategory::Income),
                ..Default::default()
            },
        )
        .await
        .expect("move");
        assert!(!moved.exclude_from_living);

        let flag = update_item(
            &pool,
            item.id,
            &MonthItemPatch {
                exclude_from_living: Some(true),
                ..Default::default()
            },
        )
        .await;
        assert!(matches!(flag, Err(ServiceError::Validation(_))));
    }
}
