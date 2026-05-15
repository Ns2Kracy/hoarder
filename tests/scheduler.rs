use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use hoarder::{
    AppConfig,
    app::{job_service, scheduler},
    connectors::traits::ConnectorConfig,
    core::types::ConnectorKind,
    db::{
        connect_sqlite,
        repository::{
            NewScheduledSyncJob, NewSource, SeaOrmRepository, SourceRepository, SyncJobRepository,
            SyncJobSchedule,
        },
        schema::sync_schema,
    },
};
use uuid::Uuid;

#[tokio::test]
async fn scheduler_runs_due_interval_jobs_once() -> Result<(), Box<dyn std::error::Error>> {
    let test = TestScheduler::new().await?;

    let started = scheduler::run_due_jobs_once(Arc::clone(&test.repository), &test.config).await?;
    assert_eq!(started, 1);

    let jobs = job_service::list_jobs(test.repository.as_ref()).await?;
    let interval_job = jobs
        .iter()
        .find(|job| job.name == "interval sync")
        .expect("interval job exists");
    assert!(interval_job.last_run_at.is_some());
    assert!(interval_job.next_run_at.is_some());

    let started = scheduler::run_due_jobs_once(Arc::clone(&test.repository), &test.config).await?;
    assert_eq!(started, 0);

    Ok(())
}

struct TestScheduler {
    repository: Arc<SeaOrmRepository>,
    config: AppConfig,
    _temp: TempDir,
}

impl TestScheduler {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = TempDir::new("scheduler");
        let source_root = temp.path.join("source");
        let vault_root = temp.path.join("vault");
        fs::create_dir_all(&source_root)?;
        fs::write(source_root.join("readme.md"), "hello")?;

        let db = connect_sqlite("sqlite::memory:").await?;
        sync_schema(&db).await?;
        let repository = Arc::new(SeaOrmRepository::new(db));
        let source_config = fs_config(&source_root);
        let source = repository
            .create_source(NewSource {
                name: "Local Docs".to_owned(),
                kind: ConnectorKind::OpenDal,
                config_json: serde_json::to_value(&source_config)?,
                enabled: true,
            })
            .await?;
        repository
            .create_scheduled_job(NewScheduledSyncJob {
                source_id: source.id,
                name: "manual sync".to_owned(),
                enabled: true,
                schedule: SyncJobSchedule::Manual,
            })
            .await?;
        repository
            .create_scheduled_job(NewScheduledSyncJob {
                source_id: source.id,
                name: "interval sync".to_owned(),
                enabled: true,
                schedule: SyncJobSchedule::Interval {
                    interval_seconds: 300,
                },
            })
            .await?;

        Ok(Self {
            repository,
            config: AppConfig {
                database_path: PathBuf::from(":memory:"),
                vault_path: vault_root,
                job_concurrency: 1,
                ..AppConfig::default()
            },
            _temp: temp,
        })
    }
}

fn fs_config(root: &Path) -> ConnectorConfig {
    ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::from([("root".to_owned(), root.to_string_lossy().into_owned())]),
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("hoarder-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
