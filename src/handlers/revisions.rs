use axum::{
    extract::{Path, Query, State},
    http::HeaderValue,
    response::IntoResponse,
    Json,
};

use crate::{
    db,
    error::AppError,
    models::revision::{AuditQuery, HistoryQuery},
    state::AppState,
};

pub async fn list_history(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);

    let (revisions, total) = db::revisions::list_history(&state.db, &key, page, per_page).await?;

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "X-Total-Count",
        HeaderValue::from_str(&total.to_string()).unwrap(),
    );
    headers.insert(
        "Access-Control-Expose-Headers",
        HeaderValue::from_static("X-Total-Count"),
    );

    Ok((headers, Json(revisions)))
}

pub async fn list_audit(
    State(state): State<AppState>,
    Query(query): Query<AuditQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page = db::revisions::list_audit(&state.db, &query).await?;
    Ok(Json(page))
}
