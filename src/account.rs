use axum::{Json, extract::State};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api_key::hash_key;
use crate::error::{Error, Result};
use crate::state::AppState;

const MIN_PASSWORD_LEN: usize = 8;
const SESSION_DAYS: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct AuthStatus {
    pub registration_enabled: bool,
    pub has_users: bool,
    pub authenticated: bool,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsBody {
    pub registration_enabled: bool,
}

fn normalize_email(email: &str) -> Result<String> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') || !email.contains('.') || email.len() < 5 {
        return Err(Error::BadRequest("invalid email".into()));
    }
    Ok(email)
}

fn hash_password(password: &str) -> Result<String> {
    use argon2::Argon2;
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| Error::Internal(anyhow::anyhow!("password hash: {e}")))
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::Argon2;
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    PasswordHash::new(hash)
        .ok()
        .and_then(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .ok()
        })
        .is_some()
}

async fn user_count(state: &AppState) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    Ok(count)
}

async fn registration_enabled(state: &AppState) -> Result<bool> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'registration_enabled'")
            .fetch_optional(&state.db)
            .await?;
    Ok(value.as_deref() != Some("false"))
}

async fn insert_session(state: &AppState, user_id: &str) -> Result<String> {
    let raw = format!("cc_sess_{}", Uuid::new_v4().simple());
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, hash, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(hash_key(&raw))
    .bind(now.to_rfc3339())
    .bind((now + Duration::days(SESSION_DAYS)).to_rfc3339())
    .execute(&state.db)
    .await?;
    Ok(raw)
}

pub async fn status(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<AuthStatus>> {
    let enabled = registration_enabled(&state).await?;
    let has_users = user_count(&state).await? > 0;
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let email = if let Some(token) = token {
        lookup_session_email(&state, token).await?
    } else {
        None
    };
    Ok(Json(AuthStatus {
        registration_enabled: enabled,
        has_users,
        authenticated: email.is_some(),
        email,
    }))
}

pub async fn lookup_session_email(state: &AppState, token: &str) -> Result<Option<String>> {
    if !token.starts_with("cc_sess_") {
        return Ok(None);
    }
    let now = Utc::now().to_rfc3339();
    let email: Option<String> = sqlx::query_scalar(
        "SELECT u.email FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.hash = ? AND s.expires_at > ?",
    )
    .bind(hash_key(token))
    .bind(now)
    .fetch_optional(&state.db)
    .await?;
    Ok(email)
}

pub async fn session_valid(state: &AppState, token: &str) -> Result<bool> {
    Ok(lookup_session_email(state, token).await?.is_some())
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>> {
    let email = normalize_email(&body.email)?;
    if body.password.len() < MIN_PASSWORD_LEN {
        return Err(Error::BadRequest(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    let users = user_count(&state).await?;
    if users > 0 && !registration_enabled(&state).await? {
        return Err(Error::Forbidden("registration is disabled".into()));
    }
    let username = email.clone();
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let password_hash = hash_password(&body.password)?;
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO users (id, email, username, password_hash, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&email)
    .bind(&username)
    .bind(&password_hash)
    .bind(&now)
    .execute(&state.db)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(Error::Conflict("email already registered".into()));
    }
    let token = insert_session(&state, &id).await?;
    Ok(Json(AuthResponse { token, email }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    let email = normalize_email(&body.email)?;
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;
    let Some((id, password_hash)) = row else {
        return Err(Error::Unauthorized("invalid email or password".into()));
    };
    if !verify_password(&body.password, &password_hash) {
        return Err(Error::Unauthorized("invalid email or password".into()));
    }
    let token = insert_session(&state, &id).await?;
    Ok(Json(AuthResponse { token, email }))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let Some(token) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
    else {
        return Ok(Json(serde_json::json!({ "ok": true })));
    };
    if token.starts_with("cc_sess_") {
        sqlx::query("DELETE FROM sessions WHERE hash = ?")
            .bind(hash_key(token))
            .execute(&state.db)
            .await?;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn get_settings(State(state): State<AppState>) -> Result<Json<SettingsBody>> {
    Ok(Json(SettingsBody {
        registration_enabled: registration_enabled(&state).await?,
    }))
}

pub async fn put_settings(
    State(state): State<AppState>,
    Json(body): Json<SettingsBody>,
) -> Result<Json<SettingsBody>> {
    let value = if body.registration_enabled {
        "true"
    } else {
        "false"
    };
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('registration_enabled', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(value)
    .execute(&state.db)
    .await?;
    Ok(Json(body))
}
