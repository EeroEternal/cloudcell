use axum::{
    Json,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::cell;
use crate::error::{Error, Result};
use crate::snapshot;
use crate::state::AppState;

/// Default `/exec` timeout. Long enough for a cold build/install cycle;
/// override per request with `timeout_ms`.
const DEFAULT_EXEC_TIMEOUT_MS: u64 = 600_000;

#[derive(Debug, Clone, Serialize)]
pub struct Sandbox {
    pub id: String,
    pub snapshot: String,
    pub state: SandboxState,
    pub cpu: f64,
    pub mem_bytes: u64,
    pub pids: u32,
    pub egress: Vec<String>,
    pub caches: Vec<String>,
    pub disk_bytes: Option<u64>,
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
    /// Named warm caches bind-mounted into the cell (repeatable).
    /// Known: cargo, rustup, pip, npm, go, gradle, maven.
    pub caches: Option<Vec<String>>,
    /// Optional hard workspace cap: a RAM-backed tmpfs of this size
    /// instead of the host workdir bind. Counts against `mem_bytes`.
    pub disk_bytes: Option<u64>,
    /// Git credentials injected into the cell env; never persisted.
    pub git: Option<GitAuth>,
}

#[derive(Debug, Deserialize)]
pub struct GitAuth {
    /// GitHub PAT, used over HTTPS (`GIT_CONFIG_*` insteadOf rule).
    pub token: Option<String>,
    /// OpenSSH private key for `git@github.com`.
    pub ssh_key: Option<String>,
}

/// In-cell mount point for each supported warm cache.  These are the
/// paths the toolchains look in by default, so no extra config is needed.
fn cache_mount(name: &str) -> Option<&'static str> {
    Some(match name {
        "cargo" => "/home/agent/.cargo",
        "rustup" => "/home/agent/.rustup",
        "pip" => "/home/agent/.cache/pip",
        "npm" => "/home/agent/.npm",
        "go" => "/home/agent/go",
        "gradle" => "/home/agent/.gradle",
        "maven" => "/home/agent/.m2",
        _ => return None,
    })
}

fn ensure_egress(egress: &mut Vec<String>, host: &str) {
    if !egress.iter().any(|e| e.eq_ignore_ascii_case(host)) {
        egress.push(host.to_string());
    }
}

/// Validate `HOST[:PORT]` entries, rejecting empty/whitespace hosts, IPv6
/// literals (the cell rules are AF_INET) and bad ports; dedups in place.
fn validate_egress(egress: &mut Vec<String>) -> std::result::Result<(), String> {
    let mut seen: Vec<String> = Vec::new();
    for raw in egress.iter() {
        let spec = raw.trim();
        if spec.is_empty() {
            return Err("egress_invalid: empty entry".into());
        }
        if spec.chars().any(|c| c <= ' ' || c == '[' || c == ']') {
            return Err(format!("egress_invalid: {raw:?} (no spaces or brackets)"));
        }
        let (host, port) = match spec.rsplit_once(':') {
            Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (h, Some(p)),
            _ => (spec, None),
        };
        if host.is_empty() || host.contains(':') {
            return Err(format!(
                "egress_invalid: {raw:?} (IPv6 literals unsupported)"
            ));
        }
        if !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        {
            return Err(format!("egress_invalid: {raw:?} (bad host)"));
        }
        if let Some(p) = port
            && !matches!(p.parse::<u16>(), Ok(1..=65535))
        {
            return Err(format!("egress_invalid: {raw:?} (bad port)"));
        }
        let norm = match port {
            Some(p) => format!("{host}:{p}"),
            None => host.to_string(),
        };
        if !seen.iter().any(|s| s.eq_ignore_ascii_case(&norm)) {
            seen.push(norm);
        }
    }
    *egress = seen;
    Ok(())
}

