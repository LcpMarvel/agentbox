use crate::ipc_client::IpcClient;
use crate::resources;
use crate::tools::*;
use agentbox_core::upgrade::{self, UpgradeInfo};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::{tool, tool_handler, tool_router, RoleServer, ServerHandler, ServiceExt};
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Clone)]
pub struct AgentBoxMcpServer {
    tool_router: ToolRouter<Self>,
    upgrade_cache: Arc<OnceCell<Option<UpgradeInfo>>>,
}

impl AgentBoxMcpServer {
    /// Get the cached upgrade notice string, if an update is available.
    fn upgrade_notice(&self) -> Option<String> {
        self.upgrade_cache
            .get()
            .and_then(|info| info.as_ref())
            .filter(|info| info.has_update)
            .map(|info| {
                format!(
                    "[NOTICE] AgentBox update available: v{} → v{}. \
                     Please remind the user to run `agentbox upgrade` to update.",
                    info.current, info.latest
                )
            })
    }

    /// Wrap a successful text result with upgrade notice.
    fn ok(&self, text: String) -> Result<CallToolResult, rmcp::ErrorData> {
        text_result_with_upgrade(text, self.upgrade_notice().as_deref())
    }

    /// Wrap an error result with upgrade notice.
    fn err(&self, msg: String) -> Result<CallToolResult, rmcp::ErrorData> {
        err_result_with_upgrade(msg, self.upgrade_notice().as_deref())
    }
}

#[tool_router]
impl AgentBoxMcpServer {
    pub fn new() -> Self {
        let upgrade_cache = Arc::new(OnceCell::new());

        // Spawn background upgrade check
        let cache = upgrade_cache.clone();
        tokio::spawn(async move {
            let result = upgrade::check_latest_version().await.ok();
            let _ = cache.set(result);
        });

        Self {
            tool_router: Self::tool_router(),
            upgrade_cache,
        }
    }

    // ── Agent lifecycle ──

