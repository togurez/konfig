pub mod settings;

use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{error::AppError, state::AppState};

pub async fn health(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(AppError::DatabaseError)?;
    Ok(Json(json!({ "status": "ok", "db": "ok" })))
}
