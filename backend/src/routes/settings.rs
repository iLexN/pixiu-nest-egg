use axum::Json;
use axum::extract::State;

use super::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::models::{AppSettings, AppSettingsPatch};

#[utoipa::path(
    operation_id = "settings_get",
    get,
    path = "/api/settings",
    tag = "settings",
    responses(
        (status = 200, description = "Salary and current-year pool rate", body = AppSettings),
    )
)]
pub async fn get(State(state): State<AppState>) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(crate::service::settings::get(&state.pool).await?))
}

#[utoipa::path(
    operation_id = "settings_update",
    patch,
    path = "/api/settings",
    tag = "settings",
    request_body = AppSettingsPatch,
    responses(
        (status = 200, description = "Updated settings", body = AppSettings),
        (status = 400, description = "Invalid fields", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Json(patch): Json<AppSettingsPatch>,
) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(
        crate::service::settings::update(&state.pool, &patch).await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn update_settings_round_trips_the_semi_liquid_target() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };

        // While unset the effective ratio is the sheet's 25% literal.
        let current = get(State(state.clone())).await.expect("settings").0;
        assert_eq!(current.semi_liquid_target, 0.25);

        let updated = update(
            State(state.clone()),
            Json(AppSettingsPatch {
                semi_liquid_target: Some(Some(0.3)),
                ..Default::default()
            }),
        )
        .await
        .expect("patched")
        .0;
        assert_eq!(updated.semi_liquid_target, 0.3);
        assert_eq!(
            crate::service::settings::semi_liquid_target(&pool)
                .await
                .unwrap(),
            0.3
        );

        // `null` clears the stored value — back to the default.
        let reset = update(
            State(state.clone()),
            Json(AppSettingsPatch {
                semi_liquid_target: Some(None),
                ..Default::default()
            }),
        )
        .await
        .expect("reset")
        .0;
        assert_eq!(reset.semi_liquid_target, 0.25);

        // A share of 100%+ is meaningless for the buffer and rejected.
        let err = update(
            State(state),
            Json(AppSettingsPatch {
                semi_liquid_target: Some(Some(1.5)),
                ..Default::default()
            }),
        )
        .await
        .expect_err("rejected");
        assert!(matches!(err, ApiError::Validation(_)));
    }

    #[tokio::test]
    async fn update_settings_round_trips_the_calendar_private_id() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool };

        // Unset reads as `null`.
        let current = get(State(state.clone())).await.expect("settings").0;
        assert_eq!(current.calendar_private_id, None);

        // The stored ID is trimmed.
        let updated = update(
            State(state.clone()),
            Json(AppSettingsPatch {
                calendar_private_id: Some(Some(
                    " 7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com ".to_string(),
                )),
                ..Default::default()
            }),
        )
        .await
        .expect("patched")
        .0;
        assert_eq!(
            updated.calendar_private_id.as_deref(),
            Some("7a2rh1um3kbuelitfesq7aggko@group.calendar.google.com")
        );

        // A blank string clears it.
        let blanked = update(
            State(state.clone()),
            Json(AppSettingsPatch {
                calendar_private_id: Some(Some("".to_string())),
                ..Default::default()
            }),
        )
        .await
        .expect("blanked")
        .0;
        assert_eq!(blanked.calendar_private_id, None);

        // So does `null`.
        let set = update(
            State(state.clone()),
            Json(AppSettingsPatch {
                calendar_private_id: Some(Some("id@group.calendar.google.com".to_string())),
                ..Default::default()
            }),
        )
        .await
        .expect("patched")
        .0;
        assert_eq!(
            set.calendar_private_id.as_deref(),
            Some("id@group.calendar.google.com")
        );
        let cleared = update(
            State(state),
            Json(AppSettingsPatch {
                calendar_private_id: Some(None),
                ..Default::default()
            }),
        )
        .await
        .expect("cleared")
        .0;
        assert_eq!(cleared.calendar_private_id, None);
    }

    #[tokio::test]
    async fn update_settings_round_trips_the_goal_settings() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool };

        // The constants fall back to the sheet literals while unset.
        let current = get(State(state.clone())).await.expect("settings").0;
        assert_eq!(current.goal1_amount, None);
        assert_eq!(current.goal2_amount, None);
        assert_eq!(
            current.goal3_inflation,
            crate::service::settings::DEFAULT_GOAL3_INFLATION
        );
        assert_eq!(
            current.goal3_years,
            crate::service::settings::DEFAULT_GOAL3_YEARS
        );
        assert_eq!(
            current.final_replacement,
            crate::service::settings::DEFAULT_FINAL_REPLACEMENT
        );

        let updated = update(
            State(state.clone()),
            Json(AppSettingsPatch {
                goal1_amount: Some(Some(1_550_000.0)),
                goal3_inflation: Some(Some(1.05)),
                ..Default::default()
            }),
        )
        .await
        .expect("patched")
        .0;
        assert_eq!(updated.goal1_amount, Some(1_550_000.0));
        assert_eq!(updated.goal3_inflation, 1.05);
        assert_eq!(
            updated.goal3_years,
            crate::service::settings::DEFAULT_GOAL3_YEARS
        );

        // `null` clears an amount and resets a constant to its default.
        let reset = update(
            State(state.clone()),
            Json(AppSettingsPatch {
                goal1_amount: Some(None),
                goal3_inflation: Some(None),
                ..Default::default()
            }),
        )
        .await
        .expect("reset")
        .0;
        assert_eq!(reset.goal1_amount, None);
        assert_eq!(
            reset.goal3_inflation,
            crate::service::settings::DEFAULT_GOAL3_INFLATION
        );

        // Negative amounts and non-positive constants are rejected.
        for patch in [
            AppSettingsPatch {
                goal1_amount: Some(Some(-1.0)),
                ..Default::default()
            },
            AppSettingsPatch {
                goal3_years: Some(Some(0.0)),
                ..Default::default()
            },
            AppSettingsPatch {
                final_replacement: Some(Some(-0.5)),
                ..Default::default()
            },
        ] {
            let err = update(State(state.clone()), Json(patch))
                .await
                .expect_err("rejected");
            assert!(matches!(err, ApiError::Validation(_)));
        }
    }
}
