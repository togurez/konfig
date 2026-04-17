use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    ValidationError(String),
    #[error("{0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("{0}")]
    InternalError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
            AppError::ValidationError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation_error", msg.clone())
            }
            AppError::DatabaseError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database_error", e.to_string())
            }
            AppError::InternalError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", e.to_string())
            }
        };
        (
            status,
            Json(json!({ "error": error_code, "message": message })),
        )
            .into_response()
    }
}
