use agentbox_core::config;
use agentbox_daemon::daemon;
use colored::Colorize;
use std::process::Command;

pub async fn start(foreground: bool) -> anyhow::Result<()> {
    if daemon::is_daemon_running() {
        println!(
            "Daemon is already running (pid={})",
            daemon::read_pid_file().unwrap_or(0)
        );
        return Ok(());
    }

    if foreground {
        println!("{} Starting daemon in foreground...", "▶".green());
        daemon::run_daemon().await?;
    } else {
        // Fork a background process
        let exe = std::env::current_exe()?;
        let child = Command::new(exe)
            .args(["daemon", "start", "--foreground"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()?;

        println!("{} Daemon started (pid={})", "✓".green(), child.id());

        // Wait briefly for socket to appear
        for _ in 0..30 {
            if config::socket_path().exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    Ok(())
}

pub async fn stop() -> anyhow::Result<()> {
    if !daemon::is_daemon_running() {
        println!("Daemon is not running.");
        return Ok(());
    }

    // Try IPC shutdown first
    if config::socket_path().exists()
        && super::ipc_call("daemon.stop", serde_json::json!({}))
            .await
            .is_ok()
    {
        println!("{} Daemon stopping...", "✓".green());
        // Wait for process to exit
        for _ in 0..50 {
            if !daemon::is_daemon_running() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        daemon::remove_pid_file();
        daemon::cleanup_socket();
        return Ok(());
    }

    // Fallback: kill by PID
    if let Some(pid) = daemon::read_pid_file() {
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
        println!("{} Sent SIGTERM to daemon (pid={})", "✓".green(), pid);
        daemon::remove_pid_file();
        daemon::cleanup_socket();
    } else {
        println!("Cannot determine daemon PID.");
    }

    Ok(())
}

pub async fn status() -> anyhow::Result<()> {
    if daemon::is_daemon_running() {
        let pid = daemon::read_pid_file().unwrap_or(0);
        println!("{} Daemon is running (pid={})", "✓".green(), pid);

        // Try to get more info via IPC
        if let Ok(resp) = super::ipc_call("daemon.status", serde_json::json!({})).await {
            if let Some(result) = resp.result {
                if let Some(pid) = result.get("pid") {
                    println!("  PID: {}", pid);
                }
            }
        }
    } else {
        println!("{} Daemon is not running", "✗".red());
    }

    Ok(())
}

pub async fn install() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        const LAUNCHD_LABEL: &str = "com.agentbox.daemon";

        let exe = std::env::current_exe()?;
        let plist_dir = dirs::home_dir()
            .expect("Cannot determine home directory")
            .join("Library/LaunchAgents");
        std::fs::create_dir_all(&plist_dir)?;

        let log_path = config::daemon_log_path();
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>daemon</string>
        <string>start</string>
        <string>--foreground</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>SHELL</key>
        <string>{shell}</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
</dict>
</plist>"#,
            label = LAUNCHD_LABEL,
            exe = exe.display(),
            log = log_path.display(),
            shell = shell,
        );

        let path = plist_dir.join(format!("{}.plist", LAUNCHD_LABEL));
        std::fs::write(&path, plist_content)?;

        // Load the agent
        let output = Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&path)
            .output()?;

        if output.status.success() {
            println!("{} LaunchAgent installed and loaded", "✓".green());
            println!("  Plist: {}", path.display());
            println!("  AgentBox daemon will auto-start on login");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "{} Failed to load LaunchAgent: {}",
                "✗".red(),
                stderr.trim()
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        let exe = std::env::current_exe()?;
        let log_path = config::daemon_log_path();

        let unit_dir = dirs::home_dir()
            .expect("Cannot determine home directory")
            .join(".config/systemd/user");
        std::fs::create_dir_all(&unit_dir)?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let unit_content = format!(
            "[Unit]\n\
             Description=AgentBox Daemon\n\
             After=network.target\n\
             \n\
             [Service]\n\
             Type=exec\n\
             ExecStart={exe} daemon start --foreground\n\
             Environment=SHELL={shell}\n\
             Restart=on-failure\n\
             StandardOutput=append:{log}\n\
             StandardError=append:{log}\n\
             \n\
             [Install]\n\
             WantedBy=default.target\n",
            exe = exe.display(),
            log = log_path.display(),
            shell = shell,
        );

        let unit_path = unit_dir.join("agentbox.service");
        std::fs::write(&unit_path, unit_content)?;

        // Reload and enable
        let reload = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output()?;
        if !reload.status.success() {
            let stderr = String::from_utf8_lossy(&reload.stderr);
            eprintln!("{} Failed to reload systemd: {}", "✗".red(), stderr.trim());
            return Ok(());
        }

        let enable = Command::new("systemctl")
            .args(["--user", "enable", "--now", "agentbox"])
            .output()?;

        if enable.status.success() {
            println!("{} systemd user service installed and started", "✓".green());
            println!("  Unit: {}", unit_path.display());
            println!("  AgentBox daemon will auto-start on login");
        } else {
            let stderr = String::from_utf8_lossy(&enable.stderr);
            eprintln!("{} Failed to enable service: {}", "✗".red(), stderr.trim());
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("Auto-start is not supported on this platform");
    }

    Ok(())
}

pub async fn uninstall() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        const LAUNCHD_LABEL: &str = "com.agentbox.daemon";
        let path = dirs::home_dir()
            .expect("Cannot determine home directory")
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", LAUNCHD_LABEL));

        if !path.exists() {
            println!("LaunchAgent is not installed.");
            return Ok(());
        }

        let output = Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&path)
            .output()?;

        if output.status.success() || !daemon::is_daemon_running() {
            std::fs::remove_file(&path)?;
            println!("{} LaunchAgent uninstalled", "✓".green());
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "{} Failed to unload LaunchAgent: {}",
                "✗".red(),
                stderr.trim()
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        let unit_path = dirs::home_dir()
            .expect("Cannot determine home directory")
            .join(".config/systemd/user/agentbox.service");

        if !unit_path.exists() {
            println!("systemd service is not installed.");
            return Ok(());
        }

        let disable = Command::new("systemctl")
            .args(["--user", "disable", "--now", "agentbox"])
            .output()?;

        if disable.status.success() || !daemon::is_daemon_running() {
            std::fs::remove_file(&unit_path)?;
            let _ = Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .output();
            println!("{} systemd user service uninstalled", "✓".green());
        } else {
            let stderr = String::from_utf8_lossy(&disable.stderr);
            eprintln!("{} Failed to disable service: {}", "✗".red(), stderr.trim());
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("Auto-start is not supported on this platform");
    }

    Ok(())
}

extern crate libc;
