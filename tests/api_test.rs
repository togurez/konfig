use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

use konfig::{routes::create_router, state::AppState};

async fn body_json(body: axum::body::Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn test_health(pool: PgPool) {
    let app = create_router(AppState { db: pool });
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["db"], "ok");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_setting(pool: PgPool) {
    let app = create_router(AppState { db: pool });
    let response = app
        .oneshot(json_request(
            "POST",
            "/settings",
            json!({
                "key": "feature.dark_mode",
                "setting_type": "feature_flag",
                "value": { "enabled": true, "rollout_percentage": 50 },
                "description": "Enable dark mode",
                "is_active": true
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["key"], "feature.dark_mode");
    assert_eq!(json["setting_type"], "feature_flag");
    assert!(json["id"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_duplicate_key_returns_409(pool: PgPool) {
    let app = create_router(AppState { db: pool.clone() });
    let body = json!({
        "key": "dup.key",
        "setting_type": "custom",
        "value": 1
    });

    app.oneshot(json_request("POST", "/settings", body.clone()))
        .await
        .unwrap();

    let app2 = create_router(AppState { db: pool });
    let response = app2
        .oneshot(json_request("POST", "/settings", body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["error"], "conflict");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_missing_key_returns_404(pool: PgPool) {
    let app = create_router(AppState { db: pool });
    let response = app
        .oneshot(
            Request::builder()
                .uri("/settings/no.such.key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = body_json(response.into_body()).await;
    assert_eq!(json["error"], "not_found");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_fetch_update_delete_lifecycle(pool: PgPool) {
    let key = "lifecycle.test";

    // Create
    let app = create_router(AppState { db: pool.clone() });
    let r = app
        .oneshot(json_request(
            "POST",
            "/settings",
            json!({ "key": key, "setting_type": "custom", "value": "v1" }),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::CREATED);

    // Fetch
    let app = create_router(AppState { db: pool.clone() });
    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("/settings/{key}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let json = body_json(r.into_body()).await;
    assert_eq!(json["value"], "v1");

    // Update
    let app = create_router(AppState { db: pool.clone() });
    let r = app
        .oneshot(json_request(
            "PATCH",
            &format!("/settings/{key}"),
            json!({ "value": "v2", "is_active": false }),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let json = body_json(r.into_body()).await;
    assert_eq!(json["value"], "v2");
    assert_eq!(json["is_active"], false);

    // Delete
    let app = create_router(AppState { db: pool.clone() });
    let r = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/settings/{key}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);

    // Gone
    let app = create_router(AppState { db: pool });
    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("/settings/{key}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}
