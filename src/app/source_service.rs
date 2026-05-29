use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait};

use crate::{
    AppError, AppResult,
    api::types::{
        CreateSourceRequest, SourceDto, SourceHealth, SourceTemplateDto, SourceTemplateOptionDto,
        SourceTestResponse, UpdateSourceRequest,
    },
    connectors::traits::ConnectorConfig,
    connectors::{opendal::source::OpenDalSourceConnector, traits::SourceConnector},
    core::types::{ConnectorKind, SourceId},
    db::repository::{NewSource, SeaOrmRepository, SourceRepository, UpdateSource},
    entity::source,
};

/// Lists configured sources.
///
/// # Errors
///
/// Returns an error when database reads fail or stored connector config is invalid.
pub async fn list_sources(repository: &SeaOrmRepository) -> AppResult<Vec<SourceDto>> {
    repository
        .list_sources()
        .await?
        .into_iter()
        .map(|record| {
            let config = connector_config_from_json(record.id, record.config_json)?;
            Ok(SourceDto::new(
                record.id,
                record.name,
                &config,
                record.enabled,
                SourceHealth::from_record(record.enabled, record.last_check_status.as_deref()),
                record.last_checked_at,
            ))
        })
        .collect()
}

#[must_use]
pub fn source_templates() -> Vec<SourceTemplateDto> {
    vec![
        local_filesystem_template(),
        synology_webdav_template(),
        qnap_sftp_template(),
        minio_s3_template(),
        cloudflare_r2_template(),
    ]
}

fn local_filesystem_template() -> SourceTemplateDto {
    source_template(
        "local-filesystem",
        "Local filesystem",
        "Sync a local directory into the Hoarder vault.",
        "fs",
        vec![template_option(
            "root",
            "Root path",
            true,
            false,
            None,
            Some("/Users/alex/Documents/source"),
        )],
    )
}

fn synology_webdav_template() -> SourceTemplateDto {
    source_template(
        "synology-webdav",
        "Synology WebDAV",
        "Use Synology Drive or WebDAV Server with an app password or token.",
        "webdav",
        vec![
            template_option(
                "endpoint",
                "Endpoint",
                true,
                false,
                None,
                Some("https://nas.example.com:5006/home"),
            ),
            template_option(
                "root",
                "Remote root",
                false,
                false,
                None,
                Some("/Documents"),
            ),
            template_option("username", "Username", false, false, None, Some("alice")),
            template_option("token", "Token", false, true, None, Some("app password")),
        ],
    )
}

fn qnap_sftp_template() -> SourceTemplateDto {
    source_template(
        "qnap-sftp",
        "QNAP SFTP",
        "Use SSH/SFTP access to sync a NAS share with key-based auth.",
        "sftp",
        vec![
            template_option(
                "endpoint",
                "Endpoint",
                true,
                false,
                None,
                Some("ssh://nas.example.com:22"),
            ),
            template_option("username", "Username", true, false, None, Some("alice")),
            template_option(
                "root",
                "Remote root",
                false,
                false,
                None,
                Some("/share/docs"),
            ),
            template_option(
                "private_key",
                "Private key",
                false,
                true,
                None,
                Some("/Users/alice/.ssh/id_ed25519"),
            ),
        ],
    )
}

fn minio_s3_template() -> SourceTemplateDto {
    source_template(
        "minio-s3",
        "MinIO / S3-compatible NAS",
        "Use an S3-compatible endpoint hosted by a NAS or local object store.",
        "s3",
        s3_options(Some("us-east-1"), Some("http://127.0.0.1:9000"), false),
    )
}

fn cloudflare_r2_template() -> SourceTemplateDto {
    source_template(
        "cloudflare-r2",
        "Cloudflare R2",
        "Use Cloudflare R2 through its S3-compatible API.",
        "s3",
        s3_options(
            Some("auto"),
            Some("https://account-id.r2.cloudflarestorage.com"),
            true,
        ),
    )
}

fn source_template(
    id: &str,
    label: &str,
    description: &str,
    service: &str,
    options: Vec<SourceTemplateOptionDto>,
) -> SourceTemplateDto {
    SourceTemplateDto {
        id: id.to_owned(),
        label: label.to_owned(),
        description: description.to_owned(),
        connector_kind: ConnectorKind::OpenDal,
        service: service.to_owned(),
        options,
    }
}

fn s3_options(
    region_default: Option<&str>,
    endpoint_placeholder: Option<&str>,
    endpoint_required: bool,
) -> Vec<SourceTemplateOptionDto> {
    vec![
        template_option("bucket", "Bucket", true, false, None, Some("archive")),
        template_option(
            "region",
            "Region",
            true,
            false,
            region_default,
            region_default,
        ),
        template_option(
            "endpoint",
            "Endpoint",
            endpoint_required,
            false,
            None,
            endpoint_placeholder,
        ),
        template_option("root", "Prefix", false, false, None, Some("documents/")),
        template_option("access_key_id", "Access key", true, true, None, None),
        template_option("secret_access_key", "Secret key", true, true, None, None),
    ]
}

fn template_option(
    key: &str,
    label: &str,
    required: bool,
    secret: bool,
    default_value: Option<&str>,
    placeholder: Option<&str>,
) -> SourceTemplateOptionDto {
    SourceTemplateOptionDto {
        key: key.to_owned(),
        label: label.to_owned(),
        required,
        secret,
        default_value: default_value.map(ToOwned::to_owned),
        placeholder: placeholder.map(ToOwned::to_owned),
    }
}

