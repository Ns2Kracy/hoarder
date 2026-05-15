use clap::Parser;
use hoarder::cli::{self, Cli};

#[tokio::main]
async fn main() -> hoarder::AppResult<()> {
    let cli = Cli::parse();
    hoarder::logging::init(&cli.log_level);

    cli::execute(cli).await
}
