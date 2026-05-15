use clap::Parser;
use hoarder::cli::{self, Cli};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> hoarder::AppResult<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level);

    cli::execute(cli).await
}

fn init_tracing(log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
