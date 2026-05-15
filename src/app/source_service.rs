use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait};

use crate::{
    AppError, AppResult,
    api::types::{CreateSourceRequest, SourceDto, SourceHealth, SourceTestResponse},
    connectors::{opendal::source::OpenDalSourceConnector, traits::SourceConnector},
    core::types::{ConnectorKind, SourceId},
    db::repository::{NewSource, SeaOrmRepository, SourceRepository},
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
    let source = source::Entity::find_by_id(source_id.as_uuid())
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

#[allow(clippy::needless_pass_by_value)]
fn map_db_error(error: sea_orm::DbErr) -> AppError {
    AppError::Database(error.to_string())
}
