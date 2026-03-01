use super::ipc_call;
use colored::Colorize;

#[allow(clippy::too_many_arguments)]
pub async fn execute(
    name: &str,
    command: &str,
    working_dir: Option<&str>,
    timeout: Option<i64>,
    max_retries: i64,
    retry_delay: i64,
    retry_strategy: &str,
    notify_on_success: Option<bool>,
) -> anyhow::Result<()> {
    // If no working dir specified, capture caller's current directory
    // so relative paths in commands (e.g. "python ./test.py") resolve correctly
    let effective_dir = working_dir.map(|d| d.to_string()).or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    });

    let mut params = serde_json::json!({
        "name": name,
        "command": command,
    });

    if let Some(ref dir) = effective_dir {
        params["working_dir"] = serde_json::json!(dir);
    }
    if let Some(t) = timeout {
        params["timeout_secs"] = serde_json::json!(t);
    }
    if max_retries > 0 {
        params["max_retries"] = serde_json::json!(max_retries);
        params["retry_delay_secs"] = serde_json::json!(retry_delay);
        params["retry_strategy"] = serde_json::json!(retry_strategy);
    }
    if let Some(notify) = notify_on_success {
        params["notify_on_success"] = serde_json::json!(notify);
    }

    let resp = ipc_call("agent.register", params).await?;

    if let Some(result) = resp.result {
        let agent_name = result["name"].as_str().unwrap_or(name);
        println!("{} Agent '{}' registered", "✓".green(), agent_name.bold());
        println!("  Run it:      agentbox run {}", name);
        println!("  Schedule it: agentbox schedule {} \"0 18 * * *\"", name);
    } else if let Some(error) = resp.error {
        eprintln!("{} {}", "✗".red(), error.message);
    }

    Ok(())
}
