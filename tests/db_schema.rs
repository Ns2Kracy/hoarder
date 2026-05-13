use std::collections::BTreeSet;

use hoarder::{
    core::types::{ConnectorKind, SourceId},
    db::{
        connect_sqlite,
        repository::{
            NewSource, NewSyncJob, SeaOrmRepository, SourceRepository, SyncJobRepository,
        },
        schema::sync_schema,
    },
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::json;

#[tokio::test]
async fn db_schema_syncs_tables_and_inserts_source_job() -> Result<(), Box<dyn std::error::Error>> {
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

    let created_job = repository
        .create_job(NewSyncJob {
            source_id: created_source.id,
            name: "default sync".to_owned(),
            enabled: true,
        })
        .await?;

    assert_eq!(created_job.source_id, created_source.id);
    assert_eq!(created_job.name, "default sync");
    assert_eq!(created_job.status, "idle");

    let jobs = repository.list_jobs(created_source.id).await?;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, created_job.id);

    let missing_source_jobs = repository.list_jobs(SourceId::new()).await?;
    assert!(missing_source_jobs.is_empty());

    Ok(())
}
