use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::snapshot;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct Sandbox {
    pub id: String,
    pub snapshot: String,
    pub state: SandboxState,
    pub cpu: f64,
    pub mem_bytes: u64,
    pub pids: u32,
    pub egress: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxState {
    /// Control-plane record only. No `sand serve` cell has been started.
    Pending,
    Running,
    Stopped,
}

#[derive(Debug, Deserialize)]
pub struct CreateSandbox {
    pub snapshot: Option<String>,
    pub cpu: Option<f64>,
    pub mem_bytes: Option<u64>,
    pub pids: Option<u32>,
    pub egress: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    pub argv: Vec<String>,
    pub stdin: Option<String>,
    pub timeout_ms: Option<u64>,
}

pub async fn list_sandboxes(State(state): State<AppState>) -> Result<Json<Vec<Sandbox>>> {
    let guard = state.sandboxes.lock().await;
    let mut items: Vec<Sandbox> = guard.values().cloned().collect();
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(Json(items))
}

pub async fn create_sandbox(
    State(state): State<AppState>,
    Json(body): Json<CreateSandbox>,
) -> Result<Json<Sandbox>> {
    let snapshot = body.snapshot.unwrap_or_else(|| "base".into());
    if !snapshot::exists(&snapshot) {
        return Err(Error::BadRequest(format!("unknown snapshot: {snapshot}")));
    }
    let cpu = body.cpu.unwrap_or(1.0);
    if cpu <= 0.0 {
        return Err(Error::BadRequest("cpu must be > 0".into()));
    }
    let mem_bytes = body.mem_bytes.unwrap_or(1 << 30);
    if mem_bytes == 0 {
        return Err(Error::BadRequest("mem_bytes must be > 0".into()));
    }
    let pids = body.pids.unwrap_or(64);
    if pids == 0 {
        return Err(Error::BadRequest("pids must be > 0".into()));
    }

    let sandbox = Sandbox {
        id: Uuid::new_v4().to_string(),
        snapshot,
        state: SandboxState::Pending,
        cpu,
        mem_bytes,
        pids,
        egress: body.egress.unwrap_or_default(),
        created_at: Utc::now(),
    };

    state
        .sandboxes
        .lock()
        .await
        .insert(sandbox.id.clone(), sandbox.clone());
    Ok(Json(sandbox))
}

pub async fn get_sandbox(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Sandbox>> {
    let guard = state.sandboxes.lock().await;
    guard
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or_else(|| Error::NotFound(format!("sandbox {id}")))
}

pub async fn delete_sandbox(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let mut guard = state.sandboxes.lock().await;
    guard
        .remove(&id)
        .ok_or_else(|| Error::NotFound(format!("sandbox {id}")))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn exec_sandbox(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ExecRequest>,
) -> Result<Json<serde_json::Value>> {
    let exists = state.sandboxes.lock().await.contains_key(&id);
    if !exists {
        return Err(Error::NotFound(format!("sandbox {id}")));
    }
    if body.argv.is_empty() {
        return Err(Error::BadRequest("argv must not be empty".into()));
    }
    let _ = body.stdin;
    let _ = body.timeout_ms;
    Err(Error::NotImplemented(
        "exec requires a Linux cell node running agentcell".into(),
    ))
}
