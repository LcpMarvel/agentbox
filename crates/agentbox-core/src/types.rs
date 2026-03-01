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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(Self::Idle),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "error" => Some(Self::Error),
            _ => None,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "running" => Some(Self::Running),
            "success" => Some(Self::Success),
            "failed" => Some(Self::Failed),
            "timeout" => Some(Self::Timeout),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cron" => Some(Self::Cron),
            "interval" => Some(Self::Interval),
            "after" => Some(Self::After),
            "manual" => Some(Self::Manual),
            "api" => Some(Self::Api),
            _ => None,
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "stdout" => Some(Self::Stdout),
            "stderr" => Some(Self::Stderr),
            "system" => Some(Self::System),
            _ => None,
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
