use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{Error, Result};
use crate::state::AppState;

/// Persisted key. The plaintext is never stored.
#[derive(Debug, Clone, FromRow)]
pub struct ApiKeyRecord {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub hash: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyListItem {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub hint: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyCreated {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub hint: String,
    /// Plaintext. Returned only on create/rotate. Never persisted.
    pub key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKey {
    pub name: String,
}

pub(crate) fn hash_key(key: &str) -> String {
    let digest = Sha256::digest(key.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Constant-time-ish compare of SHA-256 hex. Used when HTTP auth is wired.
pub fn key_matches(record: &ApiKeyRecord, key: &str) -> bool {
    record.hash == hash_key(key)
}

fn mint_key() -> (String, String) {
    let raw = format!("cc_live_{}", Uuid::new_v4().simple());
    let prefix: String = raw.chars().take(16).collect();
    (raw, prefix)
}

fn hint_for(prefix: &str) -> String {
    format!("{prefix}••••")
}

fn parse_created_at(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::Internal(anyhow::anyhow!("invalid created_at: {e}")))
}

fn to_list_item(record: ApiKeyRecord) -> Result<ApiKeyListItem> {
    Ok(ApiKeyListItem {
        created_at: parse_created_at(&record.created_at)?,
        id: record.id,
        name: record.name,
        prefix: record.prefix.clone(),
        hint: hint_for(&record.prefix),
    })
}

pub async fn list_keys(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<ApiKeyListItem>>> {
    let rows = sqlx::query_as::<_, ApiKeyRecord>(
        "SELECT id, name, prefix, hash, created_at FROM api_keys
         WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;
    let items = rows
        .into_iter()
        .map(to_list_item)
        .collect::<Result<Vec<_>>>()?;
    Ok(Json(items))
}

pub async fn create_key(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateApiKey>,
) -> Result<Json<ApiKeyCreated>> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(Error::BadRequest("name is required".into()));
    }
    let (key, prefix) = mint_key();
    let now = Utc::now();
    let record = ApiKeyRecord {
        id: Uuid::new_v4().to_string(),
        name: name.clone(),
        prefix: prefix.clone(),
        hash: hash_key(&key),
        created_at: now.to_rfc3339(),
    };
    let created = ApiKeyCreated {
        id: record.id.clone(),
        name,
        prefix,
        hint: hint_for(&record.prefix),
        key,
        created_at: now,
    };
    debug_assert!(key_matches(&record, &created.key));

    sqlx::query(
        "INSERT INTO api_keys (id, name, prefix, hash, created_at, user_id)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&record.id)
    .bind(&record.name)
    .bind(&record.prefix)
    .bind(&record.hash)
    .bind(&record.created_at)
    .bind(&user.id)
    .execute(&state.db)
    .await?;

    Ok(Json(created))
}

pub async fn delete_key(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM api_keys WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("api key {id}")));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn rotate_key(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiKeyCreated>> {
    let mut record = sqlx::query_as::<_, ApiKeyRecord>(
        "SELECT id, name, prefix, hash, created_at FROM api_keys WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("api key {id}")))?;

    let (key, prefix) = mint_key();
    let now = Utc::now();
    record.prefix = prefix.clone();
    record.hash = hash_key(&key);
    record.created_at = now.to_rfc3339();
    debug_assert!(key_matches(&record, &key));

    sqlx::query("UPDATE api_keys SET prefix = ?, hash = ?, created_at = ? WHERE id = ?")
        .bind(&record.prefix)
        .bind(&record.hash)
        .bind(&record.created_at)
        .bind(&record.id)
        .execute(&state.db)
        .await?;

    Ok(Json(ApiKeyCreated {
        id: record.id,
        name: record.name,
        prefix,
        hint: hint_for(&record.prefix),
        key,
        created_at: now,
    }))
}
