use super::ipc_call;
use colored::Colorize;

pub async fn execute(args: Vec<String>) -> anyhow::Result<()> {
    if args.is_empty() {
        anyhow::bail!("Usage: agentbox config <subcommand>\n\nSubcommands:\n  set <key> <value>      Set a config value\n  get <key>              Get a config value\n  max_concurrent <N>     Set max concurrent agents\n  alert.webhook <url>    Add webhook alert channel\n  alert.telegram <token> <chat_id>  Add Telegram alert\n  alert.macos            Enable macOS notifications\n  alert.list             List alert channels\n  alert.remove <id>      Remove an alert channel");
    }

    match args[0].as_str() {
        "set" => {
            if args.len() < 3 {
                anyhow::bail!("Usage: agentbox config set <key> <value>");
            }
            let resp = ipc_call(
                "config.set",
                serde_json::json!({
                    "key": args[1],
                    "value": args[2],
                }),
            )
            .await?;
            if resp.result.is_some() {
                println!(
                    "{} Config set: {} = {}",
                    "✓".green(),
                    args[1].bold(),
                    args[2]
                );
            } else if let Some(e) = resp.error {
                eprintln!("{} {}", "✗".red(), e.message);
            }
        }
        "get" => {
            if args.len() < 2 {
                anyhow::bail!("Usage: agentbox config get <key>");
            }
            let resp = ipc_call(
                "config.get",
                serde_json::json!({
                    "key": args[1],
                }),
            )
            .await?;
            if let Some(result) = resp.result {
                let val = result.get("value").and_then(|v| v.as_str()).unwrap_or("");
                println!("{} = {}", args[1], val);
            } else if let Some(e) = resp.error {
                eprintln!("{} {}", "✗".red(), e.message);
            }
        }
        "max_concurrent" => {
            if args.len() < 2 {
                anyhow::bail!("Usage: agentbox config max_concurrent <N>");
            }
            let _n: usize = args[1]
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid number: {}", args[1]))?;
            let resp = ipc_call(
                "config.set",
                serde_json::json!({
                    "key": "max_concurrent",
                    "value": args[1],
                }),
            )
            .await?;
            if resp.result.is_some() {
                println!(
                    "{} Max concurrent agents set to {}",
                    "✓".green(),
                    args[1].bold()
                );
            } else if let Some(e) = resp.error {
                eprintln!("{} {}", "✗".red(), e.message);
            }
        }
        "alert.webhook" => {
            if args.len() < 2 {
                anyhow::bail!("Usage: agentbox config alert.webhook <url>");
            }
            let resp = ipc_call(
                "alert.add",
                serde_json::json!({
                    "channel": "webhook",
                    "config": {"url": args[1]},
                }),
            )
            .await?;
            if resp.result.is_some() {
                println!("{} Webhook alert channel added: {}", "✓".green(), args[1]);
            } else if let Some(e) = resp.error {
                eprintln!("{} {}", "✗".red(), e.message);
            }
        }
        "alert.telegram" => {
            if args.len() < 3 {
                anyhow::bail!("Usage: agentbox config alert.telegram <bot_token> <chat_id>");
            }
            let resp = ipc_call(
                "alert.add",
                serde_json::json!({
                    "channel": "telegram",
                    "config": {"bot_token": args[1], "chat_id": args[2]},
                }),
            )
            .await?;
            if resp.result.is_some() {
                println!("{} Telegram alert channel added", "✓".green());
            } else if let Some(e) = resp.error {
                eprintln!("{} {}", "✗".red(), e.message);
            }
        }
        "alert.macos" => {
            let resp = ipc_call(
                "alert.add",
                serde_json::json!({
                    "channel": "macos",
                    "config": {},
                }),
            )
            .await?;
            if resp.result.is_some() {
                println!("{} macOS notification alert enabled", "✓".green());
            } else if let Some(e) = resp.error {
                eprintln!("{} {}", "✗".red(), e.message);
            }
        }
        "alert.list" => {
            let resp = ipc_call("alert.list", serde_json::json!({})).await?;
            if let Some(result) = resp.result {
                let channels: Vec<serde_json::Value> =
                    serde_json::from_value(result).unwrap_or_default();
                if channels.is_empty() {
                    println!("No alert channels configured.");
                } else {
                    println!(
                        "{:<5} {:<12} {:<8} {}",
                        "ID", "Channel", "Enabled", "Config"
                    );
                    println!("{}", "-".repeat(60));
                    for ch in channels {
                        println!(
                            "{:<5} {:<12} {:<8} {}",
                            ch.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                            ch.get("channel").and_then(|v| v.as_str()).unwrap_or(""),
                            if ch.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                "✅"
                            } else {
                                "❌"
                            },
                            ch.get("config").and_then(|v| v.as_str()).unwrap_or("{}"),
                        );
                    }
                }
            } else if let Some(e) = resp.error {
                eprintln!("{} {}", "✗".red(), e.message);
            }
        }
        "alert.remove" => {
            if args.len() < 2 {
                anyhow::bail!("Usage: agentbox config alert.remove <id>");
            }
            let id: i64 = args[1]
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid ID: {}", args[1]))?;
            let resp = ipc_call("alert.remove", serde_json::json!({"id": id})).await?;
            if resp.result.is_some() {
                println!("{} Alert channel {} removed", "✓".green(), id);
            } else if let Some(e) = resp.error {
                eprintln!("{} {}", "✗".red(), e.message);
            }
        }
        other => {
            anyhow::bail!("Unknown config subcommand: {}", other);
        }
    }

    Ok(())
}
