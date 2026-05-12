use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    AppError, AppResult,
    connectors::traits::ConnectorConfig,
    core::types::{ConnectorKind, ItemId, ItemType, JobId, RunId, SourceId, SyncStatus},
    entity::{source, sync_error, sync_item, sync_job, sync_run},
    sync::{
        engine::{SyncJob, SyncRunStatus, SyncRunSummary},
        planner::StoredItemState,
        repository::{ItemSyncOutcome, SyncRepository},
    },
};

pub type RepositoryFuture<'a, T> = BoxFuture<'a, AppResult<T>>;

#[derive(Clone)]
pub struct SeaOrmRepository {
    db: DatabaseConnection,
}

impl SeaOrmRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewSource {
    pub name: String,
    pub kind: ConnectorKind,
    pub config_json: Value,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceRecord {
    pub id: SourceId,
    pub name: String,
    pub kind: ConnectorKind,
    pub config_json: Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewSyncJob {
    pub source_id: SourceId,
    pub name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SyncJobRecord {
    pub id: JobId,
    pub source_id: SourceId,
    pub name: String,
    pub enabled: bool,
    pub status: String,
    pub cursor: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub trait SourceRepository: Send + Sync {
    fn create_source<'a>(&'a self, input: NewSource) -> RepositoryFuture<'a, SourceRecord>;

    fn list_sources(&self) -> RepositoryFuture<'_, Vec<SourceRecord>>;
}

pub trait SyncJobRepository: Send + Sync {
    fn create_job<'a>(&'a self, input: NewSyncJob) -> RepositoryFuture<'a, SyncJobRecord>;

    fn list_jobs(&self, source_id: SourceId) -> RepositoryFuture<'_, Vec<SyncJobRecord>>;
}

impl SourceRepository for SeaOrmRepository {
    fn create_source<'a>(&'a self, input: NewSource) -> RepositoryFuture<'a, SourceRecord> {
        Box::pin(async move {
            let now = Utc::now();
            let active_model = source::ActiveModel {
                id: Set(SourceId::new().as_uuid()),
                name: Set(input.name),
                kind: Set(connector_kind_to_str(input.kind).to_owned()),
                config_json: Set(input.config_json),
                enabled: Set(input.enabled),
                created_at: Set(now),
                updated_at: Set(now),
            };

            let model = active_model.insert(&self.db).await.map_err(map_db_error)?;
            source_record_from_model(model)
        })
    }

    fn list_sources(&self) -> RepositoryFuture<'_, Vec<SourceRecord>> {
        Box::pin(async move {
            let models = source::Entity::find()
                .all(&self.db)
                .await
                .map_err(map_db_error)?;

            models.into_iter().map(source_record_from_model).collect()
        })
    }
}

impl SyncJobRepository for SeaOrmRepository {
    fn create_job<'a>(&'a self, input: NewSyncJob) -> RepositoryFuture<'a, SyncJobRecord> {
        Box::pin(async move {
            let now = Utc::now();
            let active_model = sync_job::ActiveModel {
                id: Set(JobId::new().as_uuid()),
                source_id: Set(input.source_id.as_uuid()),
                name: Set(input.name),
                enabled: Set(input.enabled),
                status: Set("idle".to_owned()),
                cursor: Set(None),
                last_run_at: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };

            let model = active_model.insert(&self.db).await.map_err(map_db_error)?;
            Ok(sync_job_record_from_model(model))
        })
    }

    fn list_jobs(&self, source_id: SourceId) -> RepositoryFuture<'_, Vec<SyncJobRecord>> {
        Box::pin(async move {
            let models = sync_job::Entity::find()
                .filter(sync_job::Column::SourceId.eq(source_id.as_uuid()))
                .all(&self.db)
                .await
                .map_err(map_db_error)?;

            Ok(models.into_iter().map(sync_job_record_from_model).collect())
        })
    }
}

