use std::{collections::BTreeMap, net::SocketAddr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ConnectorConfigSchema {
    #[serde(rename = "opendal")]
    OpenDal {
        service: String,
        #[serde(default)]
        options: BTreeMap<String, String>,
    },
    #[serde(rename = "notion")]
    Notion {
        token: String,
        #[serde(default, rename = "dataSourceId", alias = "data_source_id")]
        data_source_id: Option<String>,
        #[serde(default, rename = "pageId", alias = "page_id")]
        page_id: Option<String>,
        #[serde(default)]
        version: Option<String>,
        #[serde(default, rename = "baseUrl", alias = "base_url")]
        base_url: Option<String>,
    },
    #[serde(rename = "feishu")]
    Feishu {
        #[serde(rename = "appId", alias = "app_id")]
        app_id: String,
        #[serde(rename = "appSecret", alias = "app_secret")]
        app_secret: String,
        #[serde(default, rename = "folderToken", alias = "folder_token")]
        folder_token: Option<String>,
        #[serde(default, rename = "baseUrl", alias = "base_url")]
        base_url: Option<String>,
    },
    #[serde(rename = "plugin")]
    Plugin {
        #[serde(rename = "pluginId", alias = "plugin_id")]
        plugin_id: String,
        #[serde(default)]
        options: BTreeMap<String, String>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConnectorKindSchema {
    #[serde(rename = "opendal")]
    OpenDal,
    Notion,
    Feishu,
    Plugin,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ItemTypeSchema {
    File,
    Directory,
    VirtualDocument,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum JobStatusSchema {
    Idle,
    Running,
    Paused,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunStatusSchema {
    Running,
    Completed,
    CompletedWithFailures,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SyncStatusSchema {
    Pending,
    Synced,
    Failed,
    Skipped,
    DeletedOnSource,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceListResponse {
    pub data: Vec<SourceDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceTemplateListResponse {
    pub data: Vec<SourceTemplateDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JobListResponse {
    pub data: Vec<JobDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunListResponse {
    pub data: Vec<RunDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemListResponse {
    pub data: Vec<ItemDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncErrorListResponse {
    pub data: Vec<SyncErrorDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceDto {
    #[schema(value_type = i64, minimum = 1)]
    pub id: SourceId,
    pub name: String,
    #[schema(value_type = ConnectorKindSchema)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RedactedConnectorConfig {
    #[schema(value_type = ConnectorKindSchema)]
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
            ConnectorConfig::Notion {
                token,
                data_source_id,
                page_id,
                version,
                base_url,
            } => Self {
                kind: ConnectorKind::Notion,
                service: "notion".to_owned(),
                options: redacted_app_options([
                    ("token", Some(token.as_str()), true),
                    ("data_source_id", data_source_id.as_deref(), false),
                    ("page_id", page_id.as_deref(), false),
                    ("version", version.as_deref(), false),
                    ("base_url", base_url.as_deref(), false),
                ]),
            },
            ConnectorConfig::Feishu {
                app_id,
                app_secret,
                folder_token,
                base_url,
            } => Self {
                kind: ConnectorKind::Feishu,
                service: "feishu".to_owned(),
                options: redacted_app_options([
                    ("app_id", Some(app_id.as_str()), false),
                    ("app_secret", Some(app_secret.as_str()), true),
                    ("folder_token", folder_token.as_deref(), false),
                    ("base_url", base_url.as_deref(), false),
                ]),
            },
            ConnectorConfig::Plugin { plugin_id, options } => {
                let mut options = options
                    .iter()
                    .map(|(key, value)| {
                        let value = if is_secret_key(key) {
                            "<redacted>".to_owned()
                        } else {
                            value.clone()
                        };

                        (key.clone(), value)
                    })
                    .collect::<BTreeMap<_, _>>();
                options.insert("plugin_id".to_owned(), plugin_id.clone());

                Self {
                    kind: ConnectorKind::Plugin,
                    service: "plugin".to_owned(),
                    options,
                }
            }
        }
    }
}

fn redacted_app_options<const N: usize>(
    entries: [(&str, Option<&str>, bool); N],
) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .filter_map(|(key, value, secret)| {
            value.map(|value| {
                let value = if secret {
                    "<redacted>".to_owned()
                } else {
                    value.to_owned()
                };

                (key.to_owned(), value)
            })
        })
        .collect()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceRequest {
    pub name: String,
    #[schema(value_type = ConnectorConfigSchema)]
    pub config: ConnectorConfig,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSourceRequest {
    pub name: String,
    #[schema(value_type = ConnectorConfigSchema)]
    pub config: ConnectorConfig,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceTestResponse {
    pub ok: bool,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceTemplateDto {
    pub id: String,
    pub label: String,
    pub description: String,
    #[schema(value_type = ConnectorKindSchema)]
    pub connector_kind: ConnectorKind,
    pub service: String,
    pub options: Vec<SourceTemplateOptionDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceTemplateOptionDto {
    pub key: String,
    pub label: String,
    pub required: bool,
    pub secret: bool,
    pub default_value: Option<String>,
    pub placeholder: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    #[schema(value_type = i64, minimum = 1)]
    pub id: JobId,
    #[schema(value_type = i64, minimum = 1)]
    pub source_id: SourceId,
    pub name: String,
    pub enabled: bool,
    pub schedule: JobScheduleDto,
    #[schema(value_type = JobStatusSchema)]
    pub status: JobStatus,
    pub last_run_at: Option<DateTime<Utc>>,
    #[schema(value_type = Option<RunStatusSchema>)]
    pub last_run_status: Option<RunStatus>,
    #[schema(value_type = Option<i64>, minimum = 1)]
    pub last_run_id: Option<RunId>,
    pub next_run_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum JobScheduleDto {
    Manual,
    Interval {
        #[serde(rename = "intervalSeconds", alias = "interval_seconds")]
        interval_seconds: u64,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    #[schema(value_type = i64, minimum = 1)]
    pub source_id: SourceId,
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub schedule: JobScheduleDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobRequest {
    #[schema(value_type = i64, minimum = 1)]
    pub source_id: SourceId,
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub schedule: JobScheduleDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunDto {
    #[schema(value_type = i64, minimum = 1)]
    pub id: RunId,
    #[schema(value_type = i64, minimum = 1)]
    pub job_id: JobId,
    #[schema(value_type = i64, minimum = 1)]
    pub source_id: SourceId,
    pub source_name: String,
    pub job_name: String,
    #[schema(value_type = RunStatusSchema)]
    pub status: RunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub processed_count: u64,
    pub synced_count: u64,
    pub skipped_count: u64,
    pub failed_count: u64,
    pub deleted_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunCountsDto {
    pub processed: u64,
    pub synced: u64,
    pub skipped: u64,
    pub failed: u64,
    pub deleted: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunDetailDto {
    #[schema(value_type = i64, minimum = 1)]
    pub id: RunId,
    #[schema(value_type = i64, minimum = 1)]
    pub job_id: JobId,
    #[schema(value_type = i64, minimum = 1)]
    pub source_id: SourceId,
    pub source_name: String,
    pub job_name: String,
    #[schema(value_type = RunStatusSchema)]
    pub status: RunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub counts: RunCountsDto,
    pub errors: Vec<SyncErrorDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    #[schema(value_type = i64, minimum = 1)]
    pub id: ItemId,
    #[schema(value_type = i64, minimum = 1)]
    pub source_id: SourceId,
    pub source_path: String,
    #[schema(value_type = ItemTypeSchema)]
    pub item_type: ItemType,
    #[schema(value_type = SyncStatusSchema)]
    pub status: SyncStatus,
    pub size: Option<u64>,
    pub etag: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
    pub metadata_json: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBrowseQuery {
    pub source_id: SourceId,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileBrowseResponse {
    #[schema(value_type = i64, minimum = 1)]
    pub source_id: SourceId,
    pub path: String,
    pub entries: Vec<FileEntryDto>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileEntryDto {
    pub name: String,
    pub path: String,
    pub kind: FileEntryKind,
    pub item: Option<ItemDto>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileEntryKind {
    Directory,
    File,
    VirtualDocument,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncErrorDto {
    pub id: i64,
    #[schema(value_type = Option<i64>, minimum = 1)]
    pub run_id: Option<RunId>,
    #[schema(value_type = Option<i64>, minimum = 1)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub database_path: String,
    pub vault_path: String,
    #[schema(value_type = String)]
    pub listen_addr: SocketAddr,
    pub job_concurrency: usize,
    pub file_concurrency: usize,
    pub log_level: String,
    pub read_only: ReadOnlySettingsDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadOnlySettingsDto {
    pub database_path: bool,
    pub vault_path: bool,
    pub listen_addr: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobRunResponse {
    #[schema(value_type = i64, minimum = 1)]
    pub run_id: RunId,
    #[schema(value_type = SyncStatusSchema)]
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
        || key.contains("private_key")
        || key.contains("secret_key")
        || key.contains("secret_access_key")
        || key == "key"
}
