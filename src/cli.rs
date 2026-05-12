use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "hoarder", about = "Local-first connector sync platform")]
pub struct Cli {
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[arg(long, global = true, default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Serve {
        #[arg(long)]
        addr: Option<SocketAddr>,
    },

    Db {
        #[command(subcommand)]
        command: DbCommand,
    },

    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },

    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    Sync,
}

#[derive(Debug, Subcommand)]
pub enum SourceCommand {
    List,

    Add {
        #[arg(long)]
        name: String,

        #[arg(long)]
        kind: String,

        #[arg(long)]
        config_json: String,
    },

    Test {
        #[arg(long)]
        id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SyncCommand {
    Run {
        #[arg(long)]
        job_id: String,
    },

    Status,
}
