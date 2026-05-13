use axum::{
    Json, Router,
    extract::{
        Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use sea_orm::{EntityTrait, QueryOrder};
use std::sync::Arc;

use crate::{
    AppError, AppResult,
    connectors::{opendal::source::OpenDalSourceConnector, traits::SourceConnector},
    core::types::{ConnectorKind, ItemId, ItemType, JobId, RunId, SourceId, SyncStatus},
    db::repository::{NewSource, SourceRepository},
    entity::{sync_error, sync_item, sync_job, sync_run},
    sync::{engine::SyncEngine, repository::SyncRepository, vault_writer::VaultWriter},
};

use super::{
    error::ApiError,
    state::ApiState,
    types::{
        CreateSourceRequest, HealthResponse, ItemDto, JobDto, JobRunResponse, ListResponse, RunDto,
        SettingsDto, SourceDto, SourceTestResponse, SyncErrorDto,
    },
};

pub fn router(state: ApiState) -> Router {
    api_routes_without_state()
        .fallback(api_not_found)
        .with_state(state)
}

pub fn router_without_fallback(state: ApiState) -> Router {
    api_routes_without_state().with_state(state)
}

fn api_routes_without_state() -> Router<ApiState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/sources", get(list_sources).post(create_source))
        .route("/api/sources/{id}/test", post(test_source))
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs/{id}/run", post(run_job))
        .route("/api/runs", get(list_runs))
        .route("/api/items", get(list_items))
        .route("/api/errors", get(list_errors))
        .route("/api/settings", get(settings))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

async fn list_sources(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<SourceDto>>, ApiError> {
    let records = state.repository().list_sources().await?;
    let sources = records
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
        .collect::<AppResult<Vec<_>>>()?;

    Ok(Json(ListResponse::new(sources)))
}

async fn create_source(
    State(state): State<ApiState>,
    payload: Result<Json<CreateSourceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SourceDto>), ApiError> {
    let Json(request) = payload?;
    let CreateSourceRequest {
        name,
        config,
        enabled,
    } = request;
    let config_json = serde_json::to_value(&config).map_err(|error| {
        AppError::Database(format!("failed to serialize connector config: {error}"))
    })?;
    let record = state
        .repository()
        .create_source(NewSource {
            name,
            kind: config.kind(),
            config_json,
            enabled,
        })
        .await?;
    let source = SourceDto::new(record.id, record.name, &config, record.enabled);

    Ok((StatusCode::CREATED, Json(source)))
}

async fn test_source(
    State(state): State<ApiState>,
    path: Result<Path<SourceId>, PathRejection>,
) -> Result<Json<SourceTestResponse>, ApiError> {
    let Path(source_id) = path?;
    let source = state.repository().load_source(source_id).await?;
    let config = connector_config_from_json(source.id, source.config_json)?;

    validate_source_connector(source.kind, source.id, &config).await?;

    Ok(Json(SourceTestResponse {
        ok: true,
        checked_at: Utc::now(),
    }))
}

async fn list_jobs(State(state): State<ApiState>) -> Result<Json<ListResponse<JobDto>>, ApiError> {
    let jobs = sync_job::Entity::find()
        .order_by_asc(sync_job::Column::Name)
        .all(state.repository().connection())
        .await
        .map_err(map_db_error)?;
    let jobs = jobs
        .into_iter()
        .map(|job| JobDto {
            id: JobId::from_uuid(job.id),
            source_id: SourceId::from_uuid(job.source_id),
            name: job.name,
            enabled: job.enabled,
            schedule: None,
        })
        .collect();

    Ok(Json(ListResponse::new(jobs)))
}

async fn run_job(
    State(state): State<ApiState>,
    path: Result<Path<JobId>, PathRejection>,
) -> Result<Json<JobRunResponse>, ApiError> {
    let Path(job_id) = path?;
    let job = state.repository().load_job(job_id).await?;
    let source_id = job.source_id;
    let engine = SyncEngine::new(
        Arc::clone(state.repository()),
        Arc::new(move |kind| source_connector(kind, source_id)),
        VaultWriter::new(state.vault_path()),
    );
    let summary = engine.run_job(job_id).await?;

    Ok(Json(JobRunResponse {
        run_id: summary.run_id,
        status: if summary.failed == 0 {
            SyncStatus::Synced
        } else {
            SyncStatus::Failed
        },
    }))
}

async fn list_runs(State(state): State<ApiState>) -> Result<Json<ListResponse<RunDto>>, ApiError> {
    let runs = sync_run::Entity::find()
        .order_by_desc(sync_run::Column::StartedAt)
        .all(state.repository().connection())
        .await
        .map_err(map_db_error)?;
    let runs = runs
        .into_iter()
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
        .collect::<AppResult<Vec<_>>>()?;

    Ok(Json(ListResponse::new(runs)))
}

async fn list_items(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<ItemDto>>, ApiError> {
    let items = sync_item::Entity::find()
        .order_by_asc(sync_item::Column::SourcePath)
        .all(state.repository().connection())
        .await
        .map_err(map_db_error)?;
    let items = items
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
        .collect::<AppResult<Vec<_>>>()?;

    Ok(Json(ListResponse::new(items)))
}

async fn list_errors(
    State(state): State<ApiState>,
) -> Result<Json<ListResponse<SyncErrorDto>>, ApiError> {
    let errors = sync_error::Entity::find()
        .order_by_desc(sync_error::Column::CreatedAt)
        .all(state.repository().connection())
        .await
        .map_err(map_db_error)?;
    let errors = errors
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
        .collect();

    Ok(Json(ListResponse::new(errors)))
}

async fn settings(State(state): State<ApiState>) -> Result<Json<SettingsDto>, ApiError> {
    Ok(Json(SettingsDto::from(state.config())))
}

async fn api_not_found() -> ApiError {
    ApiError::not_found("API route not found")
}

async fn validate_source_connector(
    kind: ConnectorKind,
    source_id: SourceId,
    config: &crate::connectors::traits::ConnectorConfig,
) -> AppResult<()> {
    match kind {
        ConnectorKind::OpenDal => {
            OpenDalSourceConnector::new(source_id)
                .validate(config)
                .await?;
            Ok(())
        }
        kind => Err(AppError::NotFound(format!(
            "connector factory not registered for {kind:?}"
        ))),
    }
}

fn source_connector(
    kind: ConnectorKind,
    source_id: SourceId,
) -> AppResult<Arc<dyn SourceConnector>> {
    match kind {
        ConnectorKind::OpenDal => {
            Ok(Arc::new(OpenDalSourceConnector::new(source_id)) as Arc<dyn SourceConnector>)
        }
        kind => Err(AppError::NotFound(format!(
            "connector factory not registered for {kind:?}"
        ))),
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
