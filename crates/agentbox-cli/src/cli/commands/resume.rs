use super::ipc_call;
use colored::Colorize;

pub async fn execute(name: &str) -> anyhow::Result<()> {
    let resp = ipc_call("agent.resume", serde_json::json!({"name": name})).await?;

    if resp.result.is_some() {
        println!("{} Agent '{}' resumed", "✓".green(), name.bold());
    } else if let Some(error) = resp.error {
        eprintln!("{} {}", "✗".red(), error.message);
    }

    Ok(())
}
