use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;
use hoarder::{
    cli::{Cli, Command, DbCommand, SourceCommand, SyncCommand},
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
        "--kind",
        "opendal",
        "--config-json",
        r#"{"kind":"opendal","service":"fs","options":{"root":"."}}"#,
    ]);
    match add.command {
        Command::Source {
            command:
                SourceCommand::Add {
                    name,
                    kind,
                    config_json,
                },
        } => {
            assert_eq!(name, "Local Docs");
            assert_eq!(kind, "opendal");
            assert!(config_json.contains(r#""service":"fs""#));
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
fn cli_parse_sync_commands() {
    let run = Cli::parse_from(["hoarder", "sync", "run", "--job-id", "job-1"]);
    match run.command {
        Command::Sync {
            command: SyncCommand::Run { job_id },
        } => assert_eq!(job_id, "job-1"),
        other => panic!("expected sync run command, got {other:?}"),
    }

    let status = Cli::parse_from(["hoarder", "sync", "status"]);
    assert!(matches!(
        status.command,
        Command::Sync {
            command: SyncCommand::Status
        }
    ));
}