/// Probe the cell binary once; a stale `sand` without the flags this
/// control plane passes must fail loudly, not hand back a broken sandbox.
async fn probe_caps(bin: &std::path::Path) -> std::result::Result<(), String> {
    let out = tokio::process::Command::new(bin)
        .arg("--capabilities")
        .output()
        .await
        .map_err(|e| format!("cannot run {} --capabilities: {e}", bin.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} --capabilities failed ({})",
            bin.display(),
            out.status
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for need in ["egress_multi=1", "egress_refresh=1", "egress_resolv=1"] {
        if !text.contains(need) {
            return Err(format!(
                "sand binary is missing {need}; rebuild agentcell (got: {})",
                text.trim()
            ));
        }
    }
    Ok(())
}

/// Turn a nonzero exec into a machine-readable reason: `oom`, `disk_full`,
/// `network_denied` (with the offending host when we can parse it) or
/// `nonzero`.
fn classify_failure(code: u32, output: &str) -> Option<serde_json::Value> {
    if code == 0 {
        return None;
    }
    if code == 137 {
        return Some(json!({
            "reason": "oom",
            "detail": "killed by SIGKILL (exit 137); the cgroup memory limit was likely hit"
        }));
    }
    let low = output.to_ascii_lowercase();
    if low.contains("no space left on device")
        || low.contains("disk quota exceeded")
        || low.contains("enospc")
    {
        return Some(json!({
            "reason": "disk_full",
            "detail": "the workspace ran out of space"
        }));
    }
    if let Some(host) = network_host(output) {
        return Some(json!({
            "reason": "network_denied",
            "host": host,
            "detail": "egress to this host was refused or not allowlisted"
        }));
    }
    if low.contains("network is unreachable")
        || low.contains("connection timed out")
        || low.contains("connection refused")
        || low.contains("temporary failure in name resolution")
        || low.contains("could not resolve")
    {
        return Some(json!({
            "reason": "network_denied",
            "detail": "network unreachable, refused, or DNS failed"
        }));
    }
    Some(json!({"reason": "nonzero", "detail": "command exited nonzero"}))
}

async fn write_file_0600(path: &std::path::Path, contents: &str) -> Result<()> {
    let mut f = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("write {}: {e}", path.display())))?;
    f.write_all(contents.as_bytes())
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("write {}: {e}", path.display())))
}

/// Best-effort hostname extraction from common git/curl/ssh error lines.
fn network_host(output: &str) -> Option<String> {
    for line in output.lines() {
        for (marker, stop) in [
            ("Could not resolve host: ", None),
            ("Could not resolve hostname ", Some(" ")),
            ("Failed to connect to ", Some(" port")),
            ("ssh: connect to host ", Some(" port")),
        ] {
            if let Some(rest) = line.split(marker).nth(1) {
                let end = stop
                    .and_then(|s| rest.find(s))
                    .unwrap_or_else(|| rest.find([' ', '\n', '\r']).unwrap_or(rest.len()));
                let host = rest[..end].trim();
                if !host.is_empty() {
                    return Some(host.to_string());
                }
            }
        }
    }
    None
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
    caches_json: String,
    disk_bytes: Option<i64>,
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
        let caches: Vec<String> = serde_json::from_str(&row.caches_json)
            .map_err(|e| Error::Internal(anyhow::anyhow!("invalid caches_json: {e}")))?;
        let disk_bytes = row
            .disk_bytes
            .map(|b| {
                u64::try_from(b)
                    .map_err(|_| Error::Internal(anyhow::anyhow!("negative disk_bytes")))
            })
            .transpose()?;
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
            caches,
            disk_bytes,
            created_at,
        })
    }
}

