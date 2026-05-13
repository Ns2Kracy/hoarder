use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use axum::Router;
use futures::FutureExt;
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};
use tokio::net::TcpListener;

use crate::{
    AppConfig, AppError, AppResult,
    api::{
        routes::router_without_fallback,
        state::{ApiFuture, ApiRepository, ApiState, SyncService},
        types::{
            CreateSourceRequest, ItemDto, JobDto, JobRunResponse, RunDto, SettingsDto, SourceDto,
            SyncErrorDto,
        },
    },
    assets,
    connectors::{opendal::source::OpenDalSourceConnector, traits::SourceConnector},
    core::types::{ConnectorKind, ItemId, ItemType, JobId, RunId, SourceId, SyncStatus},
    db::{
        repository::{NewSource, SeaOrmRepository, SourceRepository},
        schema::sync_schema,
    },
    entity::{sync_error, sync_item, sync_job, sync_run},
    sync::{engine::SyncEngine, repository::SyncRepository, vault_writer::VaultWriter},
};

#[derive(Clone, Debug, Default)]
pub struct ServeOptions {
    pub config_path: Option<PathBuf>,
    pub addr: Option<SocketAddr>,
}

/// Starts the local Axum server.
///
/// # Errors
///
/// Returns an error when config loading, database setup, binding, or serving
/// fails.
pub async fn serve(options: ServeOptions) -> AppResult<()> {
    let config = load_config(options.config_path.as_deref()).await?;
    let config = apply_addr_override(config, options.addr);
    let addr = config.listen_addr;
    let db = connect_sqlite(&config).await?;
    sync_schema(&db).await?;
    let app = app_with_state(database_state(config, db));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(%addr, "hoarder API listening");
    println!("hoarder API listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Synchronizes the configured database schema.
///
/// # Errors
///
/// Returns an error when config loading, database connection, or schema sync
/// fails.
pub async fn sync_database(config_path: Option<PathBuf>) -> AppResult<()> {
    let config = load_config(config_path.as_deref()).await?;
    let db = connect_sqlite(&config).await?;

    sync_schema(&db).await
}

async fn load_config(path: Option<&Path>) -> AppResult<AppConfig> {
    let Some(path) = path else {
        return Ok(AppConfig::default());
    };

    let config = tokio::fs::read_to_string(path).await?;

    serde_json::from_str(&config).map_err(|error| {
        AppError::Config(format!(
            "failed to parse JSON config at {}: {error}",
            path.display()
        ))
    })
}

const fn apply_addr_override(mut config: AppConfig, addr: Option<SocketAddr>) -> AppConfig {
    if let Some(addr) = addr {
        config.listen_addr = addr;
    }

    config
}

async fn connect_sqlite(config: &AppConfig) -> AppResult<sea_orm::DatabaseConnection> {
    if let Some(parent) = config.database_path.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await?;
    }

    let database_url = sqlite_url(config);

    sea_orm::Database::connect(&database_url)
        .await
        .map_err(|error| AppError::Config(format!("database connection failed: {error}")))
}

fn sqlite_url(config: &AppConfig) -> String {
    let path = config.database_path.to_string_lossy();
    if path == ":memory:" {
        return "sqlite://:memory:".to_owned();
    }

    format!("sqlite://{path}?mode=rwc")
}

#[must_use]
pub fn config_with_addr(addr: Option<SocketAddr>) -> AppConfig {
    apply_addr_override(AppConfig::default(), addr)
}

#[must_use]
pub fn default_state(config: AppConfig) -> ApiState {
    ApiState::new(
        Arc::new(InMemoryApiRepository::new(config)),
        Arc::new(PlaceholderSyncService),
    )
}

pub fn app(config: AppConfig) -> Router {
    app_with_state(default_state(config))
}

fn app_with_state(state: ApiState) -> Router {
    Router::new()
        .merge(router_without_fallback(state))
        .fallback(assets::serve)
}

fn database_state(config: AppConfig, db: DatabaseConnection) -> ApiState {
    let repository = Arc::new(SeaOrmRepository::new(db));

    ApiState::new(
        Arc::new(DatabaseApiRepository::new(
            repository.clone(),
            config.clone(),
        )),
        Arc::new(EngineSyncService::new(repository, config.vault_path)),
    )
}

struct DatabaseApiRepository {
    repository: Arc<SeaOrmRepository>,
    config: AppConfig,
}

impl DatabaseApiRepository {
    const fn new(repository: Arc<SeaOrmRepository>, config: AppConfig) -> Self {
        Self { repository, config }
    }
}

impl ApiRepository for DatabaseApiRepository {
    fn list_sources(&self) -> ApiFuture<'_, Vec<SourceDto>> {
        async move {
            let records = self.repository.list_sources().await?;

            records
                .into_iter()
                .map(|record| {
                    let config = connector_config_from_json(record.id, record.config_json)?;
                    Ok(SourceDto::new(
                        record.id,
                        record.name,
                        &config,
                        record.enabled,
                    ))
                })
                .collect()
        }
        .boxed()
    }

    fn create_source(&self, request: CreateSourceRequest) -> ApiFuture<'_, SourceDto> {
        async move {
            let config_json = serde_json::to_value(&request.config).map_err(|error| {
                AppError::Database(format!("failed to serialize connector config: {error}"))
            })?;
            let record = self
                .repository
                .create_source(NewSource {
                    name: request.name,
                    kind: request.config.kind(),
                    config_json,
                    enabled: request.enabled,
                })
                .await?;

            Ok(SourceDto::new(
                record.id,
                record.name,
                &request.config,
                record.enabled,
            ))
        }
        .boxed()
    }

    fn list_jobs(&self) -> ApiFuture<'_, Vec<JobDto>> {
        async move {
            let jobs = sync_job::Entity::find()
                .order_by_asc(sync_job::Column::Name)
                .all(self.repository.connection())
                .await
                .map_err(map_db_error)?;

            Ok(jobs
                .into_iter()
                .map(|job| JobDto {
                    id: JobId::from_uuid(job.id),
                    source_id: SourceId::from_uuid(job.source_id),
                    name: job.name,
                    enabled: job.enabled,
                    schedule: None,
                })
                .collect())
        }
        .boxed()
    }

    fn list_runs(&self) -> ApiFuture<'_, Vec<RunDto>> {
        async move {
            let runs = sync_run::Entity::find()
                .order_by_desc(sync_run::Column::StartedAt)
                .all(self.repository.connection())
                .await
                .map_err(map_db_error)?;

            runs.into_iter()
                .map(|run| {
                    Ok(RunDto {
                        id: RunId::from_uuid(run.id),
                        job_id: JobId::from_uuid(run.job_id),
                        status: run_status_from_str(&run.status)?,
                        started_at: Some(run.started_at),
                        finished_at: run.finished_at,
                        processed_count: i64_to_u64(run.processed_count, "processed_count")?,
                        synced_count: i64_to_u64(run.synced_count, "synced_count")?,
                        skipped_count: i64_to_u64(run.skipped_count, "skipped_count")?,
                        failed_count: i64_to_u64(run.failed_count, "failed_count")?,
                    })
                })
                .collect()
        }
        .boxed()
    }

    fn list_items(&self) -> ApiFuture<'_, Vec<ItemDto>> {
        async move {
            let items = sync_item::Entity::find()
                .order_by_asc(sync_item::Column::SourcePath)
                .all(self.repository.connection())
                .await
                .map_err(map_db_error)?;

            items
                .into_iter()
                .map(|item| {
                    Ok(ItemDto {
                        id: ItemId::from_uuid(item.id),
                        source_id: SourceId::from_uuid(item.source_id),
                        source_path: item.source_path,
                        item_type: item_type_from_str(&item.item_type)?,
                        status: sync_status_from_str(&item.status)?,
                        size: item
                            .size
                            .map(|size| i64_to_u64(size, "sync_item.size"))
                            .transpose()?,
                        etag: item.etag,
                        modified_at: item.modified_at,
                        content_hash: item.content_hash,
                        metadata_json: item.metadata_json,
                    })
                })
                .collect()
        }
        .boxed()
    }

    fn list_errors(&self) -> ApiFuture<'_, Vec<SyncErrorDto>> {
        async move {
            let errors = sync_error::Entity::find()
                .order_by_desc(sync_error::Column::CreatedAt)
                .all(self.repository.connection())
                .await
                .map_err(map_db_error)?;

            Ok(errors
                .into_iter()
                .map(|error| SyncErrorDto {
                    id: error.id.to_string(),
                    run_id: error.run_id.map(RunId::from_uuid),
                    source_id: Some(SourceId::from_uuid(error.source_id)),
                    source_path: error.source_path,
                    code: error.error_kind,
                    message: error.message,
                    created_at: Some(error.created_at),
                })
                .collect())
        }
        .boxed()
    }

    fn settings(&self) -> ApiFuture<'_, SettingsDto> {
        async move { Ok(SettingsDto::from(&self.config)) }.boxed()
    }
}

