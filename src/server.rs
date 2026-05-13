use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{Router, http::HeaderName, middleware};
use sea_orm::{ConnectOptions, DatabaseConnection};
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    propagate_header::PropagateHeaderLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
};

use crate::{
    AppConfig, AppError, AppResult,
    api::{routes::router_without_fallback, state::ApiState},
    assets,
    db::{repository::SeaOrmRepository, schema::sync_schema},
    middleware::logger::logger as middleware_logger,
};

#[derive(Clone, Debug, Default)]
pub struct ServeOptions {
    pub config_path: Option<PathBuf>,
    pub addr: Option<SocketAddr>,
}

/// Starts the local Axum server.
///
/// # Errors
///
/// Returns an error when config loading, database setup, binding, or serving
/// fails.
pub async fn serve(options: ServeOptions) -> AppResult<()> {
    let config = load_config(options.config_path.as_deref()).await?;
    let config = apply_addr_override(config, options.addr);
    let addr = config.listen_addr;
    let app = app(config).await?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(%addr, "hoarder API listening");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Synchronizes the configured database schema.
///
/// # Errors
///
/// Returns an error when config loading, database connection, or schema sync
/// fails.
pub async fn sync_database(config_path: Option<PathBuf>) -> AppResult<()> {
    let config = load_config(config_path.as_deref()).await?;
    let db = connect_sqlite(&config).await?;

    sync_schema(&db).await
}

async fn load_config(path: Option<&Path>) -> AppResult<AppConfig> {
    let Some(path) = path else {
        return Ok(AppConfig::default());
    };

    let config = tokio::fs::read_to_string(path).await?;

    serde_json::from_str(&config).map_err(|error| {
        AppError::Config(format!(
            "failed to parse JSON config at {}: {error}",
            path.display()
        ))
    })
}

const fn apply_addr_override(mut config: AppConfig, addr: Option<SocketAddr>) -> AppConfig {
    if let Some(addr) = addr {
        config.listen_addr = addr;
    }

    config
}

async fn connect_sqlite(config: &AppConfig) -> AppResult<DatabaseConnection> {
    if let Some(parent) = config.database_path.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await?;
    }

    let database_url = sqlite_url(config);
    let mut options = ConnectOptions::new(database_url);
    options.sqlx_logging(false);

    sea_orm::Database::connect(options)
        .await
        .map_err(|error| AppError::Config(format!("database connection failed: {error}")))
}

fn sqlite_url(config: &AppConfig) -> String {
    let path = config.database_path.to_string_lossy();
    if path == ":memory:" {
        return "sqlite::memory:".to_owned();
    }

    format!("sqlite://{path}?mode=rwc")
}

#[must_use]
pub fn config_with_addr(addr: Option<SocketAddr>) -> AppConfig {
    apply_addr_override(AppConfig::default(), addr)
}

/// Builds the HTTP app with an existing database connection.
pub fn app_with_db(config: AppConfig, db: DatabaseConnection) -> Router {
    app_with_state(ApiState::new(Arc::new(SeaOrmRepository::new(db)), config))
}

/// Builds the HTTP app using the configured database.
///
/// # Errors
///
/// Returns an error when connecting to or synchronizing the configured database
/// fails.
pub async fn app(config: AppConfig) -> AppResult<Router> {
    let db = connect_sqlite(&config).await?;
    sync_schema(&db).await?;

    Ok(app_with_db(config, db))
}

fn app_with_state(state: ApiState) -> Router {
    Router::new()
        .merge(router_without_fallback(state))
        .fallback(assets::serve)
        .layer(middleware::from_fn(middleware_logger))
        .layer(CompressionLayer::new())
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(CorsLayer::permissive())
}
