use axum::{
    extract::{Request, State},
    http::{Method, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::account;
use crate::api_key::hash_key;
use crate::error::Error;
use crate::state::AppState;

pub async fn require_api_key(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if is_public(request.method(), path) {
        return next.run(request).await;
    }

    let Some(token) = bearer_token(request.headers()) else {
        return Error::Unauthorized("missing bearer token".into()).into_response();
    };

    match is_authorized(&state, token).await {
        Ok(true) => next.run(request).await,
        Ok(false) => Error::Unauthorized("invalid bearer token".into()).into_response(),
        Err(err) => err.into_response(),
    }
}

fn is_public(method: &Method, path: &str) -> bool {
    matches!(
        (method, path),
        (&Method::GET, "/health")
            | (&Method::GET, "/api/v1/ping")
            | (&Method::GET, "/api/v1/auth/status")
            | (&Method::POST, "/api/v1/auth/register")
            | (&Method::POST, "/api/v1/auth/login")
            | (&Method::POST, "/api/v1/auth/send-code")
            | (&Method::POST, "/api/v1/auth/verify-code")
    )
}

fn bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn is_authorized(state: &AppState, token: &str) -> crate::error::Result<bool> {
    if token.starts_with("cc_sess_") {
        return account::session_valid(state, token).await;
    }
    let hash = hash_key(token);
    let found: Option<String> = sqlx::query_scalar("SELECT id FROM api_keys WHERE hash = ?")
        .bind(hash)
        .fetch_optional(&state.db)
        .await?;
    Ok(found.is_some())
}
