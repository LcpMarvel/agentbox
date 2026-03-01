use super::ipc_call;
use agentbox_db::models::LogEntry;
use colored::Colorize;

pub async fn execute(name: Option<&str>, all: bool, tail: i64) -> anyhow::Result<()> {
    let params = if all || name.is_none() {
        serde_json::json!({"all": true, "limit": tail})
    } else {
        serde_json::json!({"name": name.unwrap(), "limit": tail})
    };

    let resp = ipc_call("logs.tail", params).await?;

    if let Some(result) = resp.result {
        let logs: Vec<LogEntry> = serde_json::from_value(result)?;

        if logs.is_empty() {
            println!("No logs found.");
            return Ok(());
        }

        // Show oldest first
        for log in logs.iter().rev() {
            let level_str = match log.level.as_str() {
                "stderr" => "ERR".red().to_string(),
                "system" => "SYS".yellow().to_string(),
                _ => "OUT".normal().to_string(),
            };
            println!("{} [{}] {}", log.created_at.dimmed(), level_str, log.message);
        }
    } else if let Some(error) = resp.error {
        eprintln!("{} {}", "✗".red(), error.message);
    }

    Ok(())
}
