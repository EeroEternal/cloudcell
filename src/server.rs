use axum::{
    Json, Router,
    http::{HeaderValue, Method, header},
    middleware,
    routing::{get, post},
};
use serde_json::{Value, json};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::account;
use crate::api_key;
use crate::auth;
use crate::config::Config;
use crate::sandbox;
use crate::snapshot;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = cors_layer(&state.config);

    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/ping", get(ping))
        .route(
            "/api/v1/sandboxes",
            get(sandbox::list_sandboxes).post(sandbox::create_sandbox),
        )
        .route(
            "/api/v1/sandboxes/{id}",
            get(sandbox::get_sandbox).delete(sandbox::delete_sandbox),
        )
        .route("/api/v1/sandboxes/{id}/exec", post(sandbox::exec_sandbox))
        .route(
            "/api/v1/sandboxes/{id}/stream",
            get(sandbox::stream_sandbox),
        )
        .route("/api/v1/sandboxes/{id}/acp", get(sandbox::stream_sandbox))
        .route("/api/v1/snapshots", get(snapshot::list_snapshots))
        .route("/api/v1/auth/status", get(account::status))
        .route("/api/v1/auth/register", post(account::register))
        .route("/api/v1/auth/login", post(account::login))
        .route("/api/v1/auth/send-code", post(account::send_code))
        .route("/api/v1/auth/verify-code", post(account::verify_code))
        .route("/api/v1/auth/logout", post(account::logout))
        .route(
            "/api/v1/settings",
            get(account::get_settings).put(account::put_settings),
        )
        .route(
            "/api/v1/keys",
            get(api_key::list_keys).post(api_key::create_key),
        )
        .route(
            "/api/v1/keys/{id}",
            axum::routing::delete(api_key::delete_key),
        )
        .route("/api/v1/keys/{id}/rotate", post(api_key::rotate_key))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn cors_layer(config: &Config) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "cloudcell"
    }))
}

async fn ping() -> Json<Value> {
    Json(json!({
        "message": "pong"
    }))
}
