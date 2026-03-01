use agentbox_core::upgrade;
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

    println!("{}", "Downloading...".dimmed());
    upgrade::download_and_replace(&info.download_url)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    println!(
        "\n{} Successfully upgraded to v{}!",
        "✓".green(),
        info.latest
    );

    Ok(())
}
