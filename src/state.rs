use sqlx::PgPool;

use crate::auth::JwksCache;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwks: JwksCache,
}
