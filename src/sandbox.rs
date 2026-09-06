use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::cell;
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

impl SandboxState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Stopped => "stopped",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "stopped" => Ok(Self::Stopped),
            other => Err(Error::Internal(anyhow::anyhow!(
                "invalid sandbox state: {other}"
            ))),
        }
    }
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

#[derive(FromRow)]
struct SandboxRow {
    id: String,
    snapshot: String,
    state: String,
    cpu: f64,
    mem_bytes: i64,
    pids: i64,
    egress_json: String,
    created_at: String,
}

impl TryFrom<SandboxRow> for Sandbox {
    type Error = Error;

    fn try_from(row: SandboxRow) -> Result<Self> {
        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map_err(|e| Error::Internal(anyhow::anyhow!("invalid created_at: {e}")))?
            .with_timezone(&Utc);
        let egress: Vec<String> = serde_json::from_str(&row.egress_json)
            .map_err(|e| Error::Internal(anyhow::anyhow!("invalid egress_json: {e}")))?;
        Ok(Self {
            id: row.id,
            snapshot: row.snapshot,
            state: SandboxState::parse(&row.state)?,
            cpu: row.cpu,
            mem_bytes: u64::try_from(row.mem_bytes)
                .map_err(|_| Error::Internal(anyhow::anyhow!("negative mem_bytes")))?,
            pids: u32::try_from(row.pids)
                .map_err(|_| Error::Internal(anyhow::anyhow!("invalid pids")))?,
            egress,
            created_at,
        })
    }
}

pub async fn list_sandboxes(State(state): State<AppState>) -> Result<Json<Vec<Sandbox>>> {
    let rows = sqlx::query_as::<_, SandboxRow>(
        "SELECT id, snapshot, state, cpu, mem_bytes, pids, egress_json, created_at
         FROM sandboxes ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;
    let items = rows
        .into_iter()
        .map(Sandbox::try_from)
        .collect::<Result<Vec<_>>>()?;
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

    let mut sandbox = Sandbox {
        id: Uuid::new_v4().to_string(),
        snapshot,
        state: SandboxState::Pending,
        cpu,
        mem_bytes,
        pids,
        egress: body.egress.unwrap_or_default(),
        created_at: Utc::now(),
    };
    let egress_json =
        serde_json::to_string(&sandbox.egress).map_err(|e| Error::Internal(anyhow::anyhow!(e)))?;

    if let Some(sand) = &state.config.sand_bin
        && !sand.is_file()
    {
        return Err(Error::Config(format!(
            "sand binary not found: {}",
            sand.display()
        )));
    }

    sqlx::query(
        "INSERT INTO sandboxes
         (id, snapshot, state, cpu, mem_bytes, pids, egress_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&sandbox.id)
    .bind(&sandbox.snapshot)
    .bind(sandbox.state.as_str())
    .bind(sandbox.cpu)
    .bind(i64::try_from(sandbox.mem_bytes).unwrap_or(i64::MAX))
    .bind(i64::from(sandbox.pids))
    .bind(&egress_json)
    .bind(sandbox.created_at.to_rfc3339())
    .execute(&state.db)
    .await?;

    if let Some(sand) = state.config.sand_bin.clone() {
        let workdir = state
            .config
            .data_dir
            .join("sbx")
            .join(&sandbox.id)
            .join("work");
        match state
            .cells
            .start(
                sandbox.id.clone(),
                &sand,
                &workdir,
                sandbox.mem_bytes,
                sandbox.cpu,
                sandbox.pids,
            )
            .await
        {
            Ok((sock, pid)) => {
                sqlx::query(
                    "UPDATE sandboxes SET state = 'running', sock = ?, pid = ? WHERE id = ?",
                )
                .bind(sock.to_string_lossy().as_ref())
                .bind(pid)
                .bind(&sandbox.id)
                .execute(&state.db)
                .await?;
                sandbox.state = SandboxState::Running;
            }
            Err(err) => {
                let _ = sqlx::query("DELETE FROM sandboxes WHERE id = ?")
                    .bind(&sandbox.id)
                    .execute(&state.db)
                    .await;
                return Err(err);
            }
        }
    }

    Ok(Json(sandbox))
}

pub async fn get_sandbox(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Sandbox>> {
    let row = sqlx::query_as::<_, SandboxRow>(
        "SELECT id, snapshot, state, cpu, mem_bytes, pids, egress_json, created_at
         FROM sandboxes WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("sandbox {id}")))?;
    Ok(Json(Sandbox::try_from(row)?))
}

pub async fn delete_sandbox(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    state.cells.shutdown(&id).await?;
    let workdir = state.config.data_dir.join("sbx").join(&id);
    let _ = tokio::fs::remove_dir_all(&workdir).await;
    let result = sqlx::query("DELETE FROM sandboxes WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("sandbox {id}")));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn exec_sandbox(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ExecRequest>,
) -> Result<Json<serde_json::Value>> {
    if body.argv.is_empty() {
        return Err(Error::BadRequest("argv must not be empty".into()));
    }
    let row_state: Option<String> = sqlx::query_scalar("SELECT state FROM sandboxes WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?;
    let Some(row_state) = row_state else {
        return Err(Error::NotFound(format!("sandbox {id}")));
    };

    if let Some(sock) = state.cells.sock(&id).await {
        let stdin = body.stdin.unwrap_or_default();
        let timeout_ms = body.timeout_ms.unwrap_or(30_000);
        let (out, code) = cell::exec(
            &sock,
            &body.argv,
            stdin.as_bytes(),
            std::time::Duration::from_millis(timeout_ms),
        )
        .await?;
        return Ok(Json(serde_json::json!({
            "stdout": String::from_utf8_lossy(&out),
            "code": code,
        })));
    }

    match row_state.as_str() {
        "pending" => Err(Error::NotImplemented(
            "exec requires CLOUDCELL_SAND pointing at a sand binary".into(),
        )),
        "stopped" => Err(Error::Conflict(
            "sandbox is stopped; create a new one".into(),
        )),
        "running" => Err(Error::NotImplemented(
            "cell process was lost after restart; create a new sandbox".into(),
        )),
        other => Err(Error::Internal(anyhow::anyhow!(
            "invalid sandbox state: {other}"
        ))),
    }
}
