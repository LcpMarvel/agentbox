use super::ipc_call;
use colored::Colorize;

pub async fn execute(
    name: &str,
    cron: Option<String>,
    every: Option<String>,
    after: Option<String>,
    manual: bool,
) -> anyhow::Result<()> {
    let params = if manual {
        serde_json::json!({
            "name": name,
            "schedule_type": "manual",
        })
    } else if let Some(cron_expr) = cron {
        // cron crate expects 7-field expressions (sec min hour dom mon dow year)
        // Users provide 5-field, so we prepend "0 " for seconds
        let full_cron = if cron_expr.split_whitespace().count() == 5 {
            format!("0 {}", cron_expr)
        } else {
            cron_expr.clone()
        };

        // Validate
        if full_cron.parse::<cron::Schedule>().is_err() {
            anyhow::bail!("Invalid cron expression: {}", cron_expr);
        }

        serde_json::json!({
            "name": name,
            "schedule_type": "cron",
            "cron_expr": full_cron,
        })
    } else if let Some(interval) = every {
        let secs = parse_interval(&interval)?;
        serde_json::json!({
            "name": name,
            "schedule_type": "interval",
            "interval_secs": secs,
        })
    } else if let Some(after_name) = after {
        serde_json::json!({
            "name": name,
            "schedule_type": "after",
            "after_agent": after_name,
        })
    } else {
        anyhow::bail!("Provide a cron expression, --every interval, --after agent, or --manual");
    };

    let resp = ipc_call("agent.schedule", params).await?;

    if let Some(_) = resp.result {
        println!("{} Schedule updated for '{}'", "✓".green(), name.bold());
    } else if let Some(error) = resp.error {
        eprintln!("{} {}", "✗".red(), error.message);
    }

    Ok(())
}

fn parse_interval(s: &str) -> anyhow::Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty interval");
    }

    let (num_str, unit) = if s.ends_with('s') {
        (&s[..s.len() - 1], "s")
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], "m")
    } else if s.ends_with('h') {
        (&s[..s.len() - 1], "h")
    } else if s.ends_with('d') {
        (&s[..s.len() - 1], "d")
    } else {
        (s, "s") // default to seconds
    };

    let num: i64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid interval number: {}", num_str))?;

    Ok(match unit {
        "s" => num,
        "m" => num * 60,
        "h" => num * 3600,
        "d" => num * 86400,
        _ => num,
    })
}
