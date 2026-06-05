use std::collections::BTreeMap;

use hoarder_core::{
    types::{ItemId, ItemType, RunId, RunStatus, SyncStatus},
    vault_path::normalize_source_path,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::{
    AppError, AppResult,
    api::types::{
        ErrorListQuery, FileBrowseQuery, FileBrowseResponse, FileEntryDto, FileEntryKind, ItemDto,
        ItemListQuery, RunCountsDto, RunDetailDto, RunDto, SyncErrorDto,
    },
    db::repository::SeaOrmRepository,
    entity::{sync_error, sync_item, sync_run},
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
                id: RunId::from_i64(run.id),
                job_id: hoarder_core::types::JobId::from_i64(run.job_id),
                source_id: hoarder_core::types::SourceId::from_i64(run.source_id),
                source_name: run.source_name,
                job_name: run.job_name,
                status: run_status_from_str(&run.status)?,
                started_at: Some(run.started_at),
                finished_at: run.finished_at,
                processed_count: i64_to_u64(run.processed_count, "processed_count")?,
                synced_count: i64_to_u64(run.synced_count, "synced_count")?,
                skipped_count: i64_to_u64(run.skipped_count, "skipped_count")?,
                failed_count: i64_to_u64(run.failed_count, "failed_count")?,
                deleted_count: i64_to_u64(run.deleted_count, "deleted_count")?,
            })
        })
        .collect()
}

/// Loads one sync run with source, job, counts, and recent errors.
///
/// # Errors
///
/// Returns an error when the run cannot be found or stored run metadata is
/// invalid. Source and job names are read from the run snapshot.
pub async fn get_run_detail(
    repository: &SeaOrmRepository,
    run_id: RunId,
) -> AppResult<RunDetailDto> {
    let db = repository.connection();
    let run = sync_run::Entity::find_by_id(run_id.as_i64())
        .one(db)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("sync run not found: {run_id}")))?;
    let errors = list_errors(
        repository,
        ErrorListQuery {
            run_id: Some(run_id),
            source_id: None,
        },
    )
    .await?;

    Ok(RunDetailDto {
        id: run_id,
        job_id: hoarder_core::types::JobId::from_i64(run.job_id),
        source_id: hoarder_core::types::SourceId::from_i64(run.source_id),
        source_name: run.source_name,
        job_name: run.job_name,
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
            deleted: i64_to_u64(run.deleted_count, "deleted_count")?,
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
        select = select.filter(sync_item::Column::SourceId.eq(source_id.as_i64()));
    }
    if let Some(run_id) = query.run_id {
        select = select.filter(sync_item::Column::LastRunId.eq(run_id.as_i64()));
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

/// Browses synced source items as one directory level.
///
/// # Errors
///
/// Returns an error when the source does not exist, the requested path is
/// invalid, database reads fail, or stored item metadata is invalid.
pub async fn browse_files(
    repository: &SeaOrmRepository,
    query: FileBrowseQuery,
) -> AppResult<FileBrowseResponse> {
    repository.load_source(query.source_id).await?;

    let path = normalize_browse_path(query.path.as_deref())?;
    let mut entries = BTreeMap::<String, FileEntryDto>::new();
    let items = sync_item::Entity::find()
        .filter(sync_item::Column::SourceId.eq(query.source_id.as_i64()))
        .filter(sync_item::Column::Status.ne(sync_status_to_str(SyncStatus::DeletedOnSource)))
        .order_by_asc(sync_item::Column::SourcePath)
        .all(repository.connection())
        .await
        .map_err(map_db_error)?;

    for item in items {
        let Some(relative_path) = child_relative_path(&path, &item.source_path) else {
            continue;
        };
        let Some((name, is_nested)) = child_name(relative_path) else {
            continue;
        };
        let name = name.to_owned();
        let child_path = join_browse_path(&path, &name);

        if is_nested {
            entries.entry(child_path.clone()).or_insert(FileEntryDto {
                name,
                path: child_path,
                kind: FileEntryKind::Directory,
                item: None,
            });
            continue;
        }

        let item_dto = item_dto_from_model(item)?;
        entries.insert(
            child_path.clone(),
            FileEntryDto {
                name,
                path: child_path,
                kind: file_entry_kind(item_dto.item_type),
                item: Some(item_dto),
            },
        );
    }

    let mut entries = entries.into_values().collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        file_entry_sort_rank(left.kind)
            .cmp(&file_entry_sort_rank(right.kind))
            .then_with(|| left.name.cmp(&right.name))
    });

    Ok(FileBrowseResponse {
        source_id: query.source_id,
        path,
        entries,
    })
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
        select = select.filter(sync_error::Column::SourceId.eq(source_id.as_i64()));
    }
    if let Some(run_id) = query.run_id {
        select = select.filter(sync_error::Column::RunId.eq(run_id.as_i64()));
    }

    Ok(select
        .order_by_desc(sync_error::Column::CreatedAt)
        .all(repository.connection())
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(|error| SyncErrorDto {
            id: error.id,
            run_id: error.run_id.map(RunId::from_i64),
            source_id: error.source_id.map(hoarder_core::types::SourceId::from_i64),
            source_path: error.source_path,
            code: error.error_kind,
            message: error.message,
            created_at: Some(error.created_at),
        })
        .collect())
}

fn item_dto_from_model(item: sync_item::Model) -> AppResult<ItemDto> {
    Ok(ItemDto {
        id: ItemId::from_i64(item.id),
        source_id: hoarder_core::types::SourceId::from_i64(item.source_id),
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

fn normalize_browse_path(path: Option<&str>) -> AppResult<String> {
    let Some(path) = path.map(str::trim).filter(|path| !path.is_empty()) else {
        return Ok(String::new());
    };

    normalize_source_path(path)
}

fn child_relative_path<'a>(browse_path: &str, source_path: &'a str) -> Option<&'a str> {
    if browse_path.is_empty() {
        return Some(source_path);
    }

    if source_path == browse_path {
        return None;
    }

    source_path.strip_prefix(&format!("{browse_path}/"))
}

fn child_name(relative_path: &str) -> Option<(&str, bool)> {
    if relative_path.is_empty() {
        return None;
    }

    match relative_path.split_once('/') {
        Some((name, _)) if !name.is_empty() => Some((name, true)),
        Some(_) => None,
        None => Some((relative_path, false)),
    }
}

fn join_browse_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

const fn file_entry_kind(item_type: ItemType) -> FileEntryKind {
    match item_type {
        ItemType::File => FileEntryKind::File,
        ItemType::Directory => FileEntryKind::Directory,
        ItemType::VirtualDocument => FileEntryKind::VirtualDocument,
    }
}

const fn file_entry_sort_rank(kind: FileEntryKind) -> u8 {
    match kind {
        FileEntryKind::Directory => 0,
        FileEntryKind::File | FileEntryKind::VirtualDocument => 1,
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
