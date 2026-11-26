//! `GET /api/export/analysis`: the AI-analysis snapshot as a file download.

use axum::Json;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;

use super::AppState;
use crate::error::ApiError;
use crate::service::export::AnalysisExport;

/// The one-shot analysis snapshot, fully derived on read.
#[utoipa::path(
    operation_id = "analysis_export",
    get,
    path = "/api/export/analysis",
    tag = "export",
    responses(
        (status = 200, description = "Analysis export JSON attachment", body = AnalysisExport),
    )
)]
pub async fn analysis(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let export = crate::service::export::build(&state.pool).await?;
    let disposition = format!(
        "attachment; filename=\"wealth-export-{}.json\"",
        export.meta.as_of_date
    );
    Ok(([(header::CONTENT_DISPOSITION, disposition)], Json(export)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use axum::http::header;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn analysis_returns_the_snapshot_as_an_attachment() {
        let pool = connect_memory().await.expect("memory db");
        let response = analysis(State(AppState { pool }))
            .await
            .expect("export")
            .into_response();

        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .expect("content-disposition")
            .to_str()
            .expect("utf-8");
        assert!(disposition.starts_with("attachment; filename=\"wealth-export-"));
        assert!(disposition.ends_with(".json\""));

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        for section in [
            "meta",
            "liquid_assets_summary",
            "cash_flow_momentum",
            "upcoming_cash_flow_and_rigid_expenses",
            "equity_holdings",
        ] {
            assert!(json.get(section).is_some(), "missing {section}");
        }
    }
}
