pub mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "agentbox",
    version,
    about = "PM2 for AI Agents — manage your local AI agents"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Register a new agent
    Register {
        /// Agent name (unique identifier)
        name: String,
        /// Command to execute
        command: String,
        /// Working directory
        #[arg(short = 'd', long)]
        dir: Option<String>,
        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<i64>,
        /// Max retries on failure
        #[arg(long, default_value = "0")]
        retry: i64,
        /// Retry delay in seconds
        #[arg(long, default_value = "30")]
        retry_delay: i64,
        /// Retry strategy: "fixed" or "exponential"
        #[arg(long, default_value = "fixed")]
        retry_strategy: String,
        /// Send desktop notification on successful completion (default: true)
        #[arg(long)]
        notify_on_success: Option<bool>,
    },

    /// List all agents
    #[command(alias = "ls")]
    List,

    /// Run an agent immediately
    Run {
        /// Agent name
        name: String,
    },

    /// Set schedule for an agent
    Schedule {
        /// Agent name
        name: String,
        /// Cron expression (e.g. "0 18 * * *")
        cron: Option<String>,
        /// Run every N interval (e.g. "30m", "2h", "1d")
        #[arg(long)]
        every: Option<String>,
        /// Run after another agent completes
        #[arg(long)]
        after: Option<String>,
        /// Reset to manual (no schedule)
        #[arg(long)]
        manual: bool,
    },

    /// Pause an agent's schedule
    Pause {
        /// Agent name
        name: String,
    },

    /// Resume an agent's schedule
    Resume {
        /// Agent name
        name: String,
    },

    /// View agent logs
    Logs {
        /// Agent name (omit for all agents)
        name: Option<String>,
        /// Show all agents' logs
        #[arg(long)]
        all: bool,
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "50")]
        tail: i64,
    },

    /// View run history for an agent
    History {
        /// Agent name
        name: String,
        /// Number of runs to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: i64,
    },

    /// Edit an agent's command or configuration
    Edit {
        /// Agent name
        name: String,
        /// New command to execute
        #[arg(short = 'c', long)]
        command: Option<String>,
        /// New working directory
        #[arg(short = 'd', long)]
        dir: Option<String>,
        /// Timeout in seconds (0 to remove)
        #[arg(long)]
        timeout: Option<i64>,
        /// Max retries on failure
        #[arg(long)]
        retry: Option<i64>,
        /// Retry delay in seconds
        #[arg(long)]
        retry_delay: Option<i64>,
        /// Retry strategy: "fixed" or "exponential"
        #[arg(long)]
        retry_strategy: Option<String>,
        /// Send desktop notification on successful completion
        #[arg(long)]
        notify_on_success: Option<bool>,
    },

    /// Remove an agent
    #[command(alias = "rm")]
    Remove {
        /// Agent name
        name: String,
    },

    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Open the web dashboard
    Dashboard,

    /// Manage configuration and alerts
    Config {
        /// Config arguments (e.g. "set key value", "alert.webhook <url>")
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Start MCP server for AI client integration (stdio transport)
    Mcp,
}

#[derive(Subcommand)]
pub enum DaemonAction {
    /// Start the daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status,
    /// Install as system service (auto-start on login)
    Install,
    /// Uninstall system service
    Uninstall,
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Register {
            name,
            command,
            dir,
            timeout,
            retry,
            retry_delay,
            retry_strategy,
            notify_on_success,
        } => {
            commands::register::execute(
                &name,
                &command,
                dir.as_deref(),
                timeout,
                retry,
                retry_delay,
                &retry_strategy,
                notify_on_success,
            )
            .await
        }
        Commands::List => commands::list::execute().await,
        Commands::Run { name } => commands::run::execute(&name).await,
        Commands::Schedule {
            name,
            cron,
            every,
            after,
            manual,
        } => commands::schedule::execute(&name, cron, every, after, manual).await,
        Commands::Pause { name } => commands::pause::execute(&name).await,
        Commands::Resume { name } => commands::resume::execute(&name).await,
        Commands::Logs { name, all, tail } => {
            commands::logs::execute(name.as_deref(), all, tail).await
        }
        Commands::History { name, limit } => commands::history::execute(&name, limit).await,
        Commands::Edit {
            name,
            command,
            dir,
            timeout,
            retry,
            retry_delay,
            retry_strategy,
            notify_on_success,
        } => {
            commands::edit::execute(
                &name,
                command,
                dir,
                timeout,
                retry,
                retry_delay,
                retry_strategy,
                notify_on_success,
            )
            .await
        }
        Commands::Remove { name } => commands::remove::execute(&name).await,
        Commands::Daemon { action } => match action {
            DaemonAction::Start { foreground } => commands::daemon::start(foreground).await,
            DaemonAction::Stop => commands::daemon::stop().await,
            DaemonAction::Status => commands::daemon::status().await,
            DaemonAction::Install => commands::daemon::install().await,
            DaemonAction::Uninstall => commands::daemon::uninstall().await,
        },
        Commands::Dashboard => commands::dashboard::execute().await,
        Commands::Config { args } => commands::config_cmd::execute(args).await,
        Commands::Mcp => agentbox_mcp::run_server().await,
    }
}
