use axum::{
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::{handlers, state::AppState};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route(
            "/settings",
            post(handlers::settings::create_setting).get(handlers::settings::list_settings),
        )
        .route(
            "/settings/:key",
            get(handlers::settings::get_setting)
                .patch(handlers::settings::update_setting)
                .delete(handlers::settings::delete_setting),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
