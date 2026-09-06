use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::state::AppState;

/// Persisted key. The plaintext is never stored.
#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub hash: String,
    pub created_at: DateTime<Utc>,
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

fn hash_key(key: &str) -> String {
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

fn to_list_item(record: &ApiKeyRecord) -> ApiKeyListItem {
    ApiKeyListItem {
        id: record.id.clone(),
        name: record.name.clone(),
        prefix: record.prefix.clone(),
        hint: hint_for(&record.prefix),
        created_at: record.created_at,
    }
}

pub async fn list_keys(State(state): State<AppState>) -> Result<Json<Vec<ApiKeyListItem>>> {
    let guard = state.keys.lock().await;
    let mut items: Vec<ApiKeyListItem> = guard.values().map(to_list_item).collect();
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(Json(items))
}

pub async fn create_key(
    State(state): State<AppState>,
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
        created_at: now,
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
    state.keys.lock().await.insert(record.id.clone(), record);
    Ok(Json(created))
}

pub async fn delete_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let mut guard = state.keys.lock().await;
    guard
        .remove(&id)
        .ok_or_else(|| Error::NotFound(format!("api key {id}")))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn rotate_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiKeyCreated>> {
    let mut guard = state.keys.lock().await;
    let record = guard
        .get_mut(&id)
        .ok_or_else(|| Error::NotFound(format!("api key {id}")))?;
    let (key, prefix) = mint_key();
    record.prefix = prefix.clone();
    record.hash = hash_key(&key);
    record.created_at = Utc::now();
    debug_assert!(key_matches(record, &key));
    Ok(Json(ApiKeyCreated {
        id: record.id.clone(),
        name: record.name.clone(),
        prefix,
        hint: hint_for(&record.prefix),
        key,
        created_at: record.created_at,
    }))
}
