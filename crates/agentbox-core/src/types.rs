use serde::{Deserialize, Serialize};

// ── Agent Status ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Running,
    Paused,
    Error,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Error => "error",
        }
    }
}

impl std::str::FromStr for AgentStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(Self::Idle),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "error" => Ok(Self::Error),
            _ => Err(format!("unknown agent status: {}", s)),
        }
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Run Status ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Timeout,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for RunStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(Self::Running),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "timeout" => Ok(Self::Timeout),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("unknown run status: {}", s)),
        }
    }
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Trigger Type ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerType {
    Cron,
    Interval,
    After,
    Manual,
    Api,
}

impl TriggerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cron => "cron",
            Self::Interval => "interval",
            Self::After => "after",
            Self::Manual => "manual",
            Self::Api => "api",
        }
    }
}

impl std::str::FromStr for TriggerType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cron" => Ok(Self::Cron),
            "interval" => Ok(Self::Interval),
            "after" => Ok(Self::After),
            "manual" => Ok(Self::Manual),
            "api" => Ok(Self::Api),
            _ => Err(format!("unknown trigger type: {}", s)),
        }
    }
}

impl std::fmt::Display for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Log Level ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Stdout,
    Stderr,
    System,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::System => "system",
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stdout" => Ok(Self::Stdout),
            "stderr" => Ok(Self::Stderr),
            "system" => Ok(Self::System),
            _ => Err(format!("unknown log level: {}", s)),
        }
    }
}

// ── Schedule Config ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub schedule_type: String,
    pub cron_expr: Option<String>,
    pub interval_secs: Option<i64>,
    pub after_agent_id: Option<i64>,
}

// ── IPC Messages ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl IpcRequest {
    pub fn new(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

impl IpcResponse {
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: u64, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(IpcError { code, message }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: i32,
    pub message: String,
}
