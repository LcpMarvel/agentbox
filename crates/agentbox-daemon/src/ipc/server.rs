use crate::scheduler::SchedulerEvent;
use agentbox_core::config;
use agentbox_core::types::{IpcRequest, IpcResponse};
use agentbox_db::connection::DbPool;
use agentbox_db::repo::{AgentRepo, AlertRepo, ConfigRepo, LogRepo, RunRepo};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tracing::{error, info};

pub async fn start_ipc_server(
    pool: DbPool,
    sched_tx: mpsc::Sender<SchedulerEvent>,
) -> anyhow::Result<()> {
    let socket_path = config::socket_path();
    let listener = UnixListener::bind(&socket_path)?;
    info!("IPC server listening on {}", socket_path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let pool = pool.clone();
                let tx = sched_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, pool, tx).await {
                        error!("IPC connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("IPC accept error: {}", e);
            }
        }
    }
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    pool: DbPool,
    sched_tx: mpsc::Sender<SchedulerEvent>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    while buf_reader.read_line(&mut line).await? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let response = match serde_json::from_str::<IpcRequest>(trimmed) {
            Ok(req) => handle_request(req, &pool, &sched_tx).await,
            Err(e) => IpcResponse::error(0, -32700, format!("Parse error: {}", e)),
        };

        let mut resp_json = serde_json::to_string(&response)?;
        resp_json.push('\n');
        writer.write_all(resp_json.as_bytes()).await?;

        line.clear();
    }

    Ok(())
}

