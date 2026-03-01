use rmcp::model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Parameter types ──

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RegisterAgentParams {
    /// Unique name for the agent
    pub name: String,
    /// Shell command to execute
    pub command: String,
    /// Working directory for the command. Defaults to the current directory if not specified. Always set this for commands using relative paths.
    pub dir: Option<String>,
    /// Timeout in seconds (optional)
    pub timeout: Option<i64>,
    /// Max retries on failure (default: 0)
    pub retry: Option<i64>,
    /// Retry delay in seconds (default: 30)
    pub retry_delay: Option<i64>,
    /// Retry strategy: "fixed" or "exponential" (default: "fixed")
    pub retry_strategy: Option<String>,
    /// Send desktop notification on successful completion (default: true)
    pub notify_on_success: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct EditAgentParams {
    /// Agent name to edit
    pub name: String,
    /// New shell command to execute
    pub command: Option<String>,
    /// New working directory
    pub dir: Option<String>,
    /// New timeout in seconds (0 to remove)
    pub timeout: Option<i64>,
    /// Max retries on failure
    pub retry: Option<i64>,
    /// Retry delay in seconds
    pub retry_delay: Option<i64>,
    /// Retry strategy: "fixed" or "exponential"
    pub retry_strategy: Option<String>,
    /// Send desktop notification on successful completion
    pub notify_on_success: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RunAgentParams {
    /// Agent name to run
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ScheduleAgentParams {
    /// Agent name
    pub name: String,
    /// Cron expression (e.g. "0 18 * * *" for daily at 6pm)
    pub cron: Option<String>,
    /// Run every interval (e.g. "30m", "2h", "1d")
    pub every: Option<String>,
    /// Run after another agent completes (agent name)
    pub after: Option<String>,
    /// Reset to manual mode (no automatic schedule)
    pub manual: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AgentNameParams {
    /// Agent name
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetLogsParams {
    /// Agent name
    pub name: String,
    /// Number of recent log lines to return (default: 50)
    pub tail: Option<i64>,
    /// Filter by log level: "stdout", "stderr", or "system"
    pub level: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetHistoryParams {
    /// Agent name
    pub name: String,
    /// Max number of runs to return (default: 10)
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetConfigParams {
    /// Config key to retrieve. Omit to get all config.
    pub key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetConfigParams {
    /// Config key
    pub key: String,
    /// Config value
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ManageAlertsParams {
    /// Action: "add", "list", or "remove"
    pub action: String,
    /// Alert channel type (e.g. "telegram", "webhook", "wecom", "email"). Required for add/remove.
    pub channel: Option<String>,
    /// Channel config as JSON string (e.g. bot_token, chat_id for telegram). Required for add.
    pub config: Option<String>,
}

pub fn text_result(text: String) -> Result<CallToolResult, rmcp::ErrorData> {
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub fn err_result(msg: String) -> Result<CallToolResult, rmcp::ErrorData> {
    Ok(CallToolResult::error(vec![Content::text(msg)]))
}
