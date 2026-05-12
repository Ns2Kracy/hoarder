use clap::Parser;
use hoarder::{
    cli::{Cli, Command, DbCommand},
    server::{self, ServeOptions},
};

#[tokio::main]
async fn main() -> hoarder::AppResult<()> {
    let cli = Cli::parse();

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