async fn handle_request(
    req: IpcRequest,
    pool: &DbPool,
    sched_tx: &mpsc::Sender<SchedulerEvent>,
) -> IpcResponse {
    let agent_repo = AgentRepo::new(pool.clone());
    let run_repo = RunRepo::new(pool.clone());
    let log_repo = LogRepo::new(pool.clone());
    let alert_repo = AlertRepo::new(pool.clone());
    let config_repo = ConfigRepo::new(pool.clone());

    match req.method.as_str() {
        "agent.register" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let command = req
                .params
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let working_dir = req.params.get("working_dir").and_then(|v| v.as_str());

            if name.is_empty() || command.is_empty() {
                return IpcResponse::error(req.id, -32602, "name and command are required".into());
            }

            let reg_result = agent_repo
                .create(name, command, working_dir, None)
                .map(|a| (a.id, a.name.clone()))
                .map_err(|e| e.to_string());
            match reg_result {
                Ok((id, agent_name)) => {
                    // Apply timeout and retry settings if provided
                    if let Some(timeout) = req.params.get("timeout_secs").and_then(|v| v.as_i64()) {
                        let conn = pool.get().unwrap();
                        let _ = conn.execute(
                            "UPDATE agents SET timeout_secs = ?1 WHERE id = ?2",
                            rusqlite::params![timeout, id],
                        );
                    }
                    if let Some(max_retries) =
                        req.params.get("max_retries").and_then(|v| v.as_i64())
                    {
                        let retry_delay = req
                            .params
                            .get("retry_delay_secs")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(30);
                        let retry_strategy = req
                            .params
                            .get("retry_strategy")
                            .and_then(|v| v.as_str())
                            .unwrap_or("fixed");
                        let _ = agent_repo.update_retry_config(
                            id,
                            max_retries,
                            retry_delay,
                            retry_strategy,
                        );
                    }
                    if let Some(notify) = req
                        .params
                        .get("notify_on_success")
                        .and_then(|v| v.as_bool())
                    {
                        let conn = pool.get().unwrap();
                        let _ = conn.execute(
                            "UPDATE agents SET notify_on_success = ?1 WHERE id = ?2",
                            rusqlite::params![notify, id],
                        );
                    }
                    let _ = sched_tx.send(SchedulerEvent::Reload).await;
                    IpcResponse::success(
                        req.id,
                        serde_json::json!({
                            "id": id,
                            "name": agent_name,
                        }),
                    )
                }
                Err(e) => IpcResponse::error(req.id, -32000, e),
            }
        }

        "agent.list" => match agent_repo.list_all() {
            Ok(agents) => IpcResponse::success(req.id, serde_json::to_value(&agents).unwrap()),
            Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
        },

        "agent.run" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let agent_id_result = agent_repo
                .get_by_name(name)
                .map(|a| a.id)
                .map_err(|e| e.to_string());
            match agent_id_result {
                Ok(agent_id) => {
                    let _ = sched_tx
                        .send(SchedulerEvent::RunNow {
                            agent_id,
                            trigger: "manual".to_string(),
                        })
                        .await;
                    IpcResponse::success(
                        req.id,
                        serde_json::json!({"status": "triggered", "agent_id": agent_id}),
                    )
                }
                Err(e) => IpcResponse::error(req.id, -32000, format!("Agent not found: {}", e)),
            }
        }

        "agent.pause" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let agent_id_result = agent_repo
                .get_by_name(name)
                .map(|a| a.id)
                .map_err(|e| e.to_string());
            match agent_id_result {
                Ok(agent_id) => {
                    let _ = sched_tx.send(SchedulerEvent::Pause { agent_id }).await;
                    IpcResponse::success(req.id, serde_json::json!({"status": "paused"}))
                }
                Err(e) => IpcResponse::error(req.id, -32000, format!("Agent not found: {}", e)),
            }
        }

        "agent.resume" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let agent_id_result = agent_repo
                .get_by_name(name)
                .map(|a| a.id)
                .map_err(|e| e.to_string());
            match agent_id_result {
                Ok(agent_id) => {
                    let _ = sched_tx.send(SchedulerEvent::Resume { agent_id }).await;
                    IpcResponse::success(req.id, serde_json::json!({"status": "resumed"}))
                }
                Err(e) => IpcResponse::error(req.id, -32000, format!("Agent not found: {}", e)),
            }
        }

        "agent.edit" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name.is_empty() {
                return IpcResponse::error(req.id, -32602, "name is required".into());
            }

            let agent = match agent_repo.get_by_name(name) {
                Ok(a) => a,
                Err(e) => {
                    return IpcResponse::error(req.id, -32000, format!("Agent not found: {}", e))
                }
            };

            let conn = pool.get().unwrap();

            if let Some(command) = req.params.get("command").and_then(|v| v.as_str()) {
                if let Err(e) = conn.execute(
                    "UPDATE agents SET command = ?1 WHERE id = ?2",
                    rusqlite::params![command, agent.id],
                ) {
                    return IpcResponse::error(
                        req.id,
                        -32000,
                        format!("Failed to update command: {}", e),
                    );
                }
            }

            if let Some(working_dir) = req.params.get("working_dir").and_then(|v| v.as_str()) {
                if let Err(e) = conn.execute(
                    "UPDATE agents SET working_dir = ?1 WHERE id = ?2",
                    rusqlite::params![working_dir, agent.id],
                ) {
                    return IpcResponse::error(
                        req.id,
                        -32000,
                        format!("Failed to update working_dir: {}", e),
                    );
                }
            }

            if let Some(timeout) = req.params.get("timeout_secs").and_then(|v| v.as_i64()) {
                let timeout_val = if timeout == 0 { None } else { Some(timeout) };
                if let Err(e) = conn.execute(
                    "UPDATE agents SET timeout_secs = ?1 WHERE id = ?2",
                    rusqlite::params![timeout_val, agent.id],
                ) {
                    return IpcResponse::error(
                        req.id,
                        -32000,
                        format!("Failed to update timeout: {}", e),
                    );
                }
            }

            // Update retry config if any retry field is provided
            let has_retry_fields = req.params.get("max_retries").is_some()
                || req.params.get("retry_delay_secs").is_some()
                || req.params.get("retry_strategy").is_some();

            if has_retry_fields {
                let max_retries = req
                    .params
                    .get("max_retries")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(agent.max_retries);
                let retry_delay = req
                    .params
                    .get("retry_delay_secs")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(agent.retry_delay_secs);
                let retry_strategy = req
                    .params
                    .get("retry_strategy")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&agent.retry_strategy);

                if let Err(e) = agent_repo.update_retry_config(
                    agent.id,
                    max_retries,
                    retry_delay,
                    retry_strategy,
                ) {
                    return IpcResponse::error(
                        req.id,
                        -32000,
                        format!("Failed to update retry config: {}", e),
                    );
                }
            }

            if let Some(notify) = req
                .params
                .get("notify_on_success")
                .and_then(|v| v.as_bool())
            {
                if let Err(e) = conn.execute(
                    "UPDATE agents SET notify_on_success = ?1 WHERE id = ?2",
                    rusqlite::params![notify, agent.id],
                ) {
                    return IpcResponse::error(
                        req.id,
                        -32000,
                        format!("Failed to update notify_on_success: {}", e),
                    );
                }
            }

            let _ = sched_tx.send(SchedulerEvent::Reload).await;
            IpcResponse::success(
                req.id,
                serde_json::json!({
                    "status": "updated",
                    "name": name,
                }),
            )
        }

        "agent.remove" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let delete_result = agent_repo
                .get_by_name(name)
                .and_then(|a| agent_repo.delete(a.id))
                .map_err(|e| e.to_string());
            match delete_result {
                Ok(_) => {
                    let _ = sched_tx.send(SchedulerEvent::Reload).await;
                    IpcResponse::success(req.id, serde_json::json!({"status": "removed"}))
                }
                Err(e) => IpcResponse::error(req.id, -32000, format!("Agent not found: {}", e)),
            }
        }

        "agent.schedule" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let schedule_type = req
                .params
                .get("schedule_type")
                .and_then(|v| v.as_str())
                .unwrap_or("manual");
            let cron_expr = req.params.get("cron_expr").and_then(|v| v.as_str());
            let interval_secs = req.params.get("interval_secs").and_then(|v| v.as_i64());
            let after_agent_name = req.params.get("after_agent").and_then(|v| v.as_str());

            // Resolve after_agent_id from name
            let after_agent_id = if let Some(after_name) = after_agent_name {
                match agent_repo.get_by_name(after_name) {
                    Ok(dep) => Some(dep.id),
                    Err(e) => {
                        return IpcResponse::error(
                            req.id,
                            -32000,
                            format!("After-agent '{}' not found: {}", after_name, e),
                        )
                    }
                }
            } else {
                None
            };

            let sched_result = agent_repo
                .get_by_name(name)
                .and_then(|a| {
                    agent_repo.update_schedule(
                        a.id,
                        schedule_type,
                        cron_expr,
                        interval_secs,
                        after_agent_id,
                        None,
                    )
                })
                .map_err(|e| e.to_string());
            match sched_result {
                Ok(_) => {
                    let _ = sched_tx.send(SchedulerEvent::Reload).await;
                    IpcResponse::success(req.id, serde_json::json!({"status": "scheduled"}))
                }
                Err(e) => IpcResponse::error(req.id, -32000, format!("Schedule failed: {}", e)),
            }
        }

        "logs.tail" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let limit = req
                .params
                .get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(50);
            let all = req
                .params
                .get("all")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if all {
                match log_repo.list_all_recent(limit) {
                    Ok(logs) => IpcResponse::success(req.id, serde_json::to_value(&logs).unwrap()),
                    Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
                }
            } else {
                match agent_repo.get_by_name(name) {
                    Ok(agent) => match log_repo.list_by_agent(agent.id, limit) {
                        Ok(logs) => {
                            IpcResponse::success(req.id, serde_json::to_value(&logs).unwrap())
                        }
                        Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
                    },
                    Err(e) => IpcResponse::error(req.id, -32000, format!("Agent not found: {}", e)),
                }
            }
        }

        "runs.history" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let limit = req
                .params
                .get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(20);

            match agent_repo.get_by_name(name) {
                Ok(agent) => match run_repo.list_by_agent(agent.id, limit) {
                    Ok(runs) => IpcResponse::success(req.id, serde_json::to_value(&runs).unwrap()),
                    Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
                },
                Err(e) => IpcResponse::error(req.id, -32000, format!("Agent not found: {}", e)),
            }
        }

        // ── Alert & Config management ──
        "config.set" => {
            let key = req.params.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let value = req
                .params
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if key.is_empty() {
                return IpcResponse::error(req.id, -32602, "key is required".into());
            }
            match config_repo.set(key, value) {
                Ok(_) => IpcResponse::success(req.id, serde_json::json!({"status": "ok"})),
                Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
            }
        }

        "config.get" => {
            let key = req.params.get("key").and_then(|v| v.as_str()).unwrap_or("");
            match config_repo.get(key) {
                Ok(Some(val)) => {
                    IpcResponse::success(req.id, serde_json::json!({"key": key, "value": val}))
                }
                Ok(None) => {
                    IpcResponse::error(req.id, -32000, format!("Config key '{}' not found", key))
                }
                Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
            }
        }

        "alert.add" => {
            let channel = req
                .params
                .get("channel")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let config_val = req
                .params
                .get("config")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let config_str = config_val.to_string();

            if channel.is_empty() {
                return IpcResponse::error(req.id, -32602, "channel is required".into());
            }
            match alert_repo.add_channel(channel, &config_str) {
                Ok(ch) => IpcResponse::success(req.id, serde_json::to_value(&ch).unwrap()),
                Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
            }
        }

        "alert.list" => match alert_repo.list_all() {
            Ok(channels) => IpcResponse::success(req.id, serde_json::to_value(&channels).unwrap()),
            Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
        },

        "alert.remove" => {
            let id = req.params.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            match alert_repo.remove_channel(id) {
                Ok(_) => IpcResponse::success(req.id, serde_json::json!({"status": "removed"})),
                Err(e) => IpcResponse::error(req.id, -32000, e.to_string()),
            }
        }

        "daemon.status" => IpcResponse::success(
            req.id,
            serde_json::json!({
                "status": "running",
                "pid": std::process::id(),
            }),
        ),

        "daemon.stop" => {
            let _ = sched_tx.send(SchedulerEvent::Shutdown).await;
            IpcResponse::success(req.id, serde_json::json!({"status": "stopping"}))
        }

        _ => IpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method)),
    }
}