/// Creates a source from an API request.
///
/// # Errors
///
/// Returns an error when connector config serialization or database insert fails.
pub async fn create_source(
    repository: &SeaOrmRepository,
    request: CreateSourceRequest,
) -> AppResult<SourceDto> {
    let CreateSourceRequest {
        name,
        config,
        enabled,
    } = request;
    let config_json = serde_json::to_value(&config).map_err(|error| {
        AppError::Database(format!("failed to serialize connector config: {error}"))
    })?;
    let record = repository
        .create_source(NewSource {
            name,
            kind: config.kind(),
            config_json,
            enabled,
        })
        .await?;

    Ok(SourceDto::new(
        record.id,
        record.name,
        &config,
        record.enabled,
        SourceHealth::from_record(record.enabled, record.last_check_status.as_deref()),
        record.last_checked_at,
    ))
}

/// Updates an existing source from an API request.
///
/// # Errors
///
/// Returns an error when the source is missing, connector config decoding fails,
/// connector config serialization fails, or the database update fails.
pub async fn update_source(
    repository: &SeaOrmRepository,
    source_id: SourceId,
    request: UpdateSourceRequest,
) -> AppResult<SourceDto> {
    let current = repository.load_source(source_id).await?;
    let current_config = connector_config_from_json(current.id, current.config_json)?;
    let UpdateSourceRequest {
        name,
        config,
        enabled,
    } = request;
    let config = merge_redacted_connector_config(&current_config, config);
    let config_json = serde_json::to_value(&config).map_err(|error| {
        AppError::Database(format!("failed to serialize connector config: {error}"))
    })?;
    let record = repository
        .update_source(
            source_id,
            UpdateSource {
                name,
                kind: config.kind(),
                config_json,
                enabled,
            },
        )
        .await?;

    Ok(SourceDto::new(
        record.id,
        record.name,
        &config,
        record.enabled,
        SourceHealth::from_record(record.enabled, record.last_check_status.as_deref()),
        record.last_checked_at,
    ))
}

/// Validates a source connector and records the latest health check.
///
/// # Errors
///
/// Returns an error when the source is missing, connector validation fails, or
/// the health update cannot be persisted.
pub async fn test_source(
    repository: &SeaOrmRepository,
    source_id: SourceId,
) -> AppResult<SourceTestResponse> {
    let source = repository.load_source(source_id).await?;
    let config = connector_config_from_json(source.id, source.config_json)?;
    let checked_at = Utc::now();

    if let Err(error) = validate_source_connector(source.kind, source.id, &config).await {
        update_source_check(repository, source_id, "failed", checked_at).await?;
        return Err(error);
    }

    update_source_check(repository, source_id, "healthy", checked_at).await?;

    Ok(SourceTestResponse {
        ok: true,
        checked_at,
    })
}

async fn validate_source_connector(
    kind: ConnectorKind,
    source_id: SourceId,
    config: &crate::connectors::traits::ConnectorConfig,
) -> AppResult<()> {
    match kind {
        ConnectorKind::OpenDal => {
            OpenDalSourceConnector::new(source_id)
                .validate(config)
                .await?;
            Ok(())
        }
        kind => Err(AppError::NotFound(format!(
            "connector factory not registered for {kind:?}"
        ))),
    }
}

async fn update_source_check(
    repository: &SeaOrmRepository,
    source_id: SourceId,
    status: &str,
    checked_at: chrono::DateTime<Utc>,
) -> AppResult<()> {
    let db = repository.connection();
    let source = source::Entity::find_by_id(source_id.as_i64())
        .one(db)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("source not found: {source_id}")))?;
    let mut active_model: source::ActiveModel = source.into();
    active_model.last_check_status = sea_orm::ActiveValue::Set(Some(status.to_owned()));
    active_model.last_checked_at = sea_orm::ActiveValue::Set(Some(checked_at));
    active_model.updated_at = sea_orm::ActiveValue::Set(Utc::now());
    active_model.update(db).await.map_err(map_db_error)?;

    Ok(())
}

/// Decodes a connector config stored as JSON.
///
/// # Errors
///
/// Returns an error when the stored JSON does not match a known connector config.
pub fn connector_config_from_json(
    source_id: SourceId,
    config_json: serde_json::Value,
) -> AppResult<crate::connectors::traits::ConnectorConfig> {
    serde_json::from_value(config_json).map_err(|error| {
        AppError::Database(format!(
            "invalid connector config for source {source_id}: {error}"
        ))
    })
}

fn merge_redacted_connector_config(
    current: &ConnectorConfig,
    incoming: ConnectorConfig,
) -> ConnectorConfig {
    match (current, incoming) {
        (
            ConnectorConfig::OpenDal {
                service: current_service,
                options: current_options,
            },
            ConnectorConfig::OpenDal {
                service,
                mut options,
            },
        ) => {
            if current_service == &service {
                for (key, value) in current_options {
                    if is_secret_option_key(key) && !options.contains_key(key) {
                        options.insert(key.clone(), value.clone());
                    }
                }

                for (key, value) in &mut options {
                    if is_secret_option_key(key)
                        && is_redacted_value(value)
                        && let Some(current_value) = current_options.get(key)
                    {
                        value.clone_from(current_value);
                    }
                }
            }

            ConnectorConfig::OpenDal { service, options }
        }
    }
}

fn is_secret_option_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();

    normalized.contains("password")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("privatekey")
        || normalized == "key"
        || (normalized.contains("access") && normalized.contains("key"))
}

fn is_redacted_value(value: &str) -> bool {
    matches!(value.trim(), "<redacted>" | "[redacted]" | "redacted")
}

#[allow(clippy::needless_pass_by_value)]
fn map_db_error(error: sea_orm::DbErr) -> AppError {
    AppError::Database(error.to_string())
}
