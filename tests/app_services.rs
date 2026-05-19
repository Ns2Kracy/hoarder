use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use hoarder::{
    AppConfig, AppError,
    api::types::{
        CreateJobRequest, CreateSourceRequest, ErrorListQuery, ItemListQuery, JobScheduleDto,
        UpdateSettingsRequest,
    },
    app::{job_service, run_service, settings_service, source_service},
    connectors::traits::ConnectorConfig,
    core::types::{JobStatus, SyncStatus},
    db::{connect_sqlite, repository::SeaOrmRepository, schema::sync_schema},
    entity::sync_job,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use uuid::Uuid;

#[tokio::test]
async fn app_services_run_local_fs_job_and_filter_results() -> Result<(), Box<dyn std::error::Error>>
{
    let test = TestServices::new("workflow").await?;
    fs::write(test.source_root.join("readme.md"), "hello")?;

    let source = source_service::create_source(
        test.repository.as_ref(),
        CreateSourceRequest {
            name: "Local Docs".to_owned(),
            config: fs_config(&test.source_root),
            enabled: true,
        },
    )
    .await?;
    let checked = source_service::test_source(test.repository.as_ref(), source.id).await?;
    assert!(checked.ok);

    let job = job_service::create_job(
        test.repository.as_ref(),
        CreateJobRequest {
            source_id: source.id,
            name: "Docs sync".to_owned(),
            enabled: true,
            schedule: JobScheduleDto::Interval {
                interval_seconds: 300,
            },
        },
    )
    .await?;
    let response = job_service::run_job(
        Arc::clone(&test.repository),
        test.config.vault_path.clone(),
        job.id,
        test.config.file_concurrency,
    )
    .await?;
    assert_eq!(response.status, SyncStatus::Synced);

    let jobs = job_service::list_jobs(test.repository.as_ref()).await?;
    let completed_job = jobs
        .iter()
        .find(|candidate| candidate.id == job.id)
        .expect("job is listed");
    assert_eq!(completed_job.status, JobStatus::Idle);
    assert!(completed_job.last_run_at.is_some());
    assert_eq!(completed_job.last_run_id, Some(response.run_id));

    let detail = run_service::get_run_detail(test.repository.as_ref(), response.run_id).await?;
    assert_eq!(detail.source_name, "Local Docs");
    assert_eq!(detail.job_name, "Docs sync");
    assert!(detail.counts.processed >= 1);
    assert!(detail.counts.synced >= 1);

    let items = run_service::list_items(
        test.repository.as_ref(),
        ItemListQuery {
            source_id: Some(source.id),
            status: Some(SyncStatus::Synced),
            run_id: Some(response.run_id),
        },
    )
    .await?;
    assert!(
        items.iter().any(|item| item.source_path == "readme.md"),
        "synced items should include readme.md: {items:?}"
    );

    let errors =
        run_service::list_errors(test.repository.as_ref(), ErrorListQuery::default()).await?;
    assert!(errors.is_empty());

    Ok(())
}

#[tokio::test]
async fn app_services_reject_disabled_and_running_jobs() -> Result<(), Box<dyn std::error::Error>> {
    let test = TestServices::new("job-rejection").await?;
    let source = source_service::create_source(
        test.repository.as_ref(),
        CreateSourceRequest {
            name: "Local Docs".to_owned(),
            config: fs_config(&test.source_root),
            enabled: true,
        },
    )
    .await?;
    let disabled = job_service::create_job(
        test.repository.as_ref(),
        CreateJobRequest {
            source_id: source.id,
            name: "Disabled sync".to_owned(),
            enabled: false,
            schedule: JobScheduleDto::Manual,
        },
    )
    .await?;

    let disabled_error = job_service::run_job(
        Arc::clone(&test.repository),
        test.config.vault_path.clone(),
        disabled.id,
        test.config.file_concurrency,
    )
    .await
    .expect_err("disabled jobs are rejected");
    assert!(matches!(disabled_error, AppError::Unprocessable(_)));

    let running = job_service::create_job(
        test.repository.as_ref(),
        CreateJobRequest {
            source_id: source.id,
            name: "Running sync".to_owned(),
            enabled: true,
            schedule: JobScheduleDto::Manual,
        },
    )
    .await?;
    set_job_running(test.repository.as_ref(), running.id).await?;
    let running_error = job_service::run_job(
        Arc::clone(&test.repository),
        test.config.vault_path.clone(),
        running.id,
        test.config.file_concurrency,
    )
    .await
    .expect_err("running jobs are rejected");
    assert!(matches!(running_error, AppError::Conflict(_)));

    Ok(())
}

#[tokio::test]
async fn app_services_patch_runtime_settings() -> Result<(), Box<dyn std::error::Error>> {
    let test = TestServices::new("settings").await?;

    let settings = settings_service::update_settings(
        test.repository.as_ref(),
        &test.config,
        UpdateSettingsRequest {
            job_concurrency: 2,
            file_concurrency: 8,
            log_level: "debug".to_owned(),
        },
    )
    .await?;

    assert_eq!(settings.job_concurrency, 2);
    assert_eq!(settings.file_concurrency, 8);
    assert_eq!(settings.log_level, "debug");
    assert!(settings.read_only.database_path);
    assert!(settings.read_only.vault_path);
    assert!(settings.read_only.listen_addr);

    Ok(())
}

struct TestServices {
    repository: Arc<SeaOrmRepository>,
    config: AppConfig,
    source_root: PathBuf,
    _temp: TempDir,
}

impl TestServices {
    async fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = TempDir::new(name);
        let source_root = temp.path.join("source");
        let vault_path = temp.path.join("vault");
        fs::create_dir_all(&source_root)?;

        let db = connect_sqlite("sqlite::memory:").await?;
        sync_schema(&db).await?;
        let repository = Arc::new(SeaOrmRepository::new(db));
        let config = AppConfig {
            database_path: PathBuf::from(":memory:"),
            vault_path,
            file_concurrency: 2,
            ..AppConfig::default()
        };

        Ok(Self {
            repository,
            config,
            source_root,
            _temp: temp,
        })
    }
}

async fn set_job_running(
    repository: &SeaOrmRepository,
    job_id: hoarder::core::types::JobId,
) -> Result<(), Box<dyn std::error::Error>> {
    let job = sync_job::Entity::find_by_id(job_id.as_uuid())
        .one(repository.connection())
        .await?
        .expect("job exists");
    let mut active: sync_job::ActiveModel = job.into();
    active.status = Set("running".to_owned());
    active.update(repository.connection()).await?;

    Ok(())
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
        let path = std::env::temp_dir().join(format!("hoarder-app-{name}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();

        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
