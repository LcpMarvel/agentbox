use super::ipc_call;
use colored::Colorize;

pub async fn execute(
    name: &str,
    command: Option<String>,
    dir: Option<String>,
    timeout: Option<i64>,
    retry: Option<i64>,
    retry_delay: Option<i64>,
    retry_strategy: Option<String>,
) -> anyhow::Result<()> {
    if command.is_none()
        && dir.is_none()
        && timeout.is_none()
        && retry.is_none()
        && retry_delay.is_none()
        && retry_strategy.is_none()
    {
        anyhow::bail!(
            "Nothing to update. Specify at least one of: --command, --dir, --timeout, --retry, --retry-delay, --retry-strategy"
        );
    }

    let mut params = serde_json::json!({ "name": name });
    let obj = params.as_object_mut().unwrap();

    if let Some(v) = &command {
        obj.insert("command".into(), serde_json::json!(v));
    }
    if let Some(v) = &dir {
        obj.insert("working_dir".into(), serde_json::json!(v));
    }
    if let Some(v) = timeout {
        obj.insert("timeout_secs".into(), serde_json::json!(v));
    }
    if let Some(v) = retry {
        obj.insert("max_retries".into(), serde_json::json!(v));
    }
    if let Some(v) = retry_delay {
        obj.insert("retry_delay_secs".into(), serde_json::json!(v));
    }
    if let Some(v) = &retry_strategy {
        obj.insert("retry_strategy".into(), serde_json::json!(v));
    }

    let resp = ipc_call("agent.edit", params).await?;

    if let Some(result) = resp.result {
        let status = result
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("ok");
        println!("{} Agent '{}' {}", "✓".green(), name.bold(), status);
    } else if let Some(e) = resp.error {
        eprintln!("{} {}", "✗".red(), e.message);
    }

    Ok(())
}