pub async fn list_sandboxes(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<Sandbox>>> {
    let rows = sqlx::query_as::<_, SandboxRow>(
        "SELECT id, snapshot, state, cpu, mem_bytes, pids, egress_json, caches_json,
                disk_bytes, created_at
         FROM sandboxes WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&user.id)
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
    user: AuthUser,
    Json(body): Json<CreateSandbox>,
) -> Result<Json<Sandbox>> {
    let snapshot = body.snapshot.unwrap_or_else(|| "base".into());
    if !snapshot::exists(&state.config.rootfs_dir, &snapshot) {
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
    let disk_bytes = body.disk_bytes;
    if let Some(0) = disk_bytes {
        return Err(Error::BadRequest("disk_bytes must be > 0".into()));
    }
    let caches = body.caches.unwrap_or_default();
    if caches.len() > 16 {
        return Err(Error::BadRequest("too many caches (max 16)".into()));
    }
    for name in &caches {
        if cache_mount(name).is_none() {
            return Err(Error::BadRequest(format!(
                "unknown cache: {name} (known: cargo, rustup, pip, npm, go, gradle, maven)"
            )));
        }
    }
    let git = body.git;
    if let Some(g) = &git {
        let has_token = g.token.as_deref().is_some_and(|t| !t.is_empty());
        let has_key = g.ssh_key.as_deref().is_some_and(|k| !k.is_empty());
        if !has_token && !has_key {
            return Err(Error::BadRequest(
                "git requires a non-empty token and/or ssh_key".into(),
            ));
        }
    }

    let mut egress = body.egress.unwrap_or_default();
    validate_egress(&mut egress).map_err(Error::BadRequest)?;

    let mut sandbox = Sandbox {
        id: Uuid::new_v4().to_string(),
        snapshot,
        state: SandboxState::Pending,
        cpu,
        mem_bytes,
        pids,
        egress,
        caches,
        disk_bytes,
        created_at: Utc::now(),
    };
    let egress_json =
        serde_json::to_string(&sandbox.egress).map_err(|e| Error::Internal(anyhow::anyhow!(e)))?;
    let caches_json =
        serde_json::to_string(&sandbox.caches).map_err(|e| Error::Internal(anyhow::anyhow!(e)))?;

    if let Some(sand) = &state.config.sand_bin {
        if !sand.is_file() {
            return Err(Error::Config(format!(
                "sand binary not found: {}",
                sand.display()
            )));
        }
        // Refuse to run against a `sand` that lacks the flags below.
        let caps = state.caps.get_or_init(|| probe_caps(sand)).await;
        if let Err(e) = caps {
            return Err(Error::Config(e.clone()));
        }
    }
    // Fail fast: a declared snapshot with no packed rootfs on this node
    // would otherwise silently fall back to the host's live /usr.
    if state.config.sand_bin.is_some()
        && !snapshot::packed(&state.config.rootfs_dir, &sandbox.snapshot)
    {
        return Err(Error::SnapshotNotPacked(format!(
            "{} is declared but not packed at {}",
            sandbox.snapshot,
            state.config.rootfs_dir.join(&sandbox.snapshot).display()
        )));
    }

    sqlx::query(
        "INSERT INTO sandboxes
         (id, snapshot, state, cpu, mem_bytes, pids, egress_json, caches_json,
          disk_bytes, created_at, user_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&sandbox.id)
    .bind(&sandbox.snapshot)
    .bind(sandbox.state.as_str())
    .bind(sandbox.cpu)
    .bind(i64::try_from(sandbox.mem_bytes).unwrap_or(i64::MAX))
    .bind(i64::from(sandbox.pids))
    .bind(&egress_json)
    .bind(&caches_json)
    .bind(
        sandbox
            .disk_bytes
            .map(|b| i64::try_from(b).unwrap_or(i64::MAX)),
    )
    .bind(sandbox.created_at.to_rfc3339())
    .bind(&user.id)
    .execute(&state.db)
    .await?;

    if let Some(sand) = state.config.sand_bin.clone() {
        let sbx_dir = state.config.data_dir.join("sbx").join(&sandbox.id);
        let workdir = sbx_dir.join("work");
        // Content-addressed rootfs when imported, legacy tree otherwise.
        let rootfs =
            snapshot::resolve(&state.config.rootfs_dir, &sandbox.snapshot).ok_or_else(|| {
                Error::SnapshotNotPacked(format!(
                    "{} is declared but not packed at {}",
                    sandbox.snapshot,
                    state.config.rootfs_dir.display()
                ))
            })?;

        // R4 — warm caches: a stable per-user volume bind-mounted at the
        // path each toolchain reads by default.
        let mut binds: Vec<(PathBuf, String)> = Vec::new();
        for name in &sandbox.caches {
            let dst = cache_mount(name).expect("validated above");
            let src = state.config.cache_dir.join(&user.id).join(name);
            tokio::fs::create_dir_all(&src)
                .await
                .map_err(|e| Error::Internal(anyhow::anyhow!("create cache {name}: {e}")))?;
            binds.push((src, dst.to_string()));
        }

        let mut egress: Vec<String> = sandbox
            .egress
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect();

        // R5 — git auth. Env values (incl. the token) go into a 0600 env
        // file, never argv or the DB; the ssh key is copied into the
        // cell's tmpfs via --secret.
        let mut env: Vec<(String, String)> = Vec::new();
        let mut secrets: Vec<(String, PathBuf)> = Vec::new();
        if let Some(git) = &git {
            if let Some(token) = git.token.as_deref().filter(|t| !t.is_empty()) {
                env.push(("GITHUB_TOKEN".into(), token.into()));
                env.push(("GH_TOKEN".into(), token.into()));
                env.push(("GIT_CONFIG_COUNT".into(), "1".into()));
                env.push((
                    "GIT_CONFIG_KEY_0".into(),
                    format!("url.https://x-access-token:{token}@github.com/.insteadOf"),
                ));
                env.push(("GIT_CONFIG_VALUE_0".into(), "https://github.com/".into()));
                ensure_egress(&mut egress, "github.com:443");
            }
            if let Some(key) = git.ssh_key.as_deref().filter(|k| !k.is_empty()) {
                tokio::fs::create_dir_all(&sbx_dir)
                    .await
                    .map_err(|e| Error::Internal(anyhow::anyhow!("create sandbox dir: {e}")))?;
                let key_path = sbx_dir.join("git.id_ed25519");
                write_file_0600(&key_path, key).await?;
                secrets.push(("/run/agentcell-git/id_ed25519".into(), key_path));
                env.push((
                    "GIT_SSH_COMMAND".into(),
                    "ssh -i /run/agentcell-git/id_ed25519 -o IdentitiesOnly=yes \
                     -o StrictHostKeyChecking=accept-new \
                     -o UserKnownHostsFile=/run/agentcell-git/known_hosts"
                        .into(),
                ));
                ensure_egress(&mut egress, "github.com:22");
            }
        }
        let env_file = if env.is_empty() {
            None
        } else {
            tokio::fs::create_dir_all(&sbx_dir)
                .await
                .map_err(|e| Error::Internal(anyhow::anyhow!("create sandbox dir: {e}")))?;
            let path = sbx_dir.join("cell.env");
            let mut text = String::new();
            for (k, v) in &env {
                text.push_str(k);
                text.push('=');
                text.push_str(v);
                text.push('\n');
            }
            write_file_0600(&path, &text).await?;
            Some(path)
        };

        match state
            .cells
            .start(
                sandbox.id.clone(),
                cell::SpawnOpts {
                    sand_bin: sand,
                    workdir,
                    mem_bytes: sandbox.mem_bytes,
                    cpu: sandbox.cpu,
                    pids: sandbox.pids,
                    rootfs: Some(rootfs),
                    net_veth: true,
                    egress,
                    binds,
                    env_file,
                    secrets,
                    workdir_size: sandbox.disk_bytes,
                },
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
                let msg = err.to_string();
                if msg.contains("egress unavailable") || msg.contains("egress_unresolved") {
                    return Err(Error::BadGateway(format!(
                        "egress could not be provisioned: {msg}"
                    )));
                }
                return Err(err);
            }
        }
    }

    Ok(Json(sandbox))
}

pub async fn get_sandbox(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Sandbox>> {
    let row = sqlx::query_as::<_, SandboxRow>(
        "SELECT id, snapshot, state, cpu, mem_bytes, pids, egress_json, caches_json,
                disk_bytes, created_at
         FROM sandboxes WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("sandbox {id}")))?;
    Ok(Json(Sandbox::try_from(row)?))
}

pub async fn delete_sandbox(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let owned: Option<String> =
        sqlx::query_scalar("SELECT id FROM sandboxes WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&user.id)
            .fetch_optional(&state.db)
            .await?;
    if owned.is_none() {
        return Err(Error::NotFound(format!("sandbox {id}")));
    }
    state.cells.shutdown(&id).await?;
    let workdir = state.config.data_dir.join("sbx").join(&id);
    let _ = tokio::fs::remove_dir_all(&workdir).await;
    sqlx::query("DELETE FROM sandboxes WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn exec_sandbox(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<ExecRequest>,
) -> Result<Json<serde_json::Value>> {
    if body.argv.is_empty() {
        return Err(Error::BadRequest("argv must not be empty".into()));
    }
    let row_state: Option<String> =
        sqlx::query_scalar("SELECT state FROM sandboxes WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&user.id)
            .fetch_optional(&state.db)
            .await?;
    let Some(row_state) = row_state else {
        return Err(Error::NotFound(format!("sandbox {id}")));
    };

    if let Some(sock) = state.cells.sock(&id).await {
        let stdin = body.stdin.unwrap_or_default();
        let timeout_ms = body.timeout_ms.unwrap_or(DEFAULT_EXEC_TIMEOUT_MS);
        let (out, code) = cell::exec(
            &sock,
            &body.argv,
            stdin.as_bytes(),
            std::time::Duration::from_millis(timeout_ms),
        )
        .await?;
        let stdout = String::from_utf8_lossy(&out).into_owned();
        let mut reply = json!({
            "stdout": stdout,
            "code": code,
        });
        if let Some(failure) = classify_failure(code, &stdout) {
            reply["failure"] = failure;
        }
        return Ok(Json(reply));
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

#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    pub argv: Option<String>,
}

pub async fn stream_sandbox(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Query(query): Query<StreamQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse> {
    let row_state: Option<String> =
        sqlx::query_scalar("SELECT state FROM sandboxes WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&user.id)
            .fetch_optional(&state.db)
            .await?;
    let Some(row_state) = row_state else {
        return Err(Error::NotFound(format!("sandbox {id}")));
    };

    if row_state != "running" {
        return Err(Error::Conflict(format!(
            "sandbox is not running (state: {row_state})"
        )));
    }

    let sock = state
        .cells
        .sock(&id)
        .await
        .ok_or_else(|| Error::NotImplemented("cell socket unavailable".into()))?;

    let argv: Vec<String> = if let Some(raw) = query.argv {
        serde_json::from_str(&raw)
            .unwrap_or_else(|_| raw.split_whitespace().map(String::from).collect())
    } else {
        vec!["zene".to_string(), "acp".to_string()]
    };

    Ok(ws.on_upgrade(move |socket| handle_ws_stream(socket, sock, argv)))
}

async fn handle_ws_stream(socket: WebSocket, sock_path: std::path::PathBuf, argv: Vec<String>) {
    use futures_util::{SinkExt, StreamExt};

    let (mut ws_sender, mut ws_receiver) = socket.split();

    let cell_stream = match cell::connect_stream(&sock_path, &argv).await {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(error = %err, "failed to connect cell stream");
            let _ = ws_sender
                .send(Message::Text(
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": { "code": -32000, "message": format!("cell stream connect error: {err}") }
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            return;
        }
    };

    let (mut cell_reader, mut cell_writer) = (cell_stream.reader, cell_stream.writer);

    let mut ws_to_cell = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let mut bytes = text.as_bytes().to_vec();
                    if !bytes.ends_with(b"\n") {
                        bytes.push(b'\n');
                    }
                    if let Err(e) = cell_writer.write_all(&bytes).await {
                        tracing::debug!(error = %e, "cell write error");
                        break;
                    }
                }
                Ok(Message::Binary(bin)) => {
                    if let Err(e) = cell_writer.write_all(&bin).await {
                        tracing::debug!(error = %e, "cell write error");
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                Err(e) => {
                    tracing::debug!(error = %e, "ws recv error");
                    break;
                }
            }
        }
        let _ = cell_writer.shutdown().await;
    });

    let mut cell_to_ws = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match cell_reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if let Err(e) = ws_sender
                        .send(Message::Binary(buf[..n].to_vec().into()))
                        .await
                    {
                        tracing::debug!(error = %e, "ws send error");
                        break;
                    }
                }
                Err(err) => {
                    tracing::debug!(error = %err, "cell read error");
                    break;
                }
            }
        }
        let _ = ws_sender.close().await;
    });

    tokio::select! {
        _ = &mut ws_to_cell => {
            cell_to_ws.abort();
        }
        _ = &mut cell_to_ws => {
            ws_to_cell.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_mounts_known_names() {
        assert_eq!(cache_mount("cargo"), Some("/home/agent/.cargo"));
        assert_eq!(cache_mount("pip"), Some("/home/agent/.cache/pip"));
        assert!(cache_mount("bogus").is_none());
    }

    #[test]
    fn classify_zero_is_none() {
        assert!(classify_failure(0, "all good").is_none());
    }

    #[test]
    fn classify_oom_disk_network() {
        let oom = classify_failure(137, "").unwrap();
        assert_eq!(oom["reason"], "oom");

        let disk = classify_failure(1, "cp: error: No space left on device").unwrap();
        assert_eq!(disk["reason"], "disk_full");

        let net = classify_failure(
            1,
            "fatal: unable to access 'https://github.com/x/y': Could not resolve host: github.com",
        )
        .unwrap();
        assert_eq!(net["reason"], "network_denied");
        assert_eq!(net["host"], "github.com");

        let nonzero = classify_failure(2, "boom").unwrap();
        assert_eq!(nonzero["reason"], "nonzero");
    }

    #[test]
    fn egress_validation() {
        let mut ok = vec!["crates.io".into(), "static.crates.io:443".into()];
        assert!(validate_egress(&mut ok).is_ok());
        assert_eq!(ok, vec!["crates.io", "static.crates.io:443"]);

        let mut dup = vec!["a.example".into(), "A.example".into()];
        assert!(validate_egress(&mut dup).is_ok());
        assert_eq!(dup.len(), 1);

        for bad in ["bad host", "::1", "[::1]:443", "a:0", "a:99999", ""] {
            let mut v = vec![bad.to_string()];
            assert!(
                validate_egress(&mut v).is_err(),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn network_host_patterns() {
        assert_eq!(
            network_host("Failed to connect to crates.io port 443: timed out").as_deref(),
            Some("crates.io")
        );
        assert_eq!(
            network_host("ssh: connect to host github.com port 22: Network is unreachable")
                .as_deref(),
            Some("github.com")
        );
        assert_eq!(network_host("no host here"), None);
    }
}
