use sqlx::SqlitePool;

use crate::calc::FieldError;
use crate::models::{ManualAsset, ManualAssetPatch, NewManualAsset};
use crate::patch::merge_text;
use crate::service::ServiceError;
use crate::store::manual_assets;

fn validate(label: &str, amount: f64) -> Result<String, ServiceError> {
    let mut errors = Vec::new();
    let label = label.trim();
    if label.is_empty() {
        errors.push(FieldError::new("label", "label is required"));
    }
    if !amount.is_finite() {
        errors.push(FieldError::new("amount", "amount must be a number"));
    }
    if errors.is_empty() {
        Ok(label.to_string())
    } else {
        Err(ServiceError::Validation(errors))
    }
}

pub async fn create(pool: &SqlitePool, body: &NewManualAsset) -> Result<ManualAsset, ServiceError> {
    let label = validate(&body.label, body.amount)?;
    let sort_order = manual_assets::next_sort_order(pool).await?;
    let note = body
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty());
    let id = manual_assets::insert(
        pool,
        &label,
        body.liquidity.unwrap_or_default().as_str(),
        body.amount,
        note,
        sort_order,
    )
    .await?;
    Ok(manual_assets::load_one(pool, id).await?)
}

/// `Err(NotFound)` while `id` names no asset.
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    patch: &ManualAssetPatch,
) -> Result<ManualAsset, ServiceError> {
    let existing = manual_assets::load_one(pool, id).await?;
    let label = patch.label.clone().unwrap_or(existing.label);
    let liquidity = patch.liquidity.unwrap_or(existing.liquidity);
    let amount = patch.amount.unwrap_or(existing.amount);
    let label = validate(&label, amount)?;
    let note = merge_text(patch.note.clone(), existing.note);
    manual_assets::update(
        pool,
        id,
        &label,
        liquidity.as_str(),
        amount,
        note.as_deref(),
    )
    .await?;
    Ok(manual_assets::load_one(pool, id).await?)
}

/// `Err(NotFound)` while `id` names no asset.
pub async fn remove(pool: &SqlitePool, id: i64) -> Result<(), ServiceError> {
    if manual_assets::delete(pool, id).await? == 0 {
        return Err(ServiceError::NotFound(format!(
            "manual asset {id} not found"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::models::ManualAssetLiquidity;

    #[tokio::test]
    async fn create_defaults_liquidity_to_long() {
        let pool = connect_memory().await.expect("memory db");
        let created = create(
            &pool,
            &NewManualAsset {
                label: "HS人壽".to_string(),
                liquidity: None,
                amount: 69440.47,
                note: None,
            },
        )
        .await
        .expect("created");
        assert_eq!(created.liquidity, ManualAssetLiquidity::Long);
    }

    #[tokio::test]
    async fn update_reclassifies_liquidity() {
        let pool = connect_memory().await.expect("memory db");
        let created = create(
            &pool,
            &NewManualAsset {
                label: "Irene".to_string(),
                liquidity: None,
                amount: 20000.0,
                note: None,
            },
        )
        .await
        .expect("created");

        let updated = update(
            &pool,
            created.id,
            &ManualAssetPatch {
                liquidity: Some(ManualAssetLiquidity::Short),
                ..Default::default()
            },
        )
        .await
        .expect("patched");
        assert_eq!(updated.liquidity, ManualAssetLiquidity::Short);

        let stored = manual_assets::load_one(&pool, created.id)
            .await
            .expect("stored");
        assert_eq!(stored.liquidity, ManualAssetLiquidity::Short);
    }

    #[tokio::test]
    async fn update_note_set_kept_and_cleared() {
        let pool = connect_memory().await.expect("memory db");
        let created = create(
            &pool,
            &NewManualAsset {
                label: "HS人壽".to_string(),
                liquidity: None,
                amount: 69440.47,
                note: None,
            },
        )
        .await
        .expect("created");
        assert_eq!(created.note, None);

        // Setting a note stores it, trimmed.
        let noted = update(
            &pool,
            created.id,
            &ManualAssetPatch {
                note: Some(Some("  保單 123, 2026-10 估值  ".to_string())),
                ..Default::default()
            },
        )
        .await
        .expect("patched");
        assert_eq!(noted.note.as_deref(), Some("保單 123, 2026-10 估值"));

        // An amount-only patch leaves the note untouched.
        let reamounted = update(
            &pool,
            created.id,
            &ManualAssetPatch {
                amount: Some(70000.0),
                ..Default::default()
            },
        )
        .await
        .expect("patched");
        assert_eq!(reamounted.note.as_deref(), Some("保單 123, 2026-10 估值"));

        // `null` clears it.
        let cleared = update(
            &pool,
            created.id,
            &ManualAssetPatch {
                note: Some(None),
                ..Default::default()
            },
        )
        .await
        .expect("patched");
        assert_eq!(cleared.note, None);

        // So does a whitespace-only value.
        let reset = update(
            &pool,
            created.id,
            &ManualAssetPatch {
                note: Some(Some("保單 456".to_string())),
                ..Default::default()
            },
        )
        .await
        .expect("patched");
        assert_eq!(reset.note.as_deref(), Some("保單 456"));
        let blanked = update(
            &pool,
            created.id,
            &ManualAssetPatch {
                note: Some(Some("   ".to_string())),
                ..Default::default()
            },
        )
        .await
        .expect("patched");
        assert_eq!(blanked.note, None);
    }

    #[tokio::test]
    async fn create_collects_field_errors() {
        let pool = connect_memory().await.expect("memory db");
        let err = create(
            &pool,
            &NewManualAsset {
                label: "  ".to_string(),
                liquidity: None,
                amount: f64::NAN,
                note: None,
            },
        )
        .await
        .expect_err("rejected");
        match err {
            ServiceError::Validation(fields) => assert_eq!(fields.len(), 2),
            other => panic!("expected validation, got {other}"),
        }
    }
}
