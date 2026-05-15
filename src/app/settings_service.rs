use crate::{
    AppConfig, AppResult,
    api::types::{SettingsDto, UpdateSettingsRequest},
    config::RuntimeSettingsPatch,
    db::repository::{RuntimeSettingsRepository, SeaOrmRepository},
};

/// Returns boot config merged with persisted runtime settings.
///
/// # Errors
///
/// Returns an error when persisted settings cannot be read.
pub async fn get_settings(
    repository: &SeaOrmRepository,
    config: &AppConfig,
) -> AppResult<SettingsDto> {
    Ok(repository.load_runtime_settings(config).await?.into())
}

/// Persists mutable runtime settings.
///
/// # Errors
///
/// Returns an error when settings are invalid or cannot be persisted.
pub async fn update_settings(
    repository: &SeaOrmRepository,
    config: &AppConfig,
    request: UpdateSettingsRequest,
) -> AppResult<SettingsDto> {
    Ok(repository
        .patch_runtime_settings(
            config,
            RuntimeSettingsPatch {
                job_concurrency: Some(request.job_concurrency),
                file_concurrency: Some(request.file_concurrency),
                log_level: Some(request.log_level),
            },
        )
        .await?
        .into())
}
