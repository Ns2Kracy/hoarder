use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;
use hoarder::{
    cli::{Cli, Command, DbCommand, JobCommand, SourceCommand, SyncCommand},
    server,
};

#[test]
fn cli_parse_serve_accepts_global_options_and_addr() {
    let cli = Cli::parse_from([
        "hoarder",
        "--config",
        "hoarder.toml",
        "--log-level",
        "debug",
        "serve",
        "--addr",
        "127.0.0.1:4761",
    ]);

    assert_eq!(cli.config, Some(PathBuf::from("hoarder.toml")));
    assert_eq!(cli.log_level, "debug");
    match cli.command {
        Command::Serve { addr } => {
            assert_eq!(addr, Some("127.0.0.1:4761".parse::<SocketAddr>().unwrap()));
        }
        other => panic!("expected serve command, got {other:?}"),
    }
}

#[test]
fn cli_parse_serve_defaults_to_no_addr_override() {
    let cli = Cli::parse_from(["hoarder", "serve"]);

    match cli.command {
        Command::Serve { addr } => assert_eq!(addr, None),
        other => panic!("expected serve command, got {other:?}"),
    }
}

#[test]
fn cli_parse_default_server_binding_remains_loopback() {
    let config = server::config_with_addr(None);

    assert_eq!(
        config.listen_addr,
        "127.0.0.1:4761".parse::<SocketAddr>().unwrap()
    );
}

#[test]
fn cli_parse_db_sync_command() {
    let cli = Cli::parse_from(["hoarder", "db", "sync"]);

    assert!(matches!(
        cli.command,
        Command::Db {
            command: DbCommand::Sync
        }
    ));
}

#[test]
fn cli_parse_source_commands() {
    let list = Cli::parse_from(["hoarder", "source", "list"]);
    assert!(matches!(
        list.command,
        Command::Source {
            command: SourceCommand::List
        }
    ));

    let add = Cli::parse_from([
        "hoarder",
        "source",
        "add",
        "--name",
        "Local Docs",
        "--service",
        "fs",
        "--root",
        ".",
    ]);
    match add.command {
        Command::Source {
            command: SourceCommand::Add(args),
        } => {
            assert_eq!(args.name, "Local Docs");
            assert_eq!(args.service, "fs");
            assert_eq!(args.root, Some(PathBuf::from(".")));
        }
        other => panic!("expected source add command, got {other:?}"),
    }

    let test = Cli::parse_from(["hoarder", "source", "test", "--id", "source-1"]);
    match test.command {
        Command::Source {
            command: SourceCommand::Test { id },
        } => assert_eq!(id, "source-1"),
        other => panic!("expected source test command, got {other:?}"),
    }
}

#[test]
fn cli_parse_job_commands() {
    let list = Cli::parse_from(["hoarder", "job", "list"]);
    assert!(matches!(
        list.command,
        Command::Job {
            command: JobCommand::List
        }
    ));

    let add = Cli::parse_from([
        "hoarder",
        "job",
        "add",
        "--source-id",
        "source-1",
        "--name",
        "Every five minutes",
        "--interval",
        "300",
    ]);
    match add.command {
        Command::Job {
            command:
                JobCommand::Add {
                    source_id,
                    name,
                    interval,
                    enabled,
                },
        } => {
            assert_eq!(source_id, "source-1");
            assert_eq!(name, "Every five minutes");
            assert_eq!(interval, Some(300));
            assert!(enabled);
        }
        other => panic!("expected job add command, got {other:?}"),
    }
}

#[test]
fn cli_parse_sync_commands() {
    let run = Cli::parse_from(["hoarder", "sync", "run", "--job-id", "job-1"]);
    match run.command {
        Command::Sync {
            command: SyncCommand::Run { job_id },
        } => assert_eq!(job_id, "job-1"),
        other => panic!("expected sync run command, got {other:?}"),
    }

    let status = Cli::parse_from(["hoarder", "sync", "status", "--job-id", "job-1"]);
    match status.command {
        Command::Sync {
            command: SyncCommand::Status { job_id, run_id },
        } => {
            assert_eq!(job_id, Some("job-1".to_owned()));
            assert_eq!(run_id, None);
        }
        other => panic!("expected sync status command, got {other:?}"),
    }
}
