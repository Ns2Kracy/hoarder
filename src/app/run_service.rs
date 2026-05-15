use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    AppError, AppResult,
    api::types::{
        ErrorListQuery, ItemDto, ItemListQuery, RunCountsDto, RunDetailDto, RunDto, SyncErrorDto,
    },
    core::types::{ItemId, ItemType, RunId, RunStatus, SyncStatus},
    db::repository::SeaOrmRepository,
    entity::{source, sync_error, sync_item, sync_job, sync_run},
};

/// Lists sync run summaries.
///
/// # Errors
///
/// Returns an error when database reads fail or stored run metadata is invalid.
pub async fn list_runs(repository: &SeaOrmRepository) -> AppResult<Vec<RunDto>> {
    sync_run::Entity::find()
        .order_by_desc(sync_run::Column::StartedAt)
        .all(repository.connection())
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(|run| {
            Ok(RunDto {
                id: RunId::from_uuid(run.id),
                job_id: crate::core::types::JobId::from_uuid(run.job_id),
                status: run_summary_status_from_str(&run.status)?,
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

/// Loads one sync run with source, job, counts, and recent errors.
///
/// # Errors
///
/// Returns an error when the run, source, or job cannot be found, or when stored
/// run metadata is invalid.
pub async fn get_run_detail(
    repository: &SeaOrmRepository,
    run_id: RunId,
) -> AppResult<RunDetailDto> {
    let db = repository.connection();
    let run = sync_run::Entity::find_by_id(run_id.as_uuid())
        .one(db)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("sync run not found: {run_id}")))?;
    let job = sync_job::Entity::find_by_id(run.job_id)
        .one(db)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("sync job not found: {}", run.job_id)))?;
    let source = source::Entity::find_by_id(run.source_id)
        .one(db)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("source not found: {}", run.source_id)))?;
    let errors = list_errors(
        repository,
        ErrorListQuery {
            run_id: Some(run_id),
            source_id: None,
        },
    )
    .await?;
    let deleted = deleted_count_for_run(repository, run_id).await?;

    Ok(RunDetailDto {
        id: run_id,
        job_id: crate::core::types::JobId::from_uuid(run.job_id),
        source_id: crate::core::types::SourceId::from_uuid(run.source_id),
        source_name: source.name,
        job_name: job.name,
        status: run_status_from_str(&run.status)?,
        started_at: Some(run.started_at),
        finished_at: run.finished_at,
        duration_ms: run.finished_at.map(|finished_at| {
            u64::try_from((finished_at - run.started_at).num_milliseconds()).unwrap_or(0)
        }),
        counts: RunCountsDto {
            processed: i64_to_u64(run.processed_count, "processed_count")?,
            synced: i64_to_u64(run.synced_count, "synced_count")?,
            skipped: i64_to_u64(run.skipped_count, "skipped_count")?,
            failed: i64_to_u64(run.failed_count, "failed_count")?,
            deleted,
        },
        errors,
    })
}

/// Lists sync items with optional source, status, and run filters.
///
/// # Errors
///
/// Returns an error when database reads fail or stored item metadata is invalid.
pub async fn list_items(
    repository: &SeaOrmRepository,
    query: ItemListQuery,
) -> AppResult<Vec<ItemDto>> {
    let mut select = sync_item::Entity::find();
    if let Some(source_id) = query.source_id {
        select = select.filter(sync_item::Column::SourceId.eq(source_id.as_uuid()));
    }
    if let Some(run_id) = query.run_id {
        select = select.filter(sync_item::Column::RunId.eq(run_id.as_uuid()));
    }
    if let Some(status) = query.status {
        select = select.filter(sync_item::Column::Status.eq(sync_status_to_str(status)));
    }

    select
        .order_by_asc(sync_item::Column::SourcePath)
        .all(repository.connection())
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(item_dto_from_model)
        .collect()
}

/// Lists sync errors with optional source and run filters.
///
/// # Errors
///
/// Returns an error when database reads fail.
pub async fn list_errors(
    repository: &SeaOrmRepository,
    query: ErrorListQuery,
) -> AppResult<Vec<SyncErrorDto>> {
    let mut select = sync_error::Entity::find();
    if let Some(source_id) = query.source_id {
        select = select.filter(sync_error::Column::SourceId.eq(source_id.as_uuid()));
    }
    if let Some(run_id) = query.run_id {
        select = select.filter(sync_error::Column::RunId.eq(run_id.as_uuid()));
    }

    Ok(select
        .order_by_desc(sync_error::Column::CreatedAt)
        .all(repository.connection())
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(|error| SyncErrorDto {
            id: error.id.to_string(),
            run_id: error.run_id.map(RunId::from_uuid),
            source_id: Some(crate::core::types::SourceId::from_uuid(error.source_id)),
            source_path: error.source_path,
            code: error.error_kind,
            message: error.message,
            created_at: Some(error.created_at),
        })
        .collect())
}

async fn deleted_count_for_run(repository: &SeaOrmRepository, run_id: RunId) -> AppResult<u64> {
    let count = sync_item::Entity::find()
        .filter(sync_item::Column::RunId.eq(run_id.as_uuid()))
        .filter(sync_item::Column::Status.eq(sync_status_to_str(SyncStatus::DeletedOnSource)))
        .count(repository.connection())
        .await
        .map_err(map_db_error)?;

    Ok(count)
}

fn item_dto_from_model(item: sync_item::Model) -> AppResult<ItemDto> {
    Ok(ItemDto {
        id: ItemId::from_uuid(item.id),
        source_id: crate::core::types::SourceId::from_uuid(item.source_id),
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

fn run_summary_status_from_str(status: &str) -> AppResult<SyncStatus> {
    match status {
        "running" => Ok(SyncStatus::Pending),
        "completed" => Ok(SyncStatus::Synced),
        "completed_with_failures" | "failed" => Ok(SyncStatus::Failed),
        other => Err(AppError::Database(format!(
            "unknown run status stored in database: {other}"
        ))),
    }
}

fn run_status_from_str(status: &str) -> AppResult<RunStatus> {
    match status {
        "running" => Ok(RunStatus::Running),
        "completed" => Ok(RunStatus::Completed),
        "completed_with_failures" => Ok(RunStatus::CompletedWithFailures),
        "failed" => Ok(RunStatus::Failed),
        "cancelled" => Ok(RunStatus::Cancelled),
        other => Err(AppError::Database(format!(
            "unknown run status stored in database: {other}"
        ))),
    }
}

const fn sync_status_to_str(status: SyncStatus) -> &'static str {
    match status {
        SyncStatus::Pending => "pending",
        SyncStatus::Synced => "synced",
        SyncStatus::Failed => "failed",
        SyncStatus::Skipped => "skipped",
        SyncStatus::DeletedOnSource => "deleted_on_source",
    }
}

fn i64_to_u64(value: i64, field: &str) -> AppResult<u64> {
    u64::try_from(value).map_err(|_| AppError::Database(format!("{field} is negative: {value}")))
}

#[allow(clippy::needless_pass_by_value)]
fn map_db_error(error: sea_orm::DbErr) -> AppError {
    AppError::Database(error.to_string())
}
