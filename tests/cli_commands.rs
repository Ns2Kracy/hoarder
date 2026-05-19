use std::{fs, path::PathBuf};

use hoarder::{
    AppConfig,
    api::types::SourceHealth,
    cli::{
        Cli, Command, DbCommand, JobCommand, SourceAddArgs, SourceCommand, SyncCommand, execute,
    },
    core::types::SyncStatus,
    server,
    sync::repository::SyncRepository,
};
use uuid::Uuid;

#[tokio::test]
async fn cli_commands_execute_local_source_job_and_sync_workflow()
-> Result<(), Box<dyn std::error::Error>> {
    let test = TestCli::new("workflow")?;
    fs::write(test.source_root.join("readme.md"), "hello")?;

    execute(test.cli(Command::Db {
        command: DbCommand::Sync,
    }))
    .await?;
    execute(test.cli(Command::Source {
        command: SourceCommand::Add(SourceAddArgs {
            name: "Local Docs".to_owned(),
            service: "fs".to_owned(),
            root: Some(test.source_root.clone()),
            endpoint: None,
            bucket: None,
            region: None,
            username: None,
            access_key_id: None,
            secret_access_key: None,
            token: None,
            kind: None,
            config_json: None,
        }),
    }))
    .await?;

    let (config, repository) = server::open_repository(Some(test.config_path.clone())).await?;
    let sources = hoarder::app::source_service::list_sources(repository.as_ref()).await?;
    let source_id = sources.first().expect("source was created").id;
    assert_eq!(sources[0].name, "Local Docs");

    execute(test.cli(Command::Source {
        command: SourceCommand::List,
    }))
    .await?;
    execute(test.cli(Command::Source {
        command: SourceCommand::Test {
            id: source_id.to_string(),
        },
    }))
    .await?;
    let sources = hoarder::app::source_service::list_sources(repository.as_ref()).await?;
    assert_eq!(sources[0].health, SourceHealth::Healthy);

    execute(test.cli(Command::Job {
        command: JobCommand::Add {
            source_id: source_id.to_string(),
            name: "Docs sync".to_owned(),
            interval: Some(300),
            enabled: true,
        },
    }))
    .await?;
    execute(test.cli(Command::Job {
        command: JobCommand::List,
    }))
    .await?;

    let jobs = hoarder::app::job_service::list_jobs(repository.as_ref()).await?;
    let job = jobs.first().expect("job was created");
    assert_eq!(job.name, "Docs sync");

    execute(test.cli(Command::Sync {
        command: SyncCommand::Run {
            job_id: job.id.to_string(),
        },
    }))
    .await?;

    let runs = hoarder::app::run_service::list_runs(repository.as_ref()).await?;
    let run = runs.first().expect("sync run was created");
    assert_eq!(run.status, SyncStatus::Synced);
    assert!(run.processed_count >= 1);

    execute(test.cli(Command::Sync {
        command: SyncCommand::Status {
            job_id: Some(job.id.to_string()),
            run_id: None,
        },
    }))
    .await?;
    execute(test.cli(Command::Sync {
        command: SyncCommand::Status {
            job_id: None,
            run_id: Some(run.id.to_string()),
        },
    }))
    .await?;

    let reloaded_job = repository.load_job(job.id).await?;
    assert_eq!(reloaded_job.source_id, source_id);
    assert!(
        config
            .vault_path
            .join(source_id.to_string())
            .join("readme.md")
            .exists()
    );

    Ok(())
}

struct TestCli {
    config_path: PathBuf,
    source_root: PathBuf,
    _temp: TempDir,
}

impl TestCli {
    fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = TempDir::new(name);
        let source_root = temp.path.join("source");
        fs::create_dir_all(&source_root)?;
        let config_path = temp.path.join("hoarder.json");
        let config = AppConfig {
            database_path: temp.path.join("hoarder.sqlite"),
            vault_path: temp.path.join("vault"),
            ..AppConfig::default()
        };
        fs::write(&config_path, serde_json::to_vec_pretty(&config)?)?;

        Ok(Self {
            config_path,
            source_root,
            _temp: temp,
        })
    }

    fn cli(&self, command: Command) -> Cli {
        Cli {
            config: Some(self.config_path.clone()),
            log_level: "info".to_owned(),
            command,
        }
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("hoarder-cli-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
