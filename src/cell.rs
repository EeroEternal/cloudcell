//! Drive `sand serve` over the AgentCell exec protocol.
//!
//! Request: little-endian `u32 argc` then `u32 len + bytes` per arg, then stdin,
//! then SHUT_WR. Response: stdout+stderr bytes, then `u32` exit status, then EOF.
//!
//! This process never links `libagentcell`. It only spawns the `sand` binary.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::error::{Error, Result};

const SERVE_WAIT: Duration = Duration::from_secs(5);

#[derive(Clone, Default)]
pub struct CellRegistry {
    inner: Arc<Mutex<HashMap<String, CellHandle>>>,
}

struct CellHandle {
    child: Child,
    sock: PathBuf,
}

pub struct SpawnOpts {
    pub sand_bin: PathBuf,
    pub workdir: PathBuf,
    pub mem_bytes: u64,
    pub cpu: f64,
    pub pids: u32,
    pub rootfs: Option<PathBuf>,
    pub net_veth: bool,
    pub egress: Option<String>,
}

impl CellRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self, id: String, opts: SpawnOpts) -> Result<(PathBuf, i64)> {
        let handle = spawn_sand(opts).await?;
        let sock = handle.sock.clone();
        let pid = handle.child.id().map(i64::from).unwrap_or(0);
        self.inner.lock().await.insert(id, handle);
        Ok((sock, pid))
    }

    pub async fn sock(&self, id: &str) -> Option<PathBuf> {
        self.inner.lock().await.get(id).map(|h| h.sock.clone())
    }

    pub async fn shutdown(&self, id: &str) -> Result<()> {
        let Some(mut handle) = self.inner.lock().await.remove(id) else {
            return Ok(());
        };
        let _ = handle.child.kill().await;
        let _ = timeout(Duration::from_secs(3), handle.child.wait()).await;
        Ok(())
    }
}

pub fn encode_argv(argv: &[String]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(argv.len() as u32).to_le_bytes());
    for arg in argv {
        let bytes = arg.as_bytes();
        buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(bytes);
    }
    buf
}

pub fn parse_sock_line(line: &str) -> Option<PathBuf> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("AGENTCELL_SOCK=") {
        let path = rest.trim();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    let rest = line.split("serving ").nth(1)?;
    let path = rest.split_whitespace().next()?;
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

pub async fn exec(
    sock: &Path,
    argv: &[String],
    stdin: &[u8],
    timeout_dur: Duration,
) -> Result<(Vec<u8>, u32)> {
    timeout(timeout_dur, exec_inner(sock, argv, stdin))
        .await
        .map_err(|_| Error::BadRequest("exec timed out".into()))?
}

async fn exec_inner(sock: &Path, argv: &[String], stdin: &[u8]) -> Result<(Vec<u8>, u32)> {
    let mut stream = UnixStream::connect(sock)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("connect cell socket: {e}")))?;
    stream
        .write_all(&encode_argv(argv))
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("write argv: {e}")))?;
    if !stdin.is_empty() {
        stream
            .write_all(stdin)
            .await
            .map_err(|e| Error::Internal(anyhow::anyhow!("write stdin: {e}")))?;
    }
    stream
        .shutdown()
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("stdin eof: {e}")))?;

    let mut reply = Vec::new();
    stream
        .read_to_end(&mut reply)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("read exec reply: {e}")))?;
    if reply.len() < 4 {
        return Err(Error::Internal(anyhow::anyhow!("short reply from cell")));
    }
    let code_bytes: [u8; 4] = reply[reply.len() - 4..]
        .try_into()
        .map_err(|_| Error::Internal(anyhow::anyhow!("exit trailer")))?;
    let code = u32::from_le_bytes(code_bytes);
    reply.truncate(reply.len() - 4);
    Ok((reply, code))
}

pub struct CellStream {
    pub reader: tokio::net::unix::OwnedReadHalf,
    pub writer: tokio::net::unix::OwnedWriteHalf,
}

pub async fn connect_stream(sock: &Path, argv: &[String]) -> Result<CellStream> {
    let mut stream = UnixStream::connect(sock)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("connect cell socket: {e}")))?;
    stream
        .write_all(&encode_argv(argv))
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("write argv: {e}")))?;
    let (reader, writer) = stream.into_split();
    Ok(CellStream { reader, writer })
}

async fn spawn_sand(opts: SpawnOpts) -> Result<CellHandle> {
    tokio::fs::create_dir_all(&opts.workdir)
        .await
        .map_err(|e| Error::Internal(anyhow::anyhow!("create workdir: {e}")))?;

    let sock_path = opts
        .workdir
        .parent()
        .map(|p| p.join("cell.sock"))
        .unwrap_or_else(|| opts.workdir.join("cell.sock"));

    let mut cmd = Command::new(&opts.sand_bin);
    cmd.arg("serve")
        .arg("--sock")
        .arg(&sock_path)
        .arg("--mem")
        .arg(opts.mem_bytes.to_string())
        .arg("--cpu")
        .arg(opts.cpu.to_string())
        .arg("--pids")
        .arg(opts.pids.to_string())
        .arg("--workdir")
        .arg(&opts.workdir);
    if let Some(rootfs) = &opts.rootfs {
        cmd.arg("--rootfs").arg(rootfs);
    }
    if opts.net_veth {
        cmd.arg("--net").arg("veth");
        if let Some(egress) = &opts.egress {
            cmd.arg("--egress").arg(egress);
        }
    } else {
        cmd.arg("--net").arg("none");
    }
    let mut child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::Internal(anyhow::anyhow!("spawn sand: {e}")))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::Internal(anyhow::anyhow!("sand stderr not piped")))?;
    let mut reader = BufReader::new(stderr);
    let sock = wait_for_sock(&mut child, &mut reader).await?;

    tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => tracing::debug!(target = "sand", "{}", line.trim_end()),
            }
        }
    });

    Ok(CellHandle { child, sock })
}

