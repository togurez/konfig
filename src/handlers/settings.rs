use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use validator::Validate;

use crate::{
    auth::CallerSub,
    db,
    error::AppError,
    models::setting::{CreateSettingRequest, ListSettingsQuery, Setting, UpdateSettingRequest},
    state::AppState,
};

pub async fn create_setting(
    State(state): State<AppState>,
    Extension(caller): Extension<CallerSub>,
    Json(req): Json<CreateSettingRequest>,
) -> Result<(StatusCode, Json<Setting>), AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;
    let setting = db::settings::insert_setting(&state.db, &req, &caller.0).await?;
    Ok((StatusCode::CREATED, Json(setting)))
}

pub async fn list_settings(
    State(state): State<AppState>,
    Query(query): Query<ListSettingsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let (settings, total) = tokio::try_join!(
        db::settings::list_settings(&state.db, &query),
        db::settings::count_settings(&state.db, &query),
    )?;

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("X-Total-Count", HeaderValue::from_str(&total.to_string()).unwrap());
    headers.insert(
        "Access-Control-Expose-Headers",
        HeaderValue::from_static("X-Total-Count"),
    );

    Ok((headers, Json(settings)))
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
    Extension(caller): Extension<CallerSub>,
    Path(key): Path<String>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<Json<Setting>, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;
    let setting = db::settings::update_setting(&state.db, &key, &req, &caller.0)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Setting '{key}' does not exist")))?;
    Ok(Json(setting))
}

pub async fn delete_setting(
    State(state): State<AppState>,
    Extension(caller): Extension<CallerSub>,
    Path(key): Path<String>,
) -> Result<StatusCode, AppError> {
    let deleted = db::settings::delete_setting(&state.db, &key, &caller.0).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Setting '{key}' does not exist")))
    }
}
