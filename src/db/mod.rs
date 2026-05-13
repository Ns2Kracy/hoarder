pub mod repository;
pub mod schema;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::{AppError, AppResult};

/// Connects to `SQLite` using either a full `SQLite` URL or a filesystem path.
///
/// # Errors
///
/// Returns an error when the database connection cannot be established.
pub async fn connect_sqlite(database_url: &str) -> AppResult<DatabaseConnection> {
    let url = if database_url.starts_with("sqlite:") {
        database_url.to_owned()
    } else {
        format!("sqlite://{database_url}?mode=rwc")
    };
    let mut options = ConnectOptions::new(url);
    options.sqlx_logging(false);

    Database::connect(options)
        .await
        .map_err(|error| AppError::Database(error.to_string()))
}