impl SyncRepository for SeaOrmRepository {
    fn load_job<'a>(&'a self, job_id: JobId) -> RepositoryFuture<'a, SyncJob> {
        Box::pin(async move {
            let job = sync_job::Entity::find_by_id(job_id.as_uuid())
                .one(&self.db)
                .await
                .map_err(map_db_error)?
                .ok_or_else(|| AppError::NotFound(format!("sync job not found: {job_id}")))?;
            let source = source::Entity::find_by_id(job.source_id)
                .one(&self.db)
                .await
                .map_err(map_db_error)?
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "source not found for sync job {}: {}",
                        job.id, job.source_id
                    ))
                })?;
            let connector_config = serde_json::from_value::<ConnectorConfig>(
                source.config_json.clone(),
            )
            .map_err(|error| {
                AppError::Database(format!(
                    "invalid connector config for source {}: {error}",
                    source.id
                ))
            })?;

            Ok(SyncJob {
                id: JobId::from_uuid(job.id),
                source_id: SourceId::from_uuid(source.id),
                connector_kind: connector_kind_from_str(&source.kind)?,
                connector_config,
                scan_cursor: job.cursor,
            })
        })
    }

    fn start_run<'a>(&'a self, job: &'a SyncJob) -> RepositoryFuture<'a, RunId> {
        Box::pin(async move {
            let now = Utc::now();
            let run_id = RunId::new();
            let active_model = sync_run::ActiveModel {
                id: Set(run_id.as_uuid()),
                job_id: Set(job.id.as_uuid()),
                source_id: Set(job.source_id.as_uuid()),
                status: Set("running".to_owned()),
                started_at: Set(now),
                finished_at: Set(None),
                processed_count: Set(0),
                synced_count: Set(0),
                skipped_count: Set(0),
                failed_count: Set(0),
                created_at: Set(now),
                updated_at: Set(now),
            };

            active_model.insert(&self.db).await.map_err(map_db_error)?;

            Ok(run_id)
        })
    }

    fn item_state<'a>(
        &'a self,
        source_id: SourceId,
        source_path: &'a str,
    ) -> RepositoryFuture<'a, Option<StoredItemState>> {
        Box::pin(async move {
            let item = sync_item::Entity::find()
                .filter(sync_item::Column::SourceId.eq(source_id.as_uuid()))
                .filter(sync_item::Column::SourcePath.eq(source_path))
                .filter(sync_item::Column::DeletedOnSourceAt.is_null())
                .one(&self.db)
                .await
                .map_err(map_db_error)?;

            item.map(stored_item_state_from_model).transpose()
        })
    }

    fn known_item_states<'a>(
        &'a self,
        source_id: SourceId,
    ) -> RepositoryFuture<'a, Vec<StoredItemState>> {
        Box::pin(async move {
            let items = sync_item::Entity::find()
                .filter(sync_item::Column::SourceId.eq(source_id.as_uuid()))
                .filter(sync_item::Column::DeletedOnSourceAt.is_null())
                .all(&self.db)
                .await
                .map_err(map_db_error)?;

            items
                .into_iter()
                .map(stored_item_state_from_model)
                .collect()
        })
    }

    fn record_item_outcome<'a>(
        &'a self,
        run_id: RunId,
        outcome: ItemSyncOutcome,
    ) -> RepositoryFuture<'a, ()> {
        Box::pin(async move {
            let item = upsert_sync_item(&self.db, run_id, &outcome).await?;

            if let Some(message) = outcome.error_message.as_ref() {
                insert_sync_error(&self.db, run_id, &outcome, item.id, message).await?;
            }

            Ok(())
        })
    }

    fn mark_deleted<'a>(
        &'a self,
        run_id: RunId,
        source_id: SourceId,
        source_path: &'a str,
    ) -> RepositoryFuture<'a, ()> {
        Box::pin(async move {
            let item = sync_item::Entity::find()
                .filter(sync_item::Column::SourceId.eq(source_id.as_uuid()))
                .filter(sync_item::Column::SourcePath.eq(source_path))
                .one(&self.db)
                .await
                .map_err(map_db_error)?
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "sync item not found for deleted source path: {source_path}"
                    ))
                })?;
            let now = Utc::now();
            let mut active_model: sync_item::ActiveModel = item.into();
            active_model.run_id = Set(Some(run_id.as_uuid()));
            active_model.status = Set(sync_status_to_str(SyncStatus::DeletedOnSource).to_owned());
            active_model.deleted_on_source_at = Set(Some(now));
            active_model.updated_at = Set(now);

            active_model.update(&self.db).await.map_err(map_db_error)?;

            Ok(())
        })
    }

    fn finish_run<'a>(
        &'a self,
        run_id: RunId,
        status: SyncRunStatus,
        summary: SyncRunSummary,
    ) -> RepositoryFuture<'a, ()> {
        Box::pin(async move {
            let run = sync_run::Entity::find_by_id(run_id.as_uuid())
                .one(&self.db)
                .await
                .map_err(map_db_error)?
                .ok_or_else(|| AppError::NotFound(format!("sync run not found: {run_id}")))?;
            let now = Utc::now();
            let job_id = run.job_id;
            let mut active_run: sync_run::ActiveModel = run.into();
            active_run.status = Set(sync_run_status_to_str(status).to_owned());
            active_run.finished_at = Set(Some(now));
            active_run.processed_count = Set(u64_to_i64(summary.processed, "processed_count")?);
            active_run.synced_count = Set(u64_to_i64(summary.synced, "synced_count")?);
            active_run.skipped_count = Set(u64_to_i64(summary.skipped, "skipped_count")?);
            active_run.failed_count = Set(u64_to_i64(summary.failed, "failed_count")?);
            active_run.updated_at = Set(now);
            active_run.update(&self.db).await.map_err(map_db_error)?;

            if let Some(job) = sync_job::Entity::find_by_id(job_id)
                .one(&self.db)
                .await
                .map_err(map_db_error)?
            {
                let mut active_job: sync_job::ActiveModel = job.into();
                active_job.status = Set(job_status_after_run(status).to_owned());
                active_job.last_run_at = Set(Some(now));
                active_job.updated_at = Set(now);
                active_job.update(&self.db).await.map_err(map_db_error)?;
            }

            Ok(())
        })
    }
}

