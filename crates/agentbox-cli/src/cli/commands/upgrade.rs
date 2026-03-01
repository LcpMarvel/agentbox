use agentbox_core::upgrade;
use agentbox_daemon::daemon;
use colored::Colorize;

pub async fn execute(check_only: bool) -> anyhow::Result<()> {
    println!("{}", "Checking for updates...".dimmed());

    let info = upgrade::check_latest_version()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    println!("  Current version: {}", format!("v{}", info.current).bold());
    println!("  Latest version:  {}", format!("v{}", info.latest).bold());

    if !info.has_update {
        println!("\n{} Already up to date.", "✓".green());
        return Ok(());
    }

    println!(
        "\n{} Update available: v{} → v{}",
        "→".cyan(),
        info.current,
        info.latest
    );

    if check_only {
        println!("\nRun {} to upgrade.", "agentbox upgrade".bold());
        return Ok(());
    }

    // Check if daemon is running before upgrade
    let daemon_was_running = daemon::is_daemon_running();

    // Stop daemon before replacing binary
    if daemon_was_running {
        println!("{}", "Stopping daemon...".dimmed());
        if let Err(e) = super::daemon::stop().await {
            eprintln!(
                "{} Failed to stop daemon: {} (continuing upgrade anyway)",
                "!".yellow(),
                e
            );
        }
    }

    println!("{}", "Downloading...".dimmed());
    upgrade::download_and_replace(&info.download_url)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    println!(
        "\n{} Successfully upgraded to v{}!",
        "✓".green(),
        info.latest
    );

    // Restart daemon if it was running
    if daemon_was_running {
        println!("{}", "Restarting daemon...".dimmed());
        if let Err(e) = super::daemon::start(false).await {
            eprintln!("{} Failed to restart daemon: {}", "!".yellow(), e);
            println!(
                "  You can start it manually with: {}",
                "agentbox daemon start".bold()
            );
        } else {
            println!("{} Daemon restarted with new version", "✓".green());
        }
    }

    Ok(())
}
