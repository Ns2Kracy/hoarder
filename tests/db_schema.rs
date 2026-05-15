use std::collections::BTreeSet;

use hoarder::{
    core::types::{ConnectorKind, JobStatus, RunStatus, SourceId},
    db::{
        connect_sqlite,
        repository::{
            NewScheduledSyncJob, NewSource, NewSyncJob, SeaOrmRepository, SourceRepository,
            SyncJobRepository, SyncJobSchedule,
        },
        schema::sync_schema,
    },
    sync::{
        engine::{SyncRunStatus, SyncRunSummary},
        repository::SyncRepository,
    },
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::json;

#[tokio::test]
async fn db_schema_syncs_expected_tables_and_job_columns() -> Result<(), Box<dyn std::error::Error>>
{
    let db = connect_sqlite("sqlite::memory:").await?;

    sync_schema(&db).await?;

    let table_rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type = 'table'".to_owned(),
        ))
        .await?;
    let table_names = table_rows
        .into_iter()
        .map(|row| row.try_get::<String>("", "name"))
        .collect::<Result<BTreeSet<_>, _>>()?;

    for expected in [
        "app_setting",
        "source",
        "sync_job",
        "sync_run",
        "sync_item",
        "sync_error",
    ] {
        assert!(
            table_names.contains(expected),
            "expected table `{expected}` to be created"
        );
    }

    let sync_job_columns = table_columns(&db, "sync_job").await?;
    for expected in [
        "schedule_kind",
        "schedule_interval_seconds",
        "last_run_at",
        "last_run_status",
        "last_run_id",
    ] {
        assert!(
            sync_job_columns.contains(expected),
            "expected sync_job column `{expected}` to be created"
        );
    }

    Ok(())
}

#[tokio::test]
async fn db_schema_inserts_source_and_scheduled_jobs() -> Result<(), Box<dyn std::error::Error>> {
    let (repository, source_id) = repository_with_source().await?;

    let created_job = repository
        .create_job(NewSyncJob {
            source_id,
            name: "default sync".to_owned(),
            enabled: true,
        })
        .await?;

    assert_eq!(created_job.source_id, source_id);
    assert_eq!(created_job.name, "default sync");
    assert_eq!(created_job.schedule, SyncJobSchedule::Manual);
    assert_eq!(created_job.status, JobStatus::Idle);
    assert_eq!(created_job.last_run_at, None);
    assert_eq!(created_job.last_run_status, None);
    assert_eq!(created_job.last_run_id, None);

    let interval_job = repository
        .create_scheduled_job(NewScheduledSyncJob {
            source_id,
            name: "interval sync".to_owned(),
            enabled: true,
            schedule: SyncJobSchedule::Interval {
                interval_seconds: 300,
            },
        })
        .await?;

    assert_interval_job_is_listed(&repository, source_id, created_job.id, interval_job.id).await?;

    Ok(())
}

#[tokio::test]
async fn db_schema_records_last_run_metadata_on_job() -> Result<(), Box<dyn std::error::Error>> {
    let (repository, source_id) = repository_with_source().await?;
    let created_job = repository
        .create_job(NewSyncJob {
            source_id,
            name: "default sync".to_owned(),
            enabled: true,
        })
        .await?;

    let loaded_job = repository.load_job(created_job.id).await?;
    let run_id = repository.start_run(&loaded_job).await?;
    repository
        .finish_run(
            run_id,
            SyncRunStatus::CompletedWithFailures,
            SyncRunSummary {
                run_id,
                processed: 3,
                synced: 2,
                skipped: 1,
                failed: 1,
                bytes_written: 128,
            },
        )
        .await?;

    let jobs = repository.list_jobs(source_id).await?;
    let finished_job = jobs
        .iter()
        .find(|job| job.id == created_job.id)
        .expect("finished job record exists");
    assert_eq!(finished_job.last_run_id, Some(run_id));
    assert_eq!(
        finished_job.last_run_status,
        Some(RunStatus::CompletedWithFailures)
    );
    assert!(finished_job.last_run_at.is_some());

    Ok(())
}

async fn repository_with_source() -> Result<(SeaOrmRepository, SourceId), Box<dyn std::error::Error>>
{
    let db = connect_sqlite("sqlite::memory:").await?;
    sync_schema(&db).await?;
    let repository = SeaOrmRepository::new(db.clone());
    let created_source = repository
        .create_source(NewSource {
            name: "local files".to_owned(),
            kind: ConnectorKind::OpenDal,
            config_json: json!({
                "kind": "opendal",
                "service": "fs",
                "options": {
                    "root": "/tmp/hoarder-source"
                }
            }),
            enabled: true,
        })
        .await?;

    assert_eq!(created_source.name, "local files");
    assert_eq!(created_source.kind, ConnectorKind::OpenDal);
    assert!(created_source.enabled);
    assert_eq!(created_source.last_check_status, None);
    assert_eq!(created_source.last_checked_at, None);

    let sources = repository.list_sources().await?;
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].id, created_source.id);

    Ok((repository, created_source.id))
}

async fn assert_interval_job_is_listed(
    repository: &SeaOrmRepository,
    source_id: SourceId,
    manual_job_id: hoarder::core::types::JobId,
    interval_job_id: hoarder::core::types::JobId,
) -> Result<(), Box<dyn std::error::Error>> {
    let jobs = repository.list_jobs(source_id).await?;
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().any(|job| job.id == manual_job_id));
    assert!(jobs.iter().any(|job| job.id == interval_job_id
        && job.schedule
            == (SyncJobSchedule::Interval {
                interval_seconds: 300,
            })));

    let missing_source_jobs = repository.list_jobs(SourceId::new()).await?;
    assert!(missing_source_jobs.is_empty());

    Ok(())
}

async fn table_columns(
    db: &impl ConnectionTrait,
    table_name: &str,
) -> Result<BTreeSet<String>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA table_info({table_name})"),
        ))
        .await?;

    rows.into_iter()
        .map(|row| row.try_get::<String>("", "name"))
        .collect()
}
