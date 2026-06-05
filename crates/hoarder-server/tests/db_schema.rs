use std::collections::BTreeSet;

use hoarder_core::{
    AppError,
    types::{ConnectorKind, JobStatus, RunStatus, SourceId},
};
use hoarder_server::db::{
    connect_sqlite,
    repository::{
        NewScheduledSyncJob, NewSource, NewSyncJob, SeaOrmRepository, SourceRepository,
        SyncJobRepository, SyncJobSchedule,
    },
    schema::sync_schema,
};
use hoarder_sync::{
    engine::{SyncRunStatus, SyncRunSummary},
    repository::SyncRepository,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, EntityTrait, Statement};
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
async fn db_schema_creates_flat_tables_without_foreign_keys_and_with_indexes()
-> Result<(), Box<dyn std::error::Error>> {
    let db = connect_sqlite("sqlite::memory:").await?;

    sync_schema(&db).await?;

    for table in ["source", "sync_job", "sync_run", "sync_item", "sync_error"] {
        assert!(
            foreign_keys(&db, table).await?.is_empty(),
            "{table} should not have database foreign keys"
        );
    }

    let sync_run_columns = table_columns(&db, "sync_run").await?;
    assert!(sync_run_columns.contains("source_name"));
    assert!(sync_run_columns.contains("job_name"));
    assert!(sync_run_columns.contains("deleted_count"));
    assert!(sync_run_columns.contains("bytes_written"));

    let sync_item_columns = table_columns(&db, "sync_item").await?;
    assert!(sync_item_columns.contains("last_run_id"));
    assert!(!sync_item_columns.contains("run_id"));

    let sync_error_columns = table_columns(&db, "sync_error").await?;
    assert!(!sync_error_columns.contains("item_id"));
    let sync_error_nullable_columns = nullable_columns(&db, "sync_error").await?;
    for expected in ["source_id", "job_id", "run_id", "source_path"] {
        assert!(
            sync_error_nullable_columns.contains(expected),
            "sync_error.{expected} should be nullable"
        );
    }

    let all_indexes = all_index_names(&db).await?;
    for expected in [
        "idx_sync_job_source_id",
        "idx_sync_job_enabled_schedule",
        "idx_sync_run_started_at",
        "idx_sync_run_job_id",
        "idx_sync_run_source_id",
        "idx_sync_item_source_path",
        "idx_sync_item_source_status_path",
        "idx_sync_item_source_last_run",
        "idx_sync_error_run_created",
        "idx_sync_error_source_created",
    ] {
        assert!(all_indexes.contains(expected), "missing index {expected}");
    }

    Ok(())
}

#[tokio::test]
async fn db_schema_rejects_job_creation_for_missing_source_in_repository()
-> Result<(), Box<dyn std::error::Error>> {
    let db = connect_sqlite("sqlite::memory:").await?;
    sync_schema(&db).await?;
    let repository = SeaOrmRepository::new(db);

    let error = repository
        .create_job(NewSyncJob {
            source_id: SourceId::from_i64(999_999),
            name: "orphan job".to_owned(),
            enabled: true,
        })
        .await
        .expect_err("job creation should validate source existence");

    assert!(matches!(error, AppError::NotFound(_)));

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
                deleted: 1,
                bytes_written: 128,
            },
            Some("cursor-after-run".to_owned()),
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
    assert_eq!(finished_job.cursor, Some("cursor-after-run".to_owned()));

    let finished_run = hoarder_server::entity::sync_run::Entity::find_by_id(run_id.as_i64())
        .one(repository.connection())
        .await?
        .expect("finished run exists");
    assert_eq!(finished_run.deleted_count, 1);

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
    manual_job_id: hoarder_core::types::JobId,
    interval_job_id: hoarder_core::types::JobId,
) -> Result<(), Box<dyn std::error::Error>> {
    let jobs = repository.list_jobs(source_id).await?;
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().any(|job| job.id == manual_job_id));
    assert!(jobs.iter().any(|job| job.id == interval_job_id
        && job.schedule
            == (SyncJobSchedule::Interval {
                interval_seconds: 300,
            })));

    let missing_source_jobs = repository.list_jobs(SourceId::from_i64(999_999)).await?;
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

async fn nullable_columns(
    db: &impl ConnectionTrait,
    table_name: &str,
) -> Result<BTreeSet<String>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA table_info({table_name})"),
        ))
        .await?;

    let mut columns = BTreeSet::new();
    for row in rows {
        let notnull = row.try_get::<i64>("", "notnull")?;
        if notnull == 0 {
            columns.insert(row.try_get::<String>("", "name")?);
        }
    }

    Ok(columns)
}

async fn foreign_keys(
    db: &impl ConnectionTrait,
    table_name: &str,
) -> Result<Vec<String>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA foreign_key_list({table_name})"),
        ))
        .await?;

    rows.into_iter()
        .map(|row| row.try_get::<String>("", "table"))
        .collect()
}

async fn all_index_names(db: &impl ConnectionTrait) -> Result<BTreeSet<String>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type = 'index'".to_owned(),
        ))
        .await?;

    rows.into_iter()
        .map(|row| row.try_get::<String>("", "name"))
        .collect()
}
