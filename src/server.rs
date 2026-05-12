use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use futures::FutureExt;
use tokio::net::TcpListener;

use crate::{
    AppConfig, AppError, AppResult,
    api::{
        routes::router,
        state::{ApiFuture, ApiRepository, ApiState, SyncService},
        types::{
            CreateSourceRequest, ItemDto, JobDto, JobRunResponse, RunDto, SettingsDto, SourceDto,
            SyncErrorDto,
        },
    },
    connectors::registry::ConnectorRegistry,
    core::types::{JobId, RunId, SyncStatus},
};

#[derive(Clone, Debug, Default)]
pub struct ServeOptions {
    pub config_path: Option<PathBuf>,
    pub addr: Option<SocketAddr>,
}

pub async fn serve(options: ServeOptions) -> AppResult<()> {
    let config = load_config(options.config_path.as_deref()).await?;
    let config = apply_addr_override(config, options.addr);
    let addr = config.listen_addr;
    let db = connect_sqlite(&config).await?;
    sync_schema(&db).await?;
    let _connector_registry = build_connector_registry();
    let app = router(default_state(config));
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(%addr, "hoarder API listening");
    println!("hoarder API listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

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

fn apply_addr_override(mut config: AppConfig, addr: Option<SocketAddr>) -> AppConfig {
    if let Some(addr) = addr {
        config.listen_addr = addr;
    }

    config
}

async fn connect_sqlite(config: &AppConfig) -> AppResult<sea_orm::DatabaseConnection> {
    if let Some(parent) = config.database_path.parent() {
        if !parent.as_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let database_url = sqlite_url(config);

    sea_orm::Database::connect(&database_url)
        .await
        .map_err(|error| AppError::Config(format!("database connection failed: {error}")))
}

fn sqlite_url(config: &AppConfig) -> String {
    let path = config.database_path.as_str();
    if path == ":memory:" {
        return "sqlite://:memory:".to_owned();
    }

    format!("sqlite://{path}?mode=rwc")
}

async fn sync_schema(_db: &sea_orm::DatabaseConnection) -> AppResult<()> {
    Ok(())
}

fn build_connector_registry() -> ConnectorRegistry {
    ConnectorRegistry::new()
}

pub fn config_with_addr(addr: Option<SocketAddr>) -> AppConfig {
    apply_addr_override(AppConfig::default(), addr)
}

pub fn default_state(config: AppConfig) -> ApiState {
    ApiState::new(
        Arc::new(InMemoryApiRepository::new(config)),
        Arc::new(PlaceholderSyncService::default()),
    )
}

struct InMemoryApiRepository {
    config: AppConfig,
    sources: Mutex<Vec<SourceDto>>,
}

impl InMemoryApiRepository {
    fn new(config: AppConfig) -> Self {
        Self {
            config,
            sources: Mutex::new(Vec::new()),
        }
    }
}

impl ApiRepository for InMemoryApiRepository {
    fn list_sources(&self) -> ApiFuture<'_, Vec<SourceDto>> {
        async move { Ok(self.sources.lock().unwrap().clone()) }.boxed()
    }

    fn create_source(&self, request: CreateSourceRequest) -> ApiFuture<'_, SourceDto> {
        async move {
            let source = SourceDto::new(
                crate::core::types::SourceId::new(),
                request.name,
                &request.config,
                request.enabled,
            );
            self.sources.lock().unwrap().push(source.clone());

            Ok(source)
        }
        .boxed()
    }

    fn list_jobs(&self) -> ApiFuture<'_, Vec<JobDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_runs(&self) -> ApiFuture<'_, Vec<RunDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_items(&self) -> ApiFuture<'_, Vec<ItemDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn list_errors(&self) -> ApiFuture<'_, Vec<SyncErrorDto>> {
        async move { Ok(vec![]) }.boxed()
    }

    fn settings(&self) -> ApiFuture<'_, SettingsDto> {
        async move { Ok(SettingsDto::from(&self.config)) }.boxed()
    }
}

#[derive(Default)]
struct PlaceholderSyncService;

impl SyncService for PlaceholderSyncService {
    fn run_job(&self, _job_id: JobId) -> ApiFuture<'_, JobRunResponse> {
        async move {
            Ok(JobRunResponse {
                run_id: RunId::new(),
                status: SyncStatus::Pending,
            })
        }
        .boxed()
    }
}
