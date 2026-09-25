use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::calc::FieldError;

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
