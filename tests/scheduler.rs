use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use hoarder::db::repository::RuntimeSettingsRepository;
use hoarder::{
    AppConfig,
    app::{job_service, scheduler},
    config::RuntimeSettingsPatch,
    connectors::traits::ConnectorConfig,
    core::types::{ConnectorKind, SourceId},
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

#[tokio::test]
async fn scheduler_uses_persisted_runtime_job_concurrency() -> Result<(), Box<dyn std::error::Error>>
{
    let test = TestScheduler::new().await?;
    test.repository
        .create_scheduled_job(NewScheduledSyncJob {
            source_id: test.source_id,
            name: "second interval sync".to_owned(),
            enabled: true,
            schedule: SyncJobSchedule::Interval {
                interval_seconds: 300,
            },
        })
        .await?;
    test.repository
        .patch_runtime_settings(
            &test.config,
            RuntimeSettingsPatch {
                job_concurrency: Some(2),
                file_concurrency: None,
                log_level: None,
            },
        )
        .await?;

    let started = scheduler::run_due_jobs_once(Arc::clone(&test.repository), &test.config).await?;

    assert_eq!(started, 2);

    Ok(())
}

#[tokio::test]
async fn scheduler_continues_after_one_due_job_fails() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestScheduler::new().await?;
    test.config.job_concurrency = 2;
    let failing_source = test
        .repository
        .create_source(NewSource {
            name: "Unsupported archive".to_owned(),
            kind: ConnectorKind::OpenDal,
            config_json: serde_json::to_value(ConnectorConfig::OpenDal {
                service: "s3".to_owned(),
                options: BTreeMap::from([
                    ("bucket".to_owned(), "archive".to_owned()),
                    ("region".to_owned(), "us-east-1".to_owned()),
                    ("access_key_id".to_owned(), "key".to_owned()),
                    ("secret_access_key".to_owned(), "secret".to_owned()),
                ]),
            })?,
            enabled: true,
        })
        .await?;
    test.repository
        .create_scheduled_job(NewScheduledSyncJob {
            source_id: failing_source.id,
            name: "bad interval sync".to_owned(),
            enabled: true,
            schedule: SyncJobSchedule::Interval {
                interval_seconds: 300,
            },
        })
        .await?;

    let started = scheduler::run_due_jobs_once(Arc::clone(&test.repository), &test.config).await?;

    assert_eq!(started, 1);
    let jobs = job_service::list_jobs(test.repository.as_ref()).await?;
    let good_job = jobs
        .iter()
        .find(|job| job.name == "interval sync")
        .expect("good interval job exists");
    assert!(good_job.last_run_at.is_some());

    Ok(())
}

struct TestScheduler {
    repository: Arc<SeaOrmRepository>,
    config: AppConfig,
    source_id: SourceId,
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
            source_id: source.id,
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
