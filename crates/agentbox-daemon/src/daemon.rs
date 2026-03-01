use agentbox_core::config;
use std::fs;
use std::io::{Read, Write};
use tracing::info;

/// Write current PID to the PID file.
pub fn write_pid_file() -> std::io::Result<()> {
    let pid = std::process::id();
    let path = config::pid_file_path();
    let mut f = fs::File::create(&path)?;
    write!(f, "{}", pid)?;
    info!("PID file written: {} (pid={})", path.display(), pid);
    Ok(())
}

/// Read PID from the PID file, if it exists.
pub fn read_pid_file() -> Option<u32> {
    let path = config::pid_file_path();
    if !path.exists() {
        return None;
    }
    let mut contents = String::new();
    fs::File::open(&path)
        .ok()?
        .read_to_string(&mut contents)
        .ok()?;
    contents.trim().parse().ok()
}

/// Remove the PID file.
pub fn remove_pid_file() {
    let path = config::pid_file_path();
    fs::remove_file(&path).ok();
}

/// Check if daemon is running by reading PID file and checking process.
pub fn is_daemon_running() -> bool {
    if let Some(pid) = read_pid_file() {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    } else {
        false
    }
}

/// Remove stale socket file if it exists.
pub fn cleanup_socket() {
    let path = config::socket_path();
    if path.exists() {
        fs::remove_file(&path).ok();
    }
}

/// Start the daemon (foreground mode — used after fork or for debugging).
pub async fn run_daemon() -> anyhow::Result<()> {
    write_pid_file()?;
    cleanup_socket();

    let db_pool = agentbox_db::connection::create_pool(&config::db_path())
        .map_err(|e| anyhow::anyhow!("Failed to create DB pool: {}", e))?;

    // Channels for scheduler events
    let (sched_tx, sched_rx) = tokio::sync::mpsc::channel(256);

    // Create shared components
    let alert_manager = crate::alert::AlertManager::new(db_pool.clone());

    let scheduler = crate::scheduler::SchedulerEngine::new(
        agentbox_db::repo::AgentRepo::new(db_pool.clone()),
        agentbox_db::repo::RunRepo::new(db_pool.clone()),
        agentbox_db::repo::LogRepo::new(db_pool.clone()),
        agentbox_db::repo::ConfigRepo::new(db_pool.clone()),
        alert_manager,
        sched_tx.clone(),
    );

    let _scheduler_handle = tokio::spawn(async move {
        scheduler.run(sched_rx).await;
    });

    // Start web server
    let web_pool = db_pool.clone();
    let _web_handle = tokio::spawn(async move {
        if let Err(e) = agentbox_web::server::start_server(web_pool).await {
            tracing::error!("Web server error: {}", e);
        }
    });

    // Start IPC server
    let ipc_pool = db_pool.clone();
    let _ipc_handle = tokio::spawn(async move {
        if let Err(e) = crate::ipc::server::start_ipc_server(ipc_pool, sched_tx).await {
            tracing::error!("IPC server error: {}", e);
        }
    });

    info!("AgentBox daemon started (pid={})", std::process::id());

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down daemon...");

    remove_pid_file();
    cleanup_socket();

    Ok(())
}

extern crate libc;
