use std::{path::PathBuf, sync::Arc};

use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder};

use crate::{
    AppError, AppResult,
    api::types::{CreateJobRequest, JobDto, JobRunResponse, JobScheduleDto},
    connectors::{opendal::source::OpenDalSourceConnector, traits::SourceConnector},
    core::types::{ConnectorKind, JobId, JobStatus, RunId, RunStatus, SourceId, SyncStatus},
    db::repository::{
        NewScheduledSyncJob, SeaOrmRepository, SyncJobRecord, SyncJobRepository, SyncJobSchedule,
    },
    entity::sync_job,
    sync::{engine::SyncEngine, vault_writer::VaultWriter},
};

/// Lists all sync jobs.
///
/// # Errors
///
/// Returns an error when database reads fail or stored job metadata is invalid.
pub async fn list_jobs(repository: &SeaOrmRepository) -> AppResult<Vec<JobDto>> {
    let jobs = sync_job::Entity::find()
        .order_by_asc(sync_job::Column::Name)
        .all(repository.connection())
        .await
        .map_err(map_db_error)?;

    jobs.into_iter()
        .map(|job| {
            let record = SyncJobRecord {
                id: JobId::from_uuid(job.id),
                source_id: SourceId::from_uuid(job.source_id),
                name: job.name,
                enabled: job.enabled,
                schedule: schedule_from_storage(&job.schedule_kind, job.schedule_interval_seconds)?,
                status: job_status_from_str(&job.status)?,
                cursor: job.cursor,
                last_run_at: job.last_run_at,
                last_run_status: job
                    .last_run_status
                    .as_deref()
                    .map(run_status_from_str)
                    .transpose()?,
                last_run_id: job.last_run_id.map(RunId::from_uuid),
                created_at: job.created_at,
                updated_at: job.updated_at,
            };

            Ok(job_dto_from_record(record))
        })
        .collect()
}

/// Creates a sync job from an API request.
///
/// # Errors
///
/// Returns an error when the schedule is invalid or the database insert fails.
pub async fn create_job(
    repository: &SeaOrmRepository,
    request: CreateJobRequest,
) -> AppResult<JobDto> {
    let schedule = schedule_from_dto(&request.schedule)?;
    let record = repository
        .create_scheduled_job(NewScheduledSyncJob {
            source_id: request.source_id,
            name: request.name,
            enabled: request.enabled,
            schedule,
        })
        .await?;

    Ok(job_dto_from_record(record))
}

/// Runs a sync job through the shared sync engine.
///
/// # Errors
///
/// Returns an error when the job is missing, disabled, already running, or when
/// the sync engine fails.
pub async fn run_job(
    repository: Arc<SeaOrmRepository>,
    vault_path: PathBuf,
    job_id: JobId,
) -> AppResult<JobRunResponse> {
    let source_id = mark_job_running(repository.as_ref(), job_id).await?;
    let engine = SyncEngine::new(
        Arc::clone(&repository),
        Arc::new(move |kind| source_connector(kind, source_id)),
        VaultWriter::new(vault_path),
    );

    match engine.run_job(job_id).await {
        Ok(summary) => Ok(JobRunResponse {
            run_id: summary.run_id,
            status: if summary.failed == 0 {
                SyncStatus::Synced
            } else {
                SyncStatus::Failed
            },
        }),
        Err(error) => {
            set_job_status(repository.as_ref(), job_id, JobStatus::Failed).await?;
            Err(error)
        }
    }
}

async fn mark_job_running(repository: &SeaOrmRepository, job_id: JobId) -> AppResult<SourceId> {
    let db = repository.connection();
    let job = sync_job::Entity::find_by_id(job_id.as_uuid())
        .one(db)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("sync job not found: {job_id}")))?;
    let status = job_status_from_str(&job.status)?;
    if status == JobStatus::Running {
        return Err(AppError::Conflict(format!(
            "sync job is already running: {job_id}"
        )));
    }
    if !job.enabled {
        return Err(AppError::Unprocessable(format!(
            "sync job is disabled: {job_id}"
        )));
    }

    let source_id = SourceId::from_uuid(job.source_id);
    let mut active_model: sync_job::ActiveModel = job.into();
    active_model.status = sea_orm::ActiveValue::Set("running".to_owned());
    active_model.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
    active_model.update(db).await.map_err(map_db_error)?;

    Ok(source_id)
}