async fn upsert_sync_item(
    db: &DatabaseConnection,
    run_id: RunId,
    outcome: &ItemSyncOutcome,
) -> AppResult<sync_item::Model> {
    let existing = sync_item::Entity::find()
        .filter(sync_item::Column::SourceId.eq(outcome.source_id.as_uuid()))
        .filter(sync_item::Column::SourcePath.eq(&outcome.source_path))
        .one(db)
        .await
        .map_err(map_db_error)?;
    let now = Utc::now();
    let target_path = outcome.target_path.as_ref().map(ToString::to_string);
    let size = optional_u64_to_i64(outcome.size, "sync_item.size")?;

    if let Some(model) = existing {
        let (local_path, content_hash) = if outcome.status == SyncStatus::Synced {
            (target_path, outcome.content_hash.clone())
        } else {
            (
                target_path.or(model.local_path.clone()),
                outcome.content_hash.clone().or(model.content_hash.clone()),
            )
        };
        let metadata_json = model.metadata_json.clone();
        let synced_at = if outcome.status == SyncStatus::Synced {
            Some(now)
        } else {
            model.synced_at
        };
        let mut active_model: sync_item::ActiveModel = model.into();
        active_model.run_id = Set(Some(run_id.as_uuid()));
        active_model.item_type = Set(item_type_to_str(outcome.item_type).to_owned());
        active_model.status = Set(sync_status_to_str(outcome.status).to_owned());
        active_model.size = Set(size);
        active_model.etag = Set(outcome.etag.clone());
        active_model.modified_at = Set(outcome.modified_at);
        active_model.content_hash = Set(content_hash);
        active_model.local_path = Set(local_path);
        active_model.metadata_json = Set(metadata_json);
        active_model.last_seen_at = Set(now);
        active_model.synced_at = Set(synced_at);
        active_model.deleted_on_source_at = Set(None);
        active_model.updated_at = Set(now);

        return active_model.update(db).await.map_err(map_db_error);
    }

    let active_model = sync_item::ActiveModel {
        id: Set(ItemId::new().as_uuid()),
        source_id: Set(outcome.source_id.as_uuid()),
        run_id: Set(Some(run_id.as_uuid())),
        source_path: Set(outcome.source_path.clone()),
        item_type: Set(item_type_to_str(outcome.item_type).to_owned()),
        status: Set(sync_status_to_str(outcome.status).to_owned()),
        size: Set(size),
        etag: Set(outcome.etag.clone()),
        modified_at: Set(outcome.modified_at),
        content_hash: Set(outcome.content_hash.clone()),
        local_path: Set(target_path),
        metadata_json: Set(None),
        last_seen_at: Set(now),
        synced_at: Set((outcome.status == SyncStatus::Synced).then_some(now)),
        deleted_on_source_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    active_model.insert(db).await.map_err(map_db_error)
}

async fn insert_sync_error(
    db: &DatabaseConnection,
    run_id: RunId,
    outcome: &ItemSyncOutcome,
    item_id: Uuid,
    message: &str,
) -> AppResult<()> {
    let run = sync_run::Entity::find_by_id(run_id.as_uuid())
        .one(db)
        .await
        .map_err(map_db_error)?;
    let active_model = sync_error::ActiveModel {
        id: Set(Uuid::now_v7()),
        source_id: Set(outcome.source_id.as_uuid()),
        job_id: Set(run.as_ref().map(|run| run.job_id)),
        run_id: Set(Some(run_id.as_uuid())),
        item_id: Set(Some(item_id)),
        source_path: Set(Some(outcome.source_path.clone())),
        error_kind: Set("item_sync".to_owned()),
        message: Set(message.to_owned()),
        created_at: Set(Utc::now()),
    };

    active_model.insert(db).await.map_err(map_db_error)?;

    Ok(())
}

fn stored_item_state_from_model(model: sync_item::Model) -> AppResult<StoredItemState> {
    Ok(StoredItemState {
        source_path: model.source_path,
        item_type: item_type_from_str(&model.item_type)?,
        size: optional_i64_to_u64(model.size, "sync_item.size")?,
        etag: model.etag,
        modified_at: model.modified_at,
        content_hash: model.content_hash,
    })
}

fn source_record_from_model(model: source::Model) -> AppResult<SourceRecord> {
    Ok(SourceRecord {
        id: SourceId::from_uuid(model.id),
        name: model.name,
        kind: connector_kind_from_str(&model.kind)?,
        config_json: model.config_json,
        enabled: model.enabled,
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

fn sync_job_record_from_model(model: sync_job::Model) -> SyncJobRecord {
    SyncJobRecord {
        id: JobId::from_uuid(model.id),
        source_id: SourceId::from_uuid(model.source_id),
        name: model.name,
        enabled: model.enabled,
        status: model.status,
        cursor: model.cursor,
        last_run_at: model.last_run_at,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

fn connector_kind_to_str(kind: ConnectorKind) -> &'static str {
    match kind {
        ConnectorKind::OpenDal => "opendal",
        ConnectorKind::Notion => "notion",
        ConnectorKind::Feishu => "feishu",
    }
}

fn connector_kind_from_str(kind: &str) -> AppResult<ConnectorKind> {
    match kind {
        "opendal" => Ok(ConnectorKind::OpenDal),
        "notion" => Ok(ConnectorKind::Notion),
        "feishu" => Ok(ConnectorKind::Feishu),
        other => Err(AppError::Database(format!(
            "unknown connector kind stored in database: {other}"
        ))),
    }
}

fn item_type_to_str(item_type: ItemType) -> &'static str {
    match item_type {
        ItemType::File => "file",
        ItemType::Directory => "directory",
        ItemType::VirtualDocument => "virtual_document",
    }
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

fn sync_status_to_str(status: SyncStatus) -> &'static str {
    match status {
        SyncStatus::Pending => "pending",
        SyncStatus::Synced => "synced",
        SyncStatus::Failed => "failed",
        SyncStatus::Skipped => "skipped",
        SyncStatus::DeletedOnSource => "deleted_on_source",
    }
}

fn sync_run_status_to_str(status: SyncRunStatus) -> &'static str {
    match status {
        SyncRunStatus::Completed => "completed",
        SyncRunStatus::CompletedWithFailures => "completed_with_failures",
        SyncRunStatus::Failed => "failed",
    }
}

fn job_status_after_run(status: SyncRunStatus) -> &'static str {
    match status {
        SyncRunStatus::Completed | SyncRunStatus::CompletedWithFailures => "idle",
        SyncRunStatus::Failed => "failed",
    }
}

fn optional_u64_to_i64(value: Option<u64>, field: &str) -> AppResult<Option<i64>> {
    value.map(|value| u64_to_i64(value, field)).transpose()
}

fn u64_to_i64(value: u64, field: &str) -> AppResult<i64> {
    i64::try_from(value)
        .map_err(|_| AppError::Database(format!("{field} value exceeds SQLite i64 range: {value}")))
}

fn optional_i64_to_u64(value: Option<i64>, field: &str) -> AppResult<Option<u64>> {
    value
        .map(|value| {
            u64::try_from(value)
                .map_err(|_| AppError::Database(format!("{field} value is negative: {value}")))
        })
        .transpose()
}

fn map_db_error(error: sea_orm::DbErr) -> AppError {
    AppError::Database(error.to_string())
}
