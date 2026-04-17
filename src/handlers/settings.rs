use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use validator::Validate;

use crate::{
    db,
    error::AppError,
    models::setting::{CreateSettingRequest, ListSettingsQuery, Setting, UpdateSettingRequest},
    state::AppState,
};

pub async fn create_setting(
    State(state): State<AppState>,
    Json(req): Json<CreateSettingRequest>,
) -> Result<(StatusCode, Json<Setting>), AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;
    let setting = db::settings::insert_setting(&state.db, &req).await?;
    Ok((StatusCode::CREATED, Json(setting)))
}

pub async fn list_settings(
    State(state): State<AppState>,
    Query(query): Query<ListSettingsQuery>,
) -> Result<Json<Vec<Setting>>, AppError> {
    let settings = db::settings::list_settings(&state.db, &query).await?;
    Ok(Json(settings))
}

pub async fn get_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<Setting>, AppError> {
    let setting = db::settings::get_setting_by_key(&state.db, &key)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Setting '{key}' does not exist")))?;
    Ok(Json(setting))
}

pub async fn update_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<Json<Setting>, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;
    let setting = db::settings::update_setting(&state.db, &key, &req)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Setting '{key}' does not exist")))?;
    Ok(Json(setting))
}

pub async fn delete_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<StatusCode, AppError> {
    let deleted = db::settings::delete_setting(&state.db, &key).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Setting '{key}' does not exist")))
    }
}
