use crate::ipc_client::IpcClient;
use rmcp::model::*;

fn make_resource(uri: &str, name: &str, description: &str, mime: &str) -> Resource {
    RawResource {
        uri: uri.to_string(),
        name: name.to_string(),
        title: None,
        description: Some(description.to_string()),
        mime_type: Some(mime.to_string()),
        size: None,
        icons: None,
        meta: None,
    }
    .no_annotation()
}

/// List available MCP resources.
pub async fn list_resources() -> Vec<Resource> {
    let mut resources = vec![make_resource(
        "agentbox://agents",
        "All Agents",
        "List of all registered agents with their current status",
        "application/json",
    )];

    // Dynamic per-agent resources
    if let Ok(val) = IpcClient::call_ok("agent.list", serde_json::json!({})).await {
        if let Some(agents) = val.as_array() {
            for agent in agents {
                if let Some(name) = agent.get("name").and_then(|n| n.as_str()) {
                    resources.push(make_resource(
                        &format!("agentbox://agents/{}", name),
                        &format!("Agent: {}", name),
                        &format!("Details for agent '{}' including config and last run", name),
                        "application/json",
                    ));
                    resources.push(make_resource(
                        &format!("agentbox://agents/{}/logs", name),
                        &format!("Logs: {}", name),
                        &format!("Recent logs for agent '{}'", name),
                        "text/plain",
                    ));
                }
            }
        }
    }

    resources
}

fn text_contents(uri: &str, mime: &str, text: String) -> ResourceContents {
    ResourceContents::TextResourceContents {
        uri: uri.to_string(),
        mime_type: Some(mime.to_string()),
        text,
        meta: None,
    }
}

/// Read a specific resource by URI.
pub async fn read_resource(uri: &str) -> Result<ReadResourceResult, rmcp::ErrorData> {
    let path = uri
        .strip_prefix("agentbox://")
        .ok_or_else(|| {
            rmcp::ErrorData::invalid_params("Invalid URI scheme, expected agentbox://", None)
        })?;

    let parts: Vec<&str> = path.split('/').collect();

    match parts.as_slice() {
        ["agents"] => {
            let val = IpcClient::call_ok("agent.list", serde_json::json!({}))
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e, None))?;

            Ok(ReadResourceResult {
                contents: vec![text_contents(
                    uri,
                    "application/json",
                    serde_json::to_string_pretty(&val).unwrap_or_default(),
                )],
            })
        }

        ["agents", name, "logs"] => {
            let ipc_params = serde_json::json!({
                "name": name,
                "tail": 100,
            });
            let val = IpcClient::call_ok("logs.tail", ipc_params)
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e, None))?;

            let text = if let Some(logs) = val.as_array() {
                logs.iter()
                    .filter_map(|entry| {
                        let ts = entry.get("created_at")?.as_str()?;
                        let level = entry.get("level")?.as_str()?;
                        let msg = entry.get("message")?.as_str()?;
                        Some(format!("[{}] [{}] {}", ts, level, msg))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                serde_json::to_string_pretty(&val).unwrap_or_default()
            };

            Ok(ReadResourceResult {
                contents: vec![text_contents(
                    uri,
                    "text/plain",
                    if text.is_empty() {
                        "No logs found.".to_string()
                    } else {
                        text
                    },
                )],
            })
        }

        ["agents", name] => {
            let agents = IpcClient::call_ok("agent.list", serde_json::json!({}))
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e, None))?;

            let agent = agents
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(name))
                })
                .cloned();

            match agent {
                Some(agent_data) => {
                    let history = IpcClient::call_ok(
                        "runs.history",
                        serde_json::json!({ "name": name, "limit": 5 }),
                    )
                    .await
                    .unwrap_or(serde_json::Value::Null);

                    let combined = serde_json::json!({
                        "agent": agent_data,
                        "recent_runs": history,
                    });

                    Ok(ReadResourceResult {
                        contents: vec![text_contents(
                            uri,
                            "application/json",
                            serde_json::to_string_pretty(&combined).unwrap_or_default(),
                        )],
                    })
                }
                None => Err(rmcp::ErrorData::invalid_params(
                    format!("Agent '{}' not found", name),
                    None,
                )),
            }
        }

        _ => Err(rmcp::ErrorData::invalid_params(
            format!("Unknown resource URI: {}", uri),
            None,
        )),
    }
}