async fn set_job_status(
    repository: &SeaOrmRepository,
    job_id: JobId,
    status: JobStatus,
) -> AppResult<()> {
    let db = repository.connection();
    let job = sync_job::Entity::find_by_id(job_id.as_uuid())
        .one(db)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("sync job not found: {job_id}")))?;
    let mut active_model: sync_job::ActiveModel = job.into();
    active_model.status = sea_orm::ActiveValue::Set(job_status_to_str(status).to_owned());
    active_model.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
    active_model.update(db).await.map_err(map_db_error)?;

    Ok(())
}

fn job_dto_from_record(record: SyncJobRecord) -> JobDto {
    JobDto {
        id: record.id,
        source_id: record.source_id,
        name: record.name,
        enabled: record.enabled,
        schedule: schedule_to_dto(&record.schedule),
        status: record.status,
        last_run_at: record.last_run_at,
        last_run_status: record.last_run_status,
        last_run_id: record.last_run_id,
        next_run_at: next_run_at(record.last_run_at, &record.schedule),
    }
}

fn schedule_from_dto(schedule: &JobScheduleDto) -> AppResult<SyncJobSchedule> {
    match schedule {
        JobScheduleDto::Manual => Ok(SyncJobSchedule::Manual),
        JobScheduleDto::Interval { interval_seconds } if *interval_seconds > 0 => {
            Ok(SyncJobSchedule::Interval {
                interval_seconds: *interval_seconds,
            })
        }
        JobScheduleDto::Interval { .. } => Err(AppError::Unprocessable(
            "intervalSeconds must be greater than zero".to_owned(),
        )),
    }
}

const fn schedule_to_dto(schedule: &SyncJobSchedule) -> JobScheduleDto {
    match schedule {
        SyncJobSchedule::Manual => JobScheduleDto::Manual,
        SyncJobSchedule::Interval { interval_seconds } => JobScheduleDto::Interval {
            interval_seconds: *interval_seconds,
        },
    }
}

fn next_run_at(
    last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    schedule: &SyncJobSchedule,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let SyncJobSchedule::Interval { interval_seconds } = schedule else {
        return None;
    };
    let seconds = i64::try_from(*interval_seconds).ok()?;
    last_run_at.map(|last_run_at| last_run_at + chrono::Duration::seconds(seconds))
}

fn schedule_from_storage(kind: &str, interval_seconds: Option<i64>) -> AppResult<SyncJobSchedule> {
    match kind {
        "manual" => Ok(SyncJobSchedule::Manual),
        "interval" => {
            let interval_seconds = interval_seconds.ok_or_else(|| {
                AppError::Database("interval job is missing schedule_interval_seconds".to_owned())
            })?;
            let interval_seconds = u64::try_from(interval_seconds).map_err(|_| {
                AppError::Database(format!(
                    "schedule_interval_seconds is negative: {interval_seconds}"
                ))
            })?;
            Ok(SyncJobSchedule::Interval { interval_seconds })
        }
        other => Err(AppError::Database(format!(
            "unknown schedule kind stored in database: {other}"
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

fn job_status_from_str(status: &str) -> AppResult<JobStatus> {
    match status {
        "idle" => Ok(JobStatus::Idle),
        "running" => Ok(JobStatus::Running),
        "paused" => Ok(JobStatus::Paused),
        "failed" => Ok(JobStatus::Failed),
        other => Err(AppError::Database(format!(
            "unknown job status stored in database: {other}"
        ))),
    }
}

const fn job_status_to_str(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Idle => "idle",
        JobStatus::Running => "running",
        JobStatus::Paused => "paused",
        JobStatus::Failed => "failed",
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

#[allow(clippy::needless_pass_by_value)]
fn map_db_error(error: sea_orm::DbErr) -> AppError {
    AppError::Database(error.to_string())
}
