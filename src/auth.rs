use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;

use crate::state::AppState;

#[derive(Debug, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iss: String,
    pub scope: Option<String>,
}

#[derive(Clone)]
pub struct JwksCache {
    inner: Arc<RwLock<Option<JwkSet>>>,
    pub domain: String,
    pub audience: String,
}

impl JwksCache {
    pub fn new(domain: String, audience: String) -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
            domain,
            audience,
        }
    }

    async fn fetch_jwks(&self) -> anyhow::Result<JwkSet> {
        let url = format!("https://{}/.well-known/jwks.json", self.domain);
        let jwks = reqwest::get(&url).await?.json::<JwkSet>().await?;
        Ok(jwks)
    }

    async fn get_decoding_key(&self, kid: &str) -> anyhow::Result<DecodingKey> {
        {
            let cache = self.inner.read().await;
            if let Some(jwks) = cache.as_ref() {
                if let Some(key) = find_key(jwks, kid) {
                    return Ok(key);
                }
            }
        }

        let jwks = self.fetch_jwks().await?;
        let key = find_key(&jwks, kid)
            .ok_or_else(|| anyhow::anyhow!("No JWK found for kid: {}", kid))?;

        *self.inner.write().await = Some(jwks);
        Ok(key)
    }
}

fn find_key(jwks: &JwkSet, kid: &str) -> Option<DecodingKey> {
    jwks.find(kid)
        .and_then(|jwk| DecodingKey::from_jwk(jwk).ok())
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    match validate_bearer(&state.jwks, request.headers()).await {
        Ok(claims) => {
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(resp) => resp,
    }
}

async fn validate_bearer(
    jwks: &JwksCache,
    headers: &axum::http::HeaderMap,
) -> Result<Claims, Response> {
    let auth_value = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| unauthorized("Missing Authorization header"))?;

    let token = auth_value
        .strip_prefix("Bearer ")
        .ok_or_else(|| unauthorized("Authorization header must use Bearer scheme"))?;

    let header =
        decode_header(token).map_err(|_| unauthorized("Malformed token header"))?;

    let kid = header
        .kid
        .ok_or_else(|| unauthorized("Token missing kid claim"))?;

    let decoding_key = jwks
        .get_decoding_key(&kid)
        .await
        .map_err(|_| unauthorized("Unable to retrieve signing key"))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[&jwks.audience]);
    validation.set_issuer(&[format!("https://{}/", jwks.domain)]);

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|_| unauthorized("Invalid or expired token"))
}

fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "unauthorized", "message": message })),
    )
        .into_response()
}
