use axum::{
    extract::{FromRequestParts, Request, State},
    http::{Method, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::account;
use crate::api_key::hash_key;
use crate::error::Error;
use crate::state::AppState;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> std::result::Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or_else(|| Error::Unauthorized("missing bearer token".into()))
    }
}

pub async fn require_api_key(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if is_public(request.method(), path) {
        return next.run(request).await;
    }

    let Some(token) = extract_token(&request) else {
        return Error::Unauthorized("missing bearer token".into()).into_response();
    };

    match resolve_user(&state, &token).await {
        Ok(Some(user)) => {
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        Ok(None) => Error::Unauthorized("invalid bearer token".into()).into_response(),
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

fn extract_token(request: &Request) -> Option<String> {
    if let Some(token) = bearer_token(request.headers()) {
        return Some(token.to_string());
    }
    if let Some(query) = request.uri().query() {
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=')
                && k == "token"
                && !v.is_empty()
            {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn resolve_user(state: &AppState, token: &str) -> crate::error::Result<Option<AuthUser>> {
    if token.starts_with("cc_sess_") {
        let id = account::session_user_id(state, token).await?;
        return Ok(id.map(|id| AuthUser { id }));
    }
    let hash = hash_key(token);
    let id: Option<String> =
        sqlx::query_scalar("SELECT user_id FROM api_keys WHERE hash = ? AND user_id IS NOT NULL")
            .bind(hash)
            .fetch_optional(&state.db)
            .await?;
    Ok(id.map(|id| AuthUser { id }))
}