async fn wait_for_sock(
    child: &mut Child,
    reader: &mut BufReader<tokio::process::ChildStderr>,
) -> Result<PathBuf> {
    let deadline = Instant::now() + SERVE_WAIT;
    let mut line = String::new();
    loop {
        if Instant::now() > deadline {
            let _ = child.kill().await;
            return Err(Error::Internal(anyhow::anyhow!(
                "timed out waiting for sand serve socket"
            )));
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| Error::Internal(anyhow::anyhow!("wait sand: {e}")))?
        {
            return Err(Error::Internal(anyhow::anyhow!(
                "sand exited before serving (status {status})"
            )));
        }
        line.clear();
        match timeout(Duration::from_millis(200), reader.read_line(&mut line)).await {
            Ok(Ok(0)) => {
                return Err(Error::Internal(anyhow::anyhow!(
                    "sand closed stderr before serving"
                )));
            }
            Ok(Ok(_)) => {
                if let Some(sock) = parse_sock_line(&line) {
                    return Ok(sock);
                }
            }
            Ok(Err(e)) => {
                return Err(Error::Internal(anyhow::anyhow!("read sand stderr: {e}")));
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::UnixListener;

    #[test]
    fn parse_serving_line() {
        let line = "sand: serving /run/user/1000/agentcell-1120702.sock  (jail pid 7)\n";
        assert_eq!(
            parse_sock_line(line).unwrap(),
            PathBuf::from("/run/user/1000/agentcell-1120702.sock")
        );
        assert_eq!(
            parse_sock_line("AGENTCELL_SOCK=/var/lib/cloudcell/sbx/x/cell.sock\n").unwrap(),
            PathBuf::from("/var/lib/cloudcell/sbx/x/cell.sock")
        );
    }

    #[test]
    fn argv_header_roundtrip_shape() {
        let buf = encode_argv(&["echo".into(), "hi".into()]);
        assert_eq!(&buf[..4], 2u32.to_le_bytes());
        assert_eq!(&buf[4..8], 4u32.to_le_bytes());
        assert_eq!(&buf[8..12], b"echo");
    }

    #[tokio::test]
    async fn exec_against_fake_cell() {
        let dir = std::env::temp_dir().join(format!(
            "cloudcell-sock-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("cell.sock");
        let _ = std::fs::remove_file(&sock);
        let listener = UnixListener::bind(&sock).unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await.unwrap();
            stream.write_all(b"hello-from-cell").await.unwrap();
            stream.write_all(&7u32.to_le_bytes()).await.unwrap();
        });

        let (out, code) = exec(
            &sock,
            &["echo".into(), "hi".into()],
            b"",
            Duration::from_secs(2),
        )
        .await
        .unwrap();
        assert_eq!(out, b"hello-from-cell");
        assert_eq!(code, 7);
        server.await.unwrap();
        let _ = std::fs::remove_file(&sock);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn connect_stream_duplex_against_fake_cell() {
        let dir = std::env::temp_dir().join(format!(
            "cloudcell-stream-sock-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("cell.sock");
        let _ = std::fs::remove_file(&sock);
        let listener = UnixListener::bind(&sock).unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut argc_buf = [0u8; 4];
            stream.read_exact(&mut argc_buf).await.unwrap();
            let argc = u32::from_le_bytes(argc_buf);
            assert_eq!(argc, 2);

            // Read 2 args
            for _ in 0..argc {
                let mut len_buf = [0u8; 4];
                stream.read_exact(&mut len_buf).await.unwrap();
                let len = u32::from_le_bytes(len_buf) as usize;
                let mut arg_buf = vec![0u8; len];
                stream.read_exact(&mut arg_buf).await.unwrap();
            }

            // Duplex echo: read line, send reply
            let mut buf = [0u8; 100];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"ping\n");
            stream.write_all(b"pong\n").await.unwrap();
        });

        let mut cell_stream = connect_stream(&sock, &["zene".into(), "acp".into()])
            .await
            .unwrap();

        cell_stream.writer.write_all(b"ping\n").await.unwrap();
        let mut resp = [0u8; 5];
        cell_stream.reader.read_exact(&mut resp).await.unwrap();
        assert_eq!(&resp, b"pong\n");

        server.await.unwrap();
        let _ = std::fs::remove_file(&sock);
        let _ = std::fs::remove_dir(&dir);
    }
}
