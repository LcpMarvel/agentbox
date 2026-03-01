mod cli;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("agentbox=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();
    cli::run(cli).await
}
