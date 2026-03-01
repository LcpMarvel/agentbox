use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: i64,
    pub name: String,
    pub command: String,
    pub working_dir: Option<String>,
    pub env_vars: String,
    pub schedule_type: String,
    pub cron_expr: Option<String>,
    pub interval_secs: Option<i64>,
    pub after_agent_id: Option<i64>,
    pub status: String,
    pub paused: bool,
    pub timeout_secs: Option<i64>,
    pub max_retries: i64,
    pub created_at: String,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub retry_delay_secs: i64,
    pub retry_strategy: String,
    pub notify_on_success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: i64,
    pub agent_id: i64,
    pub status: String,
    pub trigger_type: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub pid: Option<i64>,
    pub retry_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: i64,
    pub agent_id: i64,
    pub run_id: i64,
    pub level: String,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertChannel {
    pub id: i64,
    pub channel: String,
    pub config: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistory {
    pub id: i64,
    pub agent_id: i64,
    pub run_id: Option<i64>,
    pub alert_type: String,
    pub channel: String,
    pub message: String,
    pub sent_at: String,
}
