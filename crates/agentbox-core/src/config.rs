use std::path::PathBuf;

/// Returns ~/.agentbox/ directory, creating it if it doesn't exist.
pub fn agentbox_dir() -> PathBuf {
    let dir = dirs::home_dir()
        .expect("Cannot determine home directory")
        .join(".agentbox");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Returns path to the SQLite database: ~/.agentbox/agentbox.db
pub fn db_path() -> PathBuf {
    agentbox_dir().join("agentbox.db")
}

/// Returns path to the daemon socket: ~/.agentbox/daemon.sock
pub fn socket_path() -> PathBuf {
    agentbox_dir().join("daemon.sock")
}

/// Returns path to the daemon PID file: ~/.agentbox/daemon.pid
pub fn pid_file_path() -> PathBuf {
    agentbox_dir().join("daemon.pid")
}

/// Returns path to the daemon log: ~/.agentbox/daemon.log
pub fn daemon_log_path() -> PathBuf {
    agentbox_dir().join("daemon.log")
}

/// Default web dashboard port
pub const DEFAULT_WEB_PORT: u16 = 9800;
