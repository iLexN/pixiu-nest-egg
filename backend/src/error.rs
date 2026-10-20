use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::calc::FieldError;
use crate::service::ServiceError;
use crate::store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("validation failed")]
    Validation(Vec<FieldError>),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl ApiError {
    pub fn field(field: &str, message: impl Into<String>) -> Self {
        ApiError::Validation(vec![FieldError::new(field, message)])
    }
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Database(err) => ApiError::Database(err),
            StoreError::NotFound(message) => ApiError::NotFound(message),
            StoreError::Corrupt(message) => ApiError::Conflict(message),
            StoreError::Conflict(message) => ApiError::Conflict(message),
        }
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Store(err) => ApiError::from(err),
            ServiceError::Validation(fields) => ApiError::Validation(fields),
            ServiceError::NotFound(message) => ApiError::NotFound(message),
            ServiceError::Conflict(message) => ApiError::Conflict(message),
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub error: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldError>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message, fields) = match self {
            ApiError::Validation(fields) => {
                let message = fields
                    .iter()
                    .map(|f| format!("{}: {}", f.field, f.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                (StatusCode::BAD_REQUEST, "validation", message, fields)
            }
            ApiError::NotFound(message) => {
                (StatusCode::NOT_FOUND, "not_found", message, Vec::new())
            }
            ApiError::Conflict(message) => (StatusCode::CONFLICT, "conflict", message, Vec::new()),
            ApiError::Database(err) => {
                if is_unique_violation(&err) {
                    (
                        StatusCode::CONFLICT,
                        "conflict",
                        "a record with the same unique key already exists".to_string(),
                        Vec::new(),
                    )
                } else {
                    tracing::error!("database error: {err}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal",
                        "database error".to_string(),
                        Vec::new(),
                    )
                }
            }
        };

        (
            status,
            Json(ErrorBody {
                error,
                message,
                fields,
            }),
        )
            .into_response()
    }
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db) if db.is_unique_violation())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use axum::response::IntoResponse;

    #[test]
    fn store_not_found_maps_to_404() {
        let response =
            ApiError::from(StoreError::NotFound("stock 42 not found".into())).into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn store_corrupt_maps_to_409() {
        let response = ApiError::from(StoreError::Corrupt("stored market XX is not valid".into()))
            .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn store_database_maps_through_and_unique_violation_stays_409() {
        let pool = connect_memory().await.expect("memory db");
        sqlx::query("CREATE TABLE uniq (k TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .expect("create table");
        sqlx::query("INSERT INTO uniq (k) VALUES ('a')")
            .execute(&pool)
            .await
            .expect("seed");
        let err = sqlx::query("INSERT INTO uniq (k) VALUES ('a')")
            .execute(&pool)
            .await
            .expect_err("unique violation");
        let response = ApiError::from(StoreError::Database(err)).into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
