use clap::Parser;
use hoarder_cli::cli::{self, Cli};
use hoarder_core::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();
    hoarder_server::logging::init(&cli.log_level);

    cli::execute(cli).await
}
