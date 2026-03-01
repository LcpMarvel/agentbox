use agentbox_core::config::DEFAULT_WEB_PORT;
use colored::Colorize;

pub async fn execute() -> anyhow::Result<()> {
    let url = format!("http://localhost:{}", DEFAULT_WEB_PORT);
    println!("{} Opening dashboard at {}", "▶".green(), url.bold());

    // Try to open in browser
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    }

    Ok(())
}
