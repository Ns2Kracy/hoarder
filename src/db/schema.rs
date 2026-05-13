use sea_orm::DatabaseConnection;

use crate::{AppError, AppResult};

/// Synchronizes the `SQLite` schema from `SeaORM` entity definitions.
///
/// # Errors
///
/// Returns an error when schema discovery or synchronization fails.
pub async fn sync_schema(db: &DatabaseConnection) -> AppResult<()> {
    db.get_schema_registry("hoarder::entity::*")
        .sync(db)
        .await
        .map_err(|error| AppError::Database(error.to_string()))
}
