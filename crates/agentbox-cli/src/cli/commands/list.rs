use super::ipc_call;
use agentbox_db::models::Agent;
use colored::Colorize;
use comfy_table::{Table, ContentArrangement, presets::UTF8_FULL_CONDENSED};

pub async fn execute() -> anyhow::Result<()> {
    let resp = ipc_call("agent.list", serde_json::json!({})).await?;

    if let Some(result) = resp.result {
        let agents: Vec<Agent> = serde_json::from_value(result)?;

        if agents.is_empty() {
            println!("No agents registered. Use 'agentbox register <name> <command>' to add one.");
            return Ok(());
        }

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec!["Name", "Status", "Schedule", "Last Run", "Command"]);

        for agent in &agents {
            let status = format_status(&agent.status, agent.paused);
            let schedule = format_schedule(&agent.schedule_type, &agent.cron_expr, agent.interval_secs);
            let last_run = agent.last_run_at.as_deref().unwrap_or("never");
            let cmd = if agent.command.len() > 40 {
                format!("{}…", &agent.command[..39])
            } else {
                agent.command.clone()
            };

            table.add_row(vec![
                &agent.name,
                &status,
                &schedule,
                last_run,
                &cmd,
            ]);
        }

        println!("{table}");
    } else if let Some(error) = resp.error {
        eprintln!("{} {}", "✗".red(), error.message);
    }

    Ok(())
}

fn format_status(status: &str, paused: bool) -> String {
    if paused {
        return format!("{} paused", "⏸");
    }
    match status {
        "idle" => format!("{} idle", "✅"),
        "running" => format!("{} running", "🔄"),
        "error" => format!("{} failed", "❌"),
        other => other.to_string(),
    }
}

fn format_schedule(stype: &str, cron_expr: &Option<String>, interval_secs: Option<i64>) -> String {
    match stype {
        "cron" => cron_expr.as_deref().unwrap_or("cron").to_string(),
        "interval" => {
            if let Some(secs) = interval_secs {
                format_duration(secs)
            } else {
                "interval".to_string()
            }
        }
        "manual" => "manual".to_string(),
        other => other.to_string(),
    }
}

fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("every {}s", secs)
    } else if secs < 3600 {
        format!("every {}m", secs / 60)
    } else if secs < 86400 {
        format!("every {}h", secs / 3600)
    } else {
        format!("every {}d", secs / 86400)
    }
}
