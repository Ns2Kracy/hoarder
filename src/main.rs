use clap::Parser;
use hoarder::{
    cli::{Cli, Command, DbCommand},
    server::{self, ServeOptions},
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> hoarder::AppResult<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level);

    match cli.command {
        Command::Serve { addr } => {
            server::serve(ServeOptions {
                config_path: cli.config,
                addr,
            })
            .await
        }
        Command::Db {
            command: DbCommand::Sync,
        } => server::sync_database(cli.config).await,
        Command::Source { .. } | Command::Sync { .. } => Ok(()),
    }
}

fn init_tracing(log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
