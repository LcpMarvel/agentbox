use super::ipc_call;
use agentbox_db::models::Run;
use colored::Colorize;
use comfy_table::{Table, ContentArrangement, presets::UTF8_FULL_CONDENSED};

pub async fn execute(name: &str, limit: i64) -> anyhow::Result<()> {
    let resp = ipc_call("runs.history", serde_json::json!({"name": name, "limit": limit})).await?;

    if let Some(result) = resp.result {
        let runs: Vec<Run> = serde_json::from_value(result)?;

        if runs.is_empty() {
            println!("No run history for '{}'.", name);
            return Ok(());
        }

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Run ID", "Status", "Trigger", "Started", "Duration", "Exit Code"]);

        for run in &runs {
            let status = match run.status.as_str() {
                "success" => "✅ success".to_string(),
                "failed" => "❌ failed".to_string(),
                "running" => "🔄 running".to_string(),
                "timeout" => "⏱ timeout".to_string(),
                other => other.to_string(),
            };
            let duration = run.duration_ms
                .map(|ms| format!("{:.1}s", ms as f64 / 1000.0))
                .unwrap_or_else(|| "—".to_string());
            let exit = run.exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "—".to_string());

            table.add_row(vec![
                &run.id.to_string(),
                &status,
                &run.trigger_type,
                &run.started_at,
                &duration,
                &exit,
            ]);
        }

        println!("{table}");
    } else if let Some(error) = resp.error {
        eprintln!("{} {}", "✗".red(), error.message);
    }

    Ok(())
}
