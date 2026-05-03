use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::{auth, handlers, state::AppState};

pub fn create_router(state: AppState) -> Router {
    let protected = Router::new()
        .route(
            "/settings",
            post(handlers::settings::create_setting).get(handlers::settings::list_settings),
        )
        .route(
            "/settings/search",
            get(handlers::search::search_settings),
        )
        .route(
            "/settings/bulk",
            post(handlers::search::bulk_action),
        )
        .route(
            "/settings/:key",
            get(handlers::settings::get_setting)
                .patch(handlers::settings::update_setting)
                .delete(handlers::settings::delete_setting),
        )
        .route(
            "/settings/:key/history",
            get(handlers::revisions::list_history),
        )
        .route("/audit", get(handlers::revisions::list_audit))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_internal_auth,
        ));

    Router::new()
        .route("/health", get(handlers::health))
        .merge(protected)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
