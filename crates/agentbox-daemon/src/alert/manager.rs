use agentbox_db::connection::DbPool;
use agentbox_db::repo::AlertRepo;
use std::collections::HashMap;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub enum AlertType {
    Failure,
    Timeout,
    Recovery,
}

impl AlertType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Failure => "failure",
            Self::Timeout => "timeout",
            Self::Recovery => "recovery",
        }
    }
}

pub struct AlertManager {
    pool: DbPool,
}

impl AlertManager {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn send_alert(
        &self,
        agent_name: &str,
        agent_id: i64,
        run_id: Option<i64>,
        alert_type: AlertType,
        detail: &str,
    ) {
        let alert_repo = AlertRepo::new(self.pool.clone());
        let channels = match alert_repo.list_enabled() {
            Ok(ch) => ch,
            Err(e) => {
                error!("Failed to load alert channels: {}", e);
                return;
            }
        };

        if channels.is_empty() {
            return;
        }

        let emoji = match alert_type {
            AlertType::Failure => "❌",
            AlertType::Timeout => "⏰",
            AlertType::Recovery => "✅",
        };

        let message = format!(
            "{} [AgentBox] {} — Agent '{}': {}",
            emoji,
            alert_type.as_str().to_uppercase(),
            agent_name,
            detail
        );

        for channel in &channels {
            let config: HashMap<String, String> =
                serde_json::from_str(&channel.config).unwrap_or_default();

            let result = match channel.channel.as_str() {
                "webhook" => {
                    self.send_webhook(&config, &message, agent_name, &alert_type)
                        .await
                }
                "telegram" => self.send_telegram(&config, &message).await,
                "desktop" | "macos" => self.send_desktop_notification(&message, agent_name).await,
                other => {
                    warn!("Unknown alert channel: {}", other);
                    continue;
                }
            };

            match result {
                Ok(()) => {
                    info!("Alert sent via {}: {}", channel.channel, message);
                    let _ = alert_repo.record_alert(
                        agent_id,
                        run_id,
                        alert_type.as_str(),
                        &channel.channel,
                        &message,
                    );
                }
                Err(e) => {
                    error!("Failed to send alert via {}: {}", channel.channel, e);
                }
            }
        }
    }

    async fn send_webhook(
        &self,
        config: &HashMap<String, String>,
        message: &str,
        agent_name: &str,
        alert_type: &AlertType,
    ) -> Result<(), String> {
        let url = config.get("url").ok_or("webhook url not configured")?;

        let payload = serde_json::json!({
            "text": message,
            "agent": agent_name,
            "alert_type": alert_type.as_str(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        Ok(())
    }

    async fn send_telegram(
        &self,
        config: &HashMap<String, String>,
        message: &str,
    ) -> Result<(), String> {
        let token = config
            .get("bot_token")
            .ok_or("telegram bot_token not configured")?;
        let chat_id = config
            .get("chat_id")
            .ok_or("telegram chat_id not configured")?;

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "HTML",
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Telegram API returned HTTP {}", resp.status()));
        }
        Ok(())
    }

    /// When an agent succeeds and `notify_on_success` is enabled,
    /// send a desktop notification if no alert channels are configured.
    pub async fn notify_success_fallback(&self, agent_name: &str, notify_on_success: bool) {
        if !notify_on_success {
            return;
        }

        let alert_repo = AlertRepo::new(self.pool.clone());
        let has_channels = match alert_repo.list_enabled() {
            Ok(ch) => !ch.is_empty(),
            Err(_) => false,
        };

        if has_channels {
            return;
        }

        let message = format!("✅ Agent '{}' done", agent_name);
        if let Err(e) = self.send_desktop_notification(&message, agent_name).await {
            warn!("Desktop notification fallback failed: {}", e);
        }
    }

    async fn send_desktop_notification(
        &self,
        message: &str,
        agent_name: &str,
    ) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            let script = format!(
                r#"display notification "{}" with title "AgentBox" subtitle "{}""#,
                message.replace('"', r#"\""#),
                agent_name.replace('"', r#"\""#),
            );

            let output = tokio::process::Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .output()
                .await
                .map_err(|e| e.to_string())?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("osascript failed: {}", stderr));
            }
            Ok(())
        }

        #[cfg(target_os = "linux")]
        {
            let output = tokio::process::Command::new("notify-send")
                .arg(format!("AgentBox: {}", agent_name))
                .arg(message)
                .output()
                .await
                .map_err(|e| e.to_string())?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("notify-send failed: {}", stderr));
            }
            Ok(())
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Err("Desktop notifications not supported on this platform".into())
        }
    }
}
