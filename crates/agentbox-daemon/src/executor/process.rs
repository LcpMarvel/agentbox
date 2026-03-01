use crate::alert::{AlertManager, AlertType};
use agentbox_db::models::Agent;
use agentbox_db::repo::{AgentRepo, LogRepo, RunRepo};
use chrono::Utc;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::error;

pub async fn run_agent(
    agent: &Agent,
    trigger: &str,
    run_repo: &RunRepo,
    _log_repo: &LogRepo,
    agent_repo: &AgentRepo,
    alert_manager: Option<&AlertManager>,
) -> anyhow::Result<()> {
    let was_error = agent.status == "error";

    // Update agent status
    agent_repo
        .update_status(agent.id, "running")
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    agent_repo
        .update_last_run(agent.id, &Utc::now().to_rfc3339())
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let result = execute_with_retries(agent, trigger, run_repo, agent_repo).await;

    match &result {
        Ok(()) => {
            // Recovery alert: was in error state, now succeeded
            if was_error {
                if let Some(am) = alert_manager {
                    am.send_alert(
                        &agent.name,
                        agent.id,
                        None,
                        AlertType::Recovery,
                        "Agent recovered and completed successfully",
                    )
                    .await;
                }
            }
            // Fallback: send desktop notification if no channels configured
            if let Some(am) = alert_manager {
                am.notify_success_fallback(&agent.name, agent.notify_on_success)
                    .await;
            }
        }
        Err(e) => {
            let err_msg = e.to_string();
            if let Some(am) = alert_manager {
                let (atype, detail) = if err_msg.contains("timed out") {
                    (
                        AlertType::Timeout,
                        format!(
                            "Process timed out after {}s",
                            agent.timeout_secs.unwrap_or(0)
                        ),
                    )
                } else {
                    (AlertType::Failure, format!("Failed: {}", err_msg))
                };
                am.send_alert(&agent.name, agent.id, None, atype, &detail)
                    .await;
            }
        }
    }

    result
}

async fn execute_with_retries(
    agent: &Agent,
    trigger: &str,
    run_repo: &RunRepo,
    agent_repo: &AgentRepo,
) -> anyhow::Result<()> {
    let max_attempts = (agent.max_retries + 1) as u32;

    for attempt in 0..max_attempts {
        let retry_count = attempt as i64;
        let result = execute_once(agent, trigger, run_repo, agent_repo, retry_count).await;

        match result {
            Ok(true) => return Ok(()), // success
            Ok(false) => {
                // failed, but may retry
                if attempt + 1 < max_attempts {
                    let delay = compute_retry_delay(agent, attempt);
                    tracing::info!(
                        "Agent '{}' failed (attempt {}/{}), retrying in {}s",
                        agent.name,
                        attempt + 1,
                        max_attempts,
                        delay
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                } else {
                    return Err(anyhow::anyhow!(
                        "Agent failed after {} attempts",
                        max_attempts
                    ));
                }
            }
            Err(e) => return Err(e), // unrecoverable error (timeout, spawn fail)
        }
    }
    unreachable!()
}

fn compute_retry_delay(agent: &Agent, attempt: u32) -> u64 {
    let base = agent.retry_delay_secs.max(1) as u64;
    match agent.retry_strategy.as_str() {
        "exponential" => base * 2u64.pow(attempt),
        _ => base, // "fixed"
    }
}

/// Returns Ok(true) for success, Ok(false) for failed (retryable), Err for unrecoverable.
async fn execute_once(
    agent: &Agent,
    trigger: &str,
    run_repo: &RunRepo,
    agent_repo: &AgentRepo,
    retry_count: i64,
) -> anyhow::Result<bool> {
    // Spawn the child process
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&agent.command);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if let Some(ref dir) = agent.working_dir {
        cmd.current_dir(dir);
    }

    if let Ok(env_map) =
        serde_json::from_str::<std::collections::HashMap<String, String>>(&agent.env_vars)
    {
        for (k, v) in env_map {
            cmd.env(k, v);
        }
    }

    unsafe {
        cmd.pre_exec(|| {
            nix::unistd::setpgid(nix::unistd::Pid::from_raw(0), nix::unistd::Pid::from_raw(0))
                .map_err(std::io::Error::other)?;
            Ok(())
        });
    }

    let mut child = cmd.spawn()?;
    let pid = child.id().map(|p| p as i64);

    // Record the run
    let run = run_repo
        .create(agent.id, trigger, pid)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if retry_count > 0 {
        // Update retry_count on the run record
        let conn_pool = agentbox_db::connection::create_pool(&agentbox_core::config::db_path())
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let conn = conn_pool.get().map_err(|e| anyhow::anyhow!("{}", e))?;
        conn.execute(
            "UPDATE runs SET retry_count = ?1 WHERE id = ?2",
            rusqlite::params![retry_count, run.id],
        )
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    let run_id = run.id;
    let agent_id = agent.id;

    // Collect stdout
    let stdout = child.stdout.take();
    let log_pool1 = agentbox_db::connection::create_pool(&agentbox_core::config::db_path())
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let log_repo_clone = agentbox_db::repo::LogRepo::new(log_pool1);
    let stdout_handle = tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Err(e) = log_repo_clone.insert(agent_id, run_id, "stdout", &line) {
                    error!("Failed to insert stdout log: {}", e);
                }
            }
        }
    });

    // Collect stderr
    let stderr = child.stderr.take();
    let log_pool2 = agentbox_db::connection::create_pool(&agentbox_core::config::db_path())
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let log_repo_clone2 = agentbox_db::repo::LogRepo::new(log_pool2);
    let stderr_handle = tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Err(e) = log_repo_clone2.insert(agent_id, run_id, "stderr", &line) {
                    error!("Failed to insert stderr log: {}", e);
                }
            }
        }
    });

    let start = std::time::Instant::now();

    // Wait with optional timeout
    let result = if let Some(timeout_secs) = agent.timeout_secs {
        tokio::select! {
            status = child.wait() => status,
            _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs as u64)) => {
                if let Some(p) = pid {
                    unsafe {
                        libc::killpg(p as i32, libc::SIGTERM);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    unsafe {
                        libc::killpg(p as i32, libc::SIGKILL);
                    }
                }
                let duration_ms = start.elapsed().as_millis() as i64;
                run_repo.finish(run_id, "timeout", None, Some("Process timed out"), duration_ms)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                agent_repo.update_status(agent.id, "error")
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                return Err(anyhow::anyhow!("Process timed out"));
            }
        }
    } else {
        child.wait().await
    };

    let _ = stdout_handle.await;
    let _ = stderr_handle.await;

    let duration_ms = start.elapsed().as_millis() as i64;

    match result {
        Ok(status) => {
            let exit_code = status.code();
            if status.success() {
                run_repo
                    .finish(run_id, "success", exit_code, None, duration_ms)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                agent_repo
                    .update_status(agent.id, "idle")
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                Ok(true)
            } else {
                run_repo
                    .finish(run_id, "failed", exit_code, None, duration_ms)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                agent_repo
                    .update_status(agent.id, "error")
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                Ok(false)
            }
        }
        Err(e) => {
            run_repo
                .finish(run_id, "failed", None, Some(&e.to_string()), duration_ms)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            agent_repo
                .update_status(agent.id, "error")
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            Err(anyhow::anyhow!("{}", e))
        }
    }
}

extern crate libc;
