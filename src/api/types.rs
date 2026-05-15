use std::{collections::BTreeMap, net::SocketAddr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    config::{AppConfig, RuntimeSettings},
    connectors::traits::ConnectorConfig,
    core::types::{
        ConnectorKind, ItemId, ItemType, JobId, JobStatus, RunId, RunStatus, SourceId, SyncStatus,
    },
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T> {
    pub data: Vec<T>,
}

impl<T> ListResponse<T> {
    #[must_use]
    pub const fn new(data: Vec<T>) -> Self {
        Self { data }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
}

impl HealthResponse {
    #[must_use]
    pub fn ok() -> Self {
        Self {
            status: "ok".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDto {
    pub id: SourceId,
    pub name: String,
    pub connector_kind: ConnectorKind,
    pub config: RedactedConnectorConfig,
    pub enabled: bool,
    pub health: SourceHealth,
    pub last_checked_at: Option<DateTime<Utc>>,
}

impl SourceDto {
    #[must_use]
    pub fn new(
        id: SourceId,
        name: String,
        config: &ConnectorConfig,
        enabled: bool,
        health: SourceHealth,
        last_checked_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            name,
            connector_kind: config.kind(),
            config: RedactedConnectorConfig::from(config),
            enabled,
            health,
            last_checked_at,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceHealth {
    Healthy,
    Warning,
    Failed,
    Untested,
    Disabled,
}

impl SourceHealth {
    #[must_use]
    pub fn from_record(enabled: bool, last_check_status: Option<&str>) -> Self {
        if !enabled {
            return Self::Disabled;
        }

        match last_check_status {
            Some("healthy") => Self::Healthy,
            Some("warning") => Self::Warning,
            Some("failed") => Self::Failed,
            _ => Self::Untested,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedConnectorConfig {
    pub kind: ConnectorKind,
    pub service: String,
    pub options: BTreeMap<String, String>,
}

impl From<&ConnectorConfig> for RedactedConnectorConfig {
    fn from(config: &ConnectorConfig) -> Self {
        match config {
            ConnectorConfig::OpenDal { service, options } => Self {
                kind: ConnectorKind::OpenDal,
                service: service.clone(),
                options: options
                    .iter()
                    .map(|(key, value)| {
                        let value = if is_secret_key(key) {
                            "<redacted>".to_owned()
                        } else {
                            value.clone()
                        };

                        (key.clone(), value)
                    })
                    .collect(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceRequest {
    pub name: String,
    pub config: ConnectorConfig,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTestResponse {
    pub ok: bool,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    pub id: JobId,
    pub source_id: SourceId,
    pub name: String,
    pub enabled: bool,
    pub schedule: JobScheduleDto,
    pub status: JobStatus,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_run_status: Option<RunStatus>,
    pub last_run_id: Option<RunId>,
    pub next_run_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum JobScheduleDto {
    Manual,
    Interval {
        #[serde(rename = "intervalSeconds", alias = "interval_seconds")]
        interval_seconds: u64,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    pub source_id: SourceId,
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub schedule: JobScheduleDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDto {
    pub id: RunId,
    pub job_id: JobId,
    pub status: SyncStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub processed_count: u64,
    pub synced_count: u64,
    pub skipped_count: u64,
    pub failed_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCountsDto {
    pub processed: u64,
    pub synced: u64,
    pub skipped: u64,
    pub failed: u64,
    pub deleted: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDetailDto {
    pub id: RunId,
    pub job_id: JobId,
    pub source_id: SourceId,
    pub source_name: String,
    pub job_name: String,
    pub status: RunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub counts: RunCountsDto,
    pub errors: Vec<SyncErrorDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: ItemId,
    pub source_id: SourceId,
    pub source_path: String,
    pub item_type: ItemType,
    pub status: SyncStatus,
    pub size: Option<u64>,
    pub etag: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
    pub metadata_json: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncErrorDto {
    pub id: String,
    pub run_id: Option<RunId>,
    pub source_id: Option<SourceId>,
    pub source_path: Option<String>,
    pub code: String,
    pub message: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemListQuery {
    pub source_id: Option<SourceId>,
    pub status: Option<SyncStatus>,
    pub run_id: Option<RunId>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorListQuery {
    pub source_id: Option<SourceId>,
    pub run_id: Option<RunId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub database_path: String,
    pub vault_path: String,
    pub listen_addr: SocketAddr,
    pub job_concurrency: usize,
    pub file_concurrency: usize,
    pub log_level: String,
    pub read_only: ReadOnlySettingsDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadOnlySettingsDto {
    pub database_path: bool,
    pub vault_path: bool,
    pub listen_addr: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    pub job_concurrency: usize,
    pub file_concurrency: usize,
    pub log_level: String,
}

impl From<&AppConfig> for SettingsDto {
    fn from(config: &AppConfig) -> Self {
        Self {
            database_path: config.database_path.to_string_lossy().into_owned(),
            vault_path: config.vault_path.to_string_lossy().into_owned(),
            listen_addr: config.listen_addr,
            job_concurrency: config.job_concurrency,
            file_concurrency: config.file_concurrency,
            log_level: "info".to_owned(),
            read_only: ReadOnlySettingsDto {
                database_path: true,
                vault_path: true,
                listen_addr: true,
            },
        }
    }
}

impl From<RuntimeSettings> for SettingsDto {
    fn from(settings: RuntimeSettings) -> Self {
        Self {
            database_path: settings.database_path,
            vault_path: settings.vault_path,
            listen_addr: settings.listen_addr,
            job_concurrency: settings.job_concurrency,
            file_concurrency: settings.file_concurrency,
            log_level: settings.log_level,
            read_only: ReadOnlySettingsDto {
                database_path: settings.read_only.database_path,
                vault_path: settings.read_only.vault_path,
                listen_addr: settings.read_only.listen_addr,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRunResponse {
    pub run_id: RunId,
    pub status: SyncStatus,
}

const fn default_enabled() -> bool {
    true
}

fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();

    key.contains("password")
        || key.contains("token")
        || key.contains("access_key")
        || key.contains("secret_key")
        || key.contains("secret_access_key")
}
