use super::ipc_call;
use colored::Colorize;

pub async fn execute(name: &str) -> anyhow::Result<()> {
    let resp = ipc_call("agent.pause", serde_json::json!({"name": name})).await?;

    if let Some(_) = resp.result {
        println!("{} Agent '{}' paused", "✓".green(), name.bold());
    } else if let Some(error) = resp.error {
        eprintln!("{} {}", "✗".red(), error.message);
    }

    Ok(())
}
