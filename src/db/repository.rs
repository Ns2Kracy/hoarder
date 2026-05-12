use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::Value;

use crate::{
    AppError, AppResult,
    core::types::{ConnectorKind, JobId, SourceId},
    entity::{source, sync_job},
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

fn map_db_error(error: sea_orm::DbErr) -> AppError {
    AppError::Database(error.to_string())
}
