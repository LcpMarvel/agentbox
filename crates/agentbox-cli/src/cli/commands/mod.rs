pub mod config_cmd;
pub mod daemon;
pub mod dashboard;
pub mod edit;
pub mod history;
pub mod list;
pub mod logs;
pub mod pause;
pub mod register;
pub mod remove;
pub mod resume;
pub mod run;
pub mod schedule;
pub mod upgrade;

use agentbox_core::config;
use agentbox_core::types::{IpcRequest, IpcResponse};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Send an IPC request to the daemon and return the response.
pub async fn ipc_call(method: &str, params: serde_json::Value) -> anyhow::Result<IpcResponse> {
    let socket_path = config::socket_path();

    // Auto-start daemon if not running
    if !socket_path.exists() {
        eprintln!("Daemon not running, starting...");
        daemon::start(false).await?;
        // Wait for socket
        for _ in 0..30 {
            if socket_path.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        if !socket_path.exists() {
            anyhow::bail!("Failed to start daemon (socket not found after 3s)");
        }
    }

    let stream = UnixStream::connect(&socket_path).await.map_err(|e| {
        anyhow::anyhow!(
            "Cannot connect to daemon: {}. Try 'agentbox daemon start'.",
            e
        )
    })?;

    let (reader, mut writer) = stream.into_split();

    let id = REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let req = IpcRequest::new(id, method, params);
    let mut req_json = serde_json::to_string(&req)?;
    req_json.push('\n');
    writer.write_all(req_json.as_bytes()).await?;

    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    buf_reader.read_line(&mut line).await?;

    let response: IpcResponse = serde_json::from_str(line.trim())?;
    Ok(response)
}