struct EngineSyncService {
    repository: Arc<SeaOrmRepository>,
    vault_root: PathBuf,
}

impl EngineSyncService {
    const fn new(repository: Arc<SeaOrmRepository>, vault_root: PathBuf) -> Self {
        Self {
            repository,
            vault_root,
        }
    }
}

impl SyncService for EngineSyncService {
    fn run_job(&self, job_id: JobId) -> ApiFuture<'_, JobRunResponse> {
        async move {
            let job = self.repository.load_job(job_id).await?;
            let source_id = job.source_id;
            let engine = SyncEngine::new(
                self.repository.clone(),
                Arc::new(move |kind| match kind {
                    ConnectorKind::OpenDal => Ok(Arc::new(OpenDalSourceConnector::new(source_id))
                        as Arc<dyn SourceConnector>),
                    kind => Err(AppError::NotFound(format!(
                        "connector factory not registered for {kind:?}"
                    ))),
                }),
                VaultWriter::new(self.vault_root.clone()),
            );
            let summary = engine.run_job(job_id).await?;

            Ok(JobRunResponse {
                run_id: summary.run_id,
                status: if summary.failed == 0 {
                    SyncStatus::Synced
                } else {
                    SyncStatus::Failed
                },
            })
        }
        .boxed()
    }
}

