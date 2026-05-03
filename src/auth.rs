use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::state::AppState;

/// Injected into request extensions so handlers can read the caller's identity.
#[derive(Clone)]
pub struct CallerSub(pub String);

pub async fn require_internal_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let secret = request
        .headers()
        .get("X-Internal-Token")
        .and_then(|v| v.to_str().ok());

    if secret != Some(state.config.internal_api_secret.as_str()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized", "message": "Invalid or missing X-Internal-Token" })),
        )
            .into_response();
    }

    // Extract caller sub for audit fields — present on mutating requests.
    if let Some(sub) = request
        .headers()
        .get("X-Caller-Sub")
        .and_then(|v| v.to_str().ok())
    {
        request.extensions_mut().insert(CallerSub(sub.to_string()));
    }

    next.run(request).await
}
