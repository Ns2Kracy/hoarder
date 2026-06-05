use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use crate::{AppError, AppResult};

const INDEX_STATEMENTS: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_sync_job_source_id ON sync_job(source_id)",
    "CREATE INDEX IF NOT EXISTS idx_sync_job_enabled_schedule ON sync_job(enabled, schedule_kind, schedule_interval_seconds)",
    "CREATE INDEX IF NOT EXISTS idx_sync_run_started_at ON sync_run(started_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_sync_run_job_id ON sync_run(job_id)",
    "CREATE INDEX IF NOT EXISTS idx_sync_run_source_id ON sync_run(source_id)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_item_source_path ON sync_item(source_id, source_path)",
    "CREATE INDEX IF NOT EXISTS idx_sync_item_source_status_path ON sync_item(source_id, status, source_path)",
    "CREATE INDEX IF NOT EXISTS idx_sync_item_source_last_run ON sync_item(source_id, last_run_id)",
    "CREATE INDEX IF NOT EXISTS idx_sync_error_run_created ON sync_error(run_id, created_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_sync_error_source_created ON sync_error(source_id, created_at DESC)",
];

/// Synchronizes the `SQLite` schema from `SeaORM` entity definitions.
///
/// # Errors
///
/// Returns an error when schema discovery or synchronization fails.
pub async fn sync_schema(db: &DatabaseConnection) -> AppResult<()> {
    db.get_schema_registry("hoarder_server::entity::*")
        .sync(db)
        .await
        .map_err(|error| AppError::Database(error.to_string()))?;

    for statement in INDEX_STATEMENTS {
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            (*statement).to_owned(),
        ))
        .await
        .map_err(|error| AppError::Database(error.to_string()))?;
    }

    Ok(())
}