    #[tool(
        description = "List all registered agents with their current status, schedule, last run time, and duration. Returns a JSON array of agent objects."
    )]
    async fn list_agents(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        match IpcClient::call_ok("agent.list", serde_json::json!({})).await {
            Ok(val) => self.ok(serde_json::to_string_pretty(&val).unwrap_or_default()),
            Err(e) => self.err(e),
        }
    }

    #[tool(
        description = "Register a new agent. An agent is a shell command that AgentBox will manage. After registering, use schedule_agent to set when it runs. IMPORTANT: Always provide 'dir' (working directory) if the command depends on relative paths or specific project files — otherwise the command may fail when executed by the daemon."
    )]
    async fn register_agent(
        &self,
        params: Parameters<RegisterAgentParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let mut ipc_params = serde_json::json!({
            "name": params.name,
            "command": params.command,
            "working_dir": params.dir,
            "timeout_secs": params.timeout,
            "max_retries": params.retry.unwrap_or(0),
            "retry_delay_secs": params.retry_delay.unwrap_or(30),
            "retry_strategy": params.retry_strategy.unwrap_or_else(|| "fixed".to_string()),
        });
        if let Some(v) = params.notify_on_success {
            ipc_params["notify_on_success"] = serde_json::json!(v);
        }

        match IpcClient::call_ok("agent.register", ipc_params).await {
            Ok(val) => self.ok(format!(
                "Agent '{}' registered successfully.\n{}",
                params.name,
                serde_json::to_string_pretty(&val).unwrap_or_default()
            )),
            Err(e) => self.err(format!("Failed to register agent: {}", e)),
        }
    }

    #[tool(
        description = "Edit an existing agent's command, working directory, timeout, or retry configuration. Provide only the fields you want to change; omitted fields remain unchanged."
    )]
    async fn edit_agent(
        &self,
        params: Parameters<EditAgentParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let mut ipc_params = serde_json::json!({ "name": params.name });
        let obj = ipc_params.as_object_mut().unwrap();

        if let Some(v) = &params.command {
            obj.insert("command".into(), serde_json::json!(v));
        }
        if let Some(v) = &params.dir {
            obj.insert("working_dir".into(), serde_json::json!(v));
        }
        if let Some(v) = params.timeout {
            obj.insert("timeout_secs".into(), serde_json::json!(v));
        }
        if let Some(v) = params.retry {
            obj.insert("max_retries".into(), serde_json::json!(v));
        }
        if let Some(v) = params.retry_delay {
            obj.insert("retry_delay_secs".into(), serde_json::json!(v));
        }
        if let Some(v) = &params.retry_strategy {
            obj.insert("retry_strategy".into(), serde_json::json!(v));
        }
        if let Some(v) = params.notify_on_success {
            obj.insert("notify_on_success".into(), serde_json::json!(v));
        }

        match IpcClient::call_ok("agent.edit", ipc_params).await {
            Ok(val) => self.ok(format!(
                "Agent '{}' updated.\n{}",
                params.name,
                serde_json::to_string_pretty(&val).unwrap_or_default()
            )),
            Err(e) => self.err(format!("Failed to edit agent: {}", e)),
        }
    }

    #[tool(
        description = "Manually trigger an agent to run immediately, regardless of its schedule. Returns the run ID."
    )]
    async fn run_agent(
        &self,
        params: Parameters<RunAgentParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let ipc_params = serde_json::json!({ "name": params.name });
        match IpcClient::call_ok("agent.run", ipc_params).await {
            Ok(val) => self.ok(format!(
                "Agent '{}' triggered.\n{}",
                params.name,
                serde_json::to_string_pretty(&val).unwrap_or_default()
            )),
            Err(e) => self.err(format!("Failed to run agent: {}", e)),
        }
    }

    #[tool(
        description = "Set or update the schedule for an agent. Provide exactly one of: cron (cron expression like '0 18 * * *'), every (interval like '30m', '2h', '1d'), after (run after another agent completes), or manual (disable automatic scheduling)."
    )]
    async fn schedule_agent(
        &self,
        params: Parameters<ScheduleAgentParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;

        // Build IPC params matching what the daemon expects
        let ipc_params = if params.manual.unwrap_or(false) {
            serde_json::json!({
                "name": params.name,
                "schedule_type": "manual",
            })
        } else if let Some(cron_expr) = &params.cron {
            // Users provide 5-field cron; daemon expects 6-field (with seconds)
            let full_cron = if cron_expr.split_whitespace().count() == 5 {
                format!("0 {}", cron_expr)
            } else {
                cron_expr.clone()
            };
            serde_json::json!({
                "name": params.name,
                "schedule_type": "cron",
                "cron_expr": full_cron,
            })
        } else if let Some(every) = &params.every {
            match parse_interval(every) {
                Ok(secs) => serde_json::json!({
                    "name": params.name,
                    "schedule_type": "interval",
                    "interval_secs": secs,
                }),
                Err(e) => return self.err(format!("Invalid interval '{}': {}", every, e)),
            }
        } else if let Some(after_name) = &params.after {
            serde_json::json!({
                "name": params.name,
                "schedule_type": "after",
                "after_agent": after_name,
            })
        } else {
            return self.err("Provide one of: cron, every, after, or manual".to_string());
        };

        match IpcClient::call_ok("agent.schedule", ipc_params).await {
            Ok(val) => self.ok(format!(
                "Schedule updated for agent '{}'.\n{}",
                params.name,
                serde_json::to_string_pretty(&val).unwrap_or_default()
            )),
            Err(e) => self.err(format!("Failed to set schedule: {}", e)),
        }
    }

    #[tool(
        description = "Pause an agent's automatic schedule. The agent will not run until resumed. Manual runs via run_agent are still possible."
    )]
    async fn pause_agent(
        &self,
        params: Parameters<AgentNameParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let ipc_params = serde_json::json!({ "name": params.name });
        match IpcClient::call_ok("agent.pause", ipc_params).await {
            Ok(_) => self.ok(format!("Agent '{}' paused.", params.name)),
            Err(e) => self.err(format!("Failed to pause agent: {}", e)),
        }
    }

    #[tool(description = "Resume an agent's automatic schedule after it has been paused.")]
    async fn resume_agent(
        &self,
        params: Parameters<AgentNameParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let ipc_params = serde_json::json!({ "name": params.name });
        match IpcClient::call_ok("agent.resume", ipc_params).await {
            Ok(_) => self.ok(format!("Agent '{}' resumed.", params.name)),
            Err(e) => self.err(format!("Failed to resume agent: {}", e)),
        }
    }

    #[tool(description = "Permanently remove an agent and all its run history and logs.")]
    async fn remove_agent(
        &self,
        params: Parameters<AgentNameParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let ipc_params = serde_json::json!({ "name": params.name });
        match IpcClient::call_ok("agent.remove", ipc_params).await {
            Ok(_) => self.ok(format!("Agent '{}' removed.", params.name)),
            Err(e) => self.err(format!("Failed to remove agent: {}", e)),
        }
    }

    // ── Observability ──

    #[tool(
        description = "Get recent logs for an agent. Returns stdout/stderr output from recent runs. Use 'tail' to control how many lines to return (default 50). Optionally filter by 'level': stdout, stderr, or system."
    )]
    async fn get_agent_logs(
        &self,
        params: Parameters<GetLogsParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let ipc_params = serde_json::json!({
            "name": params.name,
            "tail": params.tail.unwrap_or(50),
            "level": params.level,
        });

        match IpcClient::call_ok("logs.tail", ipc_params).await {
            Ok(val) => {
                if let Some(logs) = val.as_array() {
                    let formatted: Vec<String> = logs
                        .iter()
                        .filter_map(|entry| {
                            let ts = entry.get("created_at")?.as_str()?;
                            let level = entry.get("level")?.as_str()?;
                            let msg = entry.get("message")?.as_str()?;
                            Some(format!("[{}] [{}] {}", ts, level, msg))
                        })
                        .collect();
                    self.ok(if formatted.is_empty() {
                        "No logs found.".to_string()
                    } else {
                        formatted.join("\n")
                    })
                } else {
                    self.ok(serde_json::to_string_pretty(&val).unwrap_or_default())
                }
            }
            Err(e) => self.err(format!("Failed to get logs: {}", e)),
        }
    }

    #[tool(
        description = "Get run history for an agent. Shows recent executions with start time, status (success/failed/timeout), duration, and exit code. Use 'limit' to control how many runs to return (default 10)."
    )]
    async fn get_run_history(
        &self,
        params: Parameters<GetHistoryParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let ipc_params = serde_json::json!({
            "name": params.name,
            "limit": params.limit.unwrap_or(10),
        });

        match IpcClient::call_ok("runs.history", ipc_params).await {
            Ok(val) => self.ok(serde_json::to_string_pretty(&val).unwrap_or_default()),
            Err(e) => self.err(format!("Failed to get history: {}", e)),
        }
    }

    #[tool(
        description = "Get global dashboard statistics: total agents, running count, error count, paused count, today's run count, and success rate."
    )]
    async fn get_dashboard_stats(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        match IpcClient::call_ok("daemon.status", serde_json::json!({})).await {
            Ok(val) => self.ok(serde_json::to_string_pretty(&val).unwrap_or_default()),
            Err(e) => self.err(format!("Failed to get stats: {}", e)),
        }
    }

    // ── Configuration ──

    #[tool(
        description = "Get a configuration value. Provide 'key' to get a specific setting, or omit to list all configuration."
    )]
    async fn get_config(
        &self,
        params: Parameters<GetConfigParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let ipc_params = match &params.key {
            Some(k) => serde_json::json!({ "key": k }),
            None => serde_json::json!({ "all": true }),
        };

        match IpcClient::call_ok("config.get", ipc_params).await {
            Ok(val) => self.ok(serde_json::to_string_pretty(&val).unwrap_or_default()),
            Err(e) => self.err(format!("Failed to get config: {}", e)),
        }
    }

    #[tool(description = "Set a configuration value.")]
    async fn set_config(
        &self,
        params: Parameters<SetConfigParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        let ipc_params = serde_json::json!({
            "key": params.key,
            "value": params.value,
        });

        match IpcClient::call_ok("config.set", ipc_params).await {
            Ok(_) => self.ok(format!("Config '{}' updated.", params.key)),
            Err(e) => self.err(format!("Failed to set config: {}", e)),
        }
    }

    #[tool(
        description = "Manage alert notification channels. Actions: 'add' (create a new channel), 'list' (show all channels), 'remove' (delete a channel). Supported channel types: telegram, webhook, wecom, email."
    )]
    async fn manage_alerts(
        &self,
        params: Parameters<ManageAlertsParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let params = params.0;
        match params.action.as_str() {
            "list" => match IpcClient::call_ok("alert.list", serde_json::json!({})).await {
                Ok(val) => self.ok(serde_json::to_string_pretty(&val).unwrap_or_default()),
                Err(e) => self.err(format!("Failed to list alerts: {}", e)),
            },
            "add" => {
                let channel = params.channel.as_deref().unwrap_or("");
                let config = params.config.as_deref().unwrap_or("{}");
                let ipc_params = serde_json::json!({
                    "channel": channel,
                    "config": config,
                });
                match IpcClient::call_ok("alert.add", ipc_params).await {
                    Ok(_) => self.ok(format!("Alert channel '{}' added.", channel)),
                    Err(e) => self.err(format!("Failed to add alert: {}", e)),
                }
            }
            "remove" => {
                let channel = params.channel.as_deref().unwrap_or("");
                let ipc_params = serde_json::json!({ "channel": channel });
                match IpcClient::call_ok("alert.remove", ipc_params).await {
                    Ok(_) => self.ok(format!("Alert channel '{}' removed.", channel)),
                    Err(e) => self.err(format!("Failed to remove alert: {}", e)),
                }
            }
            other => self.err(format!(
                "Unknown action '{}'. Use 'add', 'list', or 'remove'.",
                other
            )),
        }
    }

    // ── Upgrade ──

    #[tool(
        description = "Check if a newer version of AgentBox is available. Returns current version, latest version, and whether an update is available."
    )]
    async fn check_upgrade(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        match upgrade::check_latest_version().await {
            Ok(info) => {
                let msg = if info.has_update {
                    format!(
                        "Update available: v{} → v{}\nRun `agentbox upgrade` to update.",
                        info.current, info.latest
                    )
                } else {
                    format!("AgentBox v{} is up to date.", info.current)
                };
                text_result(msg)
            }
            Err(e) => err_result(format!("Failed to check for updates: {}", e)),
        }
    }
}

