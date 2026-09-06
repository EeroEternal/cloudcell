use axum::{
    extract::{Request, State},
    http::{Method, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::SqlitePool;

use crate::api_key::hash_key;
use crate::error::Error;
use crate::state::AppState;

pub async fn require_api_key(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/health" || path == "/api/v1/ping" {
        return next.run(request).await;
    }

    if request.method() == Method::POST && path == "/api/v1/keys" {
        match count_keys(&state.db).await {
            Ok(0) => return next.run(request).await,
            Ok(_) => {}
            Err(err) => return err.into_response(),
        }
    }

    let Some(token) = bearer_token(request.headers()) else {
        return Error::Unauthorized("missing bearer token".into()).into_response();
    };

    match lookup_key(&state.db, token).await {
        Ok(true) => next.run(request).await,
        Ok(false) => Error::Unauthorized("invalid bearer token".into()).into_response(),
        Err(err) => err.into_response(),
    }
}

fn bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn count_keys(db: &SqlitePool) -> crate::error::Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys")
        .fetch_one(db)
        .await?;
    Ok(count)
}

async fn lookup_key(db: &SqlitePool, token: &str) -> crate::error::Result<bool> {
    let hash = hash_key(token);
    let found: Option<String> = sqlx::query_scalar("SELECT id FROM api_keys WHERE hash = ?")
        .bind(hash)
        .fetch_optional(db)
        .await?;
    Ok(found.is_some())
}
