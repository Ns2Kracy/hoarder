use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand};

use crate::{
    AppError, AppResult,
    api::types::{CreateJobRequest, CreateSourceRequest, JobScheduleDto},
    app::{job_service, run_service, source_service},
    connectors::traits::ConnectorConfig,
    core::types::{JobId, RunId, SourceId},
    server,
};

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
#[allow(clippy::large_enum_variant)]
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

    Job {
        #[command(subcommand)]
        command: JobCommand,
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
#[allow(clippy::large_enum_variant)]
pub enum SourceCommand {
    List,

    Add(SourceAddArgs),

    Test {
        #[arg(long)]
        id: String,
    },
}

#[derive(Debug, Args)]
pub struct SourceAddArgs {
    #[arg(long)]
    pub name: String,

    #[arg(long, default_value = "fs")]
    pub service: String,

    #[arg(long)]
    pub root: Option<PathBuf>,

    #[arg(long)]
    pub endpoint: Option<String>,

    #[arg(long)]
    pub bucket: Option<String>,

    #[arg(long)]
    pub region: Option<String>,

    #[arg(long)]
    pub username: Option<String>,

    #[arg(long)]
    pub access_key_id: Option<String>,

    #[arg(long)]
    pub secret_access_key: Option<String>,

    #[arg(long)]
    pub token: Option<String>,

    #[arg(long, hide = true)]
    pub kind: Option<String>,

    #[arg(long, hide = true)]
    pub config_json: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum JobCommand {
    List,

    Add {
        #[arg(long)]
        source_id: String,

        #[arg(long)]
        name: String,

        #[arg(long)]
        interval: Option<u64>,

        #[arg(long, default_value_t = true)]
        enabled: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SyncCommand {
    Run {
        #[arg(long)]
        job_id: String,
    },

    Status {
        #[arg(long)]
        job_id: Option<String>,

        #[arg(long)]
        run_id: Option<String>,
    },
}

/// Executes a parsed CLI command.
///
/// # Errors
///
/// Returns an error when command inputs are invalid or the underlying app
/// services fail.
pub async fn execute(cli: Cli) -> AppResult<()> {
    match cli.command {
        Command::Serve { addr } => {
            server::serve(server::ServeOptions {
                config_path: cli.config,
                addr,
            })
            .await
        }
        Command::Db {
            command: DbCommand::Sync,
        } => server::sync_database(cli.config).await,
        Command::Source { command } => execute_source(cli.config, command).await,
        Command::Job { command } => execute_job(cli.config, command).await,
        Command::Sync { command } => execute_sync(cli.config, command).await,
    }
}

async fn execute_source(config_path: Option<PathBuf>, command: SourceCommand) -> AppResult<()> {
    let (_, repository) = server::open_repository(config_path).await?;
    match command {
        SourceCommand::List => {
            let sources = source_service::list_sources(repository.as_ref()).await?;
            println!("ID\tNAME\tKIND\tENABLED\tHEALTH");
            for source in sources {
                println!(
                    "{}\t{}\t{:?}\t{}\t{:?}",
                    source.id, source.name, source.connector_kind, source.enabled, source.health
                );
            }
        }
        SourceCommand::Add(args) => {
            let config = source_config_from_cli(&args)?;
            let source = source_service::create_source(
                repository.as_ref(),
                CreateSourceRequest {
                    name: args.name,
                    config,
                    enabled: true,
                },
            )
            .await?;
            println!("{}", source.id);
        }
        SourceCommand::Test { id } => {
            let source_id = parse_id::<SourceId>(&id, "source id")?;
            let result = source_service::test_source(repository.as_ref(), source_id).await?;
            println!("ok={} checkedAt={}", result.ok, result.checked_at);
        }
    }

    Ok(())
}

async fn execute_job(config_path: Option<PathBuf>, command: JobCommand) -> AppResult<()> {
    let (_, repository) = server::open_repository(config_path).await?;
    match command {
        JobCommand::List => {
            let jobs = job_service::list_jobs(repository.as_ref()).await?;
            println!("ID\tSOURCE\tNAME\tENABLED\tSTATUS\tSCHEDULE");
            for job in jobs {
                println!(
                    "{}\t{}\t{}\t{}\t{:?}\t{}",
                    job.id,
                    job.source_id,
                    job.name,
                    job.enabled,
                    job.status,
                    schedule_label(&job.schedule)
                );
            }
        }
        JobCommand::Add {
            source_id,
            name,
            interval,
            enabled,
        } => {
            let source_id = parse_id::<SourceId>(&source_id, "source id")?;
            let schedule = interval.map_or(JobScheduleDto::Manual, |interval_seconds| {
                JobScheduleDto::Interval { interval_seconds }
            });
            let job = job_service::create_job(
                repository.as_ref(),
                CreateJobRequest {
                    source_id,
                    name,
                    enabled,
                    schedule,
                },
            )
            .await?;
            println!("{}", job.id);
        }
    }

    Ok(())
}

async fn execute_sync(config_path: Option<PathBuf>, command: SyncCommand) -> AppResult<()> {
    let (config, repository) = server::open_repository(config_path).await?;
    match command {
        SyncCommand::Run { job_id } => {
            let job_id = parse_id::<JobId>(&job_id, "job id")?;
            let response =
                job_service::run_job(repository, config.vault_path.clone(), job_id).await?;
            println!("runId={} status={:?}", response.run_id, response.status);
        }
        SyncCommand::Status { job_id, run_id } => {
            if let Some(run_id) = run_id {
                let run_id = parse_id::<RunId>(&run_id, "run id")?;
                let detail = run_service::get_run_detail(repository.as_ref(), run_id).await?;
                println!(
                    "{}\t{:?}\tprocessed={}\tsynced={}\tskipped={}\tfailed={}",
                    detail.id,
                    detail.status,
                    detail.counts.processed,
                    detail.counts.synced,
                    detail.counts.skipped,
                    detail.counts.failed
                );
            } else {
                let filter_job_id = job_id
                    .as_deref()
                    .map(|id| parse_id::<JobId>(id, "job id"))
                    .transpose()?;
                for run in run_service::list_runs(repository.as_ref()).await? {
                    if filter_job_id.is_none_or(|job_id| job_id == run.job_id) {
                        println!(
                            "{}\t{}\t{:?}\tprocessed={}",
                            run.id, run.job_id, run.status, run.processed_count
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn source_config_from_cli(args: &SourceAddArgs) -> AppResult<ConnectorConfig> {
    if let Some(config_json) = args.config_json.as_ref() {
        if args.kind.as_deref().is_some_and(|kind| kind != "opendal") {
            return Err(AppError::Validation(
                "only opendal source config is supported".to_owned(),
            ));
        }
        return serde_json::from_str(config_json).map_err(|error| {
            AppError::Validation(format!("invalid connector config JSON: {error}"))
        });
    }

    let mut options = BTreeMap::new();
    if let Some(root) = args.root.as_ref() {
        options.insert("root".to_owned(), root.to_string_lossy().into_owned());
    }
    insert_option(&mut options, "endpoint", args.endpoint.as_deref());
    insert_option(&mut options, "bucket", args.bucket.as_deref());
    insert_option(&mut options, "region", args.region.as_deref());
    insert_option(&mut options, "username", args.username.as_deref());
    insert_option(&mut options, "access_key_id", args.access_key_id.as_deref());
    insert_option(
        &mut options,
        "secret_access_key",
        args.secret_access_key.as_deref(),
    );
    insert_option(&mut options, "token", args.token.as_deref());

    Ok(ConnectorConfig::OpenDal {
        service: args.service.clone(),
        options,
    })
}

fn insert_option(options: &mut BTreeMap<String, String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        options.insert(key.to_owned(), value.to_owned());
    }
}

fn parse_id<T>(value: &str, label: &str) -> AppResult<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|error| AppError::Validation(format!("invalid {label}: {error}")))
}

fn schedule_label(schedule: &JobScheduleDto) -> String {
    match schedule {
        JobScheduleDto::Manual => "manual".to_owned(),
        JobScheduleDto::Interval { interval_seconds } => {
            format!("every {interval_seconds}s")
        }
    }
}
