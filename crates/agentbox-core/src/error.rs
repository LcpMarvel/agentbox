use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentBoxError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent already exists: {0}")]
    AgentAlreadyExists(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Daemon already running")]
    DaemonAlreadyRunning,

    #[error("Invalid schedule: {0}")]
    InvalidSchedule(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AgentBoxError>;
