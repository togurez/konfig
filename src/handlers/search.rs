use axum::{
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};

use crate::{
    auth::Claims,
    db,
    error::AppError,
    models::search::{BulkAction, BulkRequest, SearchQuery},
    state::AppState,
};

pub async fn search_settings(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page = db::search::search_settings(&state.db, &query).await?;
    Ok(Json(page))
}

pub async fn bulk_action(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
    Json(req): Json<BulkRequest>,
) -> Result<impl IntoResponse, AppError> {
    if matches!(req.action, BulkAction::Delete) {
        let confirmed = headers
            .get("X-Confirm-Bulk-Delete")
            .and_then(|v| v.to_str().ok())
            == Some("true");
        if !confirmed {
            return Err(AppError::ValidationError(
                "Bulk delete requires header 'X-Confirm-Bulk-Delete: true'".to_string(),
            ));
        }
    }

    let result = db::search::bulk_action(&state.db, &req, &claims.sub).await?;
    Ok(Json(result))
}