#[tool_handler]
impl ServerHandler for AgentBoxMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "agentbox".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
            instructions: Some(
                "AgentBox MCP Server — manage your local AI agents. \
                 You can register agents (shell commands), schedule them with cron expressions, \
                 run them manually, view logs and run history, and manage alerts. \
                 Use list_agents to see all agents, register_agent to create new ones, \
                 and schedule_agent to set up automatic scheduling."
                    .to_string(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let resources_list = resources::list_resources().await;
        Ok(ListResourcesResult {
            resources: resources_list,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        resources::read_resource(&request.uri).await
    }
}

/// Parse a human-friendly interval string (e.g. "30m", "2h", "1d") into seconds.
fn parse_interval(s: &str) -> Result<i64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty interval".to_string());
    }

    let (num_str, multiplier) = if let Some(n) = s.strip_suffix('s') {
        (n, 1)
    } else if let Some(n) = s.strip_suffix('m') {
        (n, 60)
    } else if let Some(n) = s.strip_suffix('h') {
        (n, 3600)
    } else if let Some(n) = s.strip_suffix('d') {
        (n, 86400)
    } else {
        (s, 1) // default to seconds
    };

    let num: i64 = num_str
        .parse()
        .map_err(|_| format!("invalid number: {}", num_str))?;
    Ok(num * multiplier)
}

/// Entry point: start the MCP server on stdio.
pub async fn run_server() -> anyhow::Result<()> {
    let server = AgentBoxMcpServer::new();
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
