use agentbox_core::config;
use agentbox_core::types::{IpcRequest, IpcResponse};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Async IPC client that communicates with the agentbox daemon via Unix socket.
#[derive(Clone)]
pub struct IpcClient;

impl IpcClient {
    /// Send an IPC request to the daemon and return the response.
    pub async fn call(
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<IpcResponse> {
        let socket_path = config::socket_path();

        if !socket_path.exists() {
            anyhow::bail!(
                "Daemon not running (socket not found). Start it with 'agentbox daemon start'."
            );
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

    /// Convenience: call IPC and extract the result, returning an error string on failure.
    pub async fn call_ok(
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = Self::call(method, params)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(err) = resp.error {
            Err(err.message)
        } else {
            Ok(resp.result.unwrap_or(serde_json::Value::Null))
        }
    }
}