fn connector_config_from_json(
    source_id: SourceId,
    config_json: serde_json::Value,
) -> AppResult<crate::connectors::traits::ConnectorConfig> {
    serde_json::from_value(config_json).map_err(|error| {
        AppError::Database(format!(
            "invalid connector config for source {source_id}: {error}"
        ))
    })
}

fn item_type_from_str(item_type: &str) -> AppResult<ItemType> {
    match item_type {
        "file" => Ok(ItemType::File),
        "directory" => Ok(ItemType::Directory),
        "virtual_document" => Ok(ItemType::VirtualDocument),
        other => Err(AppError::Database(format!(
            "unknown item type stored in database: {other}"
        ))),
    }
}

fn sync_status_from_str(status: &str) -> AppResult<SyncStatus> {
    match status {
        "pending" => Ok(SyncStatus::Pending),
        "synced" => Ok(SyncStatus::Synced),
        "failed" => Ok(SyncStatus::Failed),
        "skipped" => Ok(SyncStatus::Skipped),
        "deleted_on_source" => Ok(SyncStatus::DeletedOnSource),
        other => Err(AppError::Database(format!(
            "unknown sync status stored in database: {other}"
        ))),
    }
}

fn run_status_from_str(status: &str) -> AppResult<SyncStatus> {
    match status {
        "running" => Ok(SyncStatus::Pending),
        "completed" => Ok(SyncStatus::Synced),
        "completed_with_failures" | "failed" => Ok(SyncStatus::Failed),
        other => Err(AppError::Database(format!(
            "unknown run status stored in database: {other}"
        ))),
    }
}

fn i64_to_u64(value: i64, field: &str) -> AppResult<u64> {
    u64::try_from(value).map_err(|_| AppError::Database(format!("{field} is negative: {value}")))
}

#[allow(clippy::needless_pass_by_value)]
fn map_db_error(error: sea_orm::DbErr) -> AppError {
    AppError::Database(error.to_string())
}

struct InMemoryApiRepository {
    config: AppConfig,
    sources: Mutex<Vec<SourceDto>>,
}

impl InMemoryApiRepository {
    const fn new(config: AppConfig) -> Self {
        Self {
            config,
            sources: Mutex::new(Vec::new()),
        }
    }
}

impl ApiRepository for InMemoryApiRepository {
    fn list_sources(&self) -> ApiFuture<'_, Vec<SourceDto>> {
        async move { Ok(self.sources.lock().unwrap().clone()) }.boxed()
    }

    fn create_source(&self, request: CreateSourceRequest) -> ApiFuture<'_, SourceDto> {
        async move {
            let source = SourceDto::new(
                crate::core::types::SourceId::new(),
                request.name,
                &request.config,
                request.enabled,
            );
            self.sources.lock().unwrap().push(source.clone());

            Ok(source)
        }
        .boxed()
    }

    fn list_jobs(&self) -> ApiFuture<'_, Vec<JobDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_runs(&self) -> ApiFuture<'_, Vec<RunDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_items(&self) -> ApiFuture<'_, Vec<ItemDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_errors(&self) -> ApiFuture<'_, Vec<SyncErrorDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn settings(&self) -> ApiFuture<'_, SettingsDto> {
        async move { Ok(SettingsDto::from(&self.config)) }.boxed()
    }
}

#[derive(Default)]
struct PlaceholderSyncService;

impl SyncService for PlaceholderSyncService {
    fn run_job(&self, _job_id: JobId) -> ApiFuture<'_, JobRunResponse> {
        async move {
            Ok(JobRunResponse {
                run_id: RunId::new(),
                status: SyncStatus::Pending,
            })
        }
        .boxed()
    }
}
