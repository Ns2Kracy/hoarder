use std::sync::OnceLock;

use tracing_subscriber::{
    EnvFilter, Registry, fmt, layer::SubscriberExt, reload, util::SubscriberInitExt,
};

use crate::{AppError, AppResult};

type LogReloadHandle = reload::Handle<EnvFilter, Registry>;

static LOG_RELOAD_HANDLE: OnceLock<LogReloadHandle> = OnceLock::new();

pub fn init(log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| env_filter(log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let (filter, handle) = reload::Layer::new(filter);

    if tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .try_init()
        .is_ok()
    {
        let _ = LOG_RELOAD_HANDLE.set(handle);
    }
}

/// Reloads the active log filter when the process owns the subscriber.
///
/// # Errors
///
/// Returns an error when the filter directive is invalid or the subscriber
/// reload handle rejects the update.
pub fn set_level(log_level: &str) -> AppResult<()> {
    let filter = env_filter(log_level)?;
    if let Some(handle) = LOG_RELOAD_HANDLE.get() {
        handle
            .reload(filter)
            .map_err(|error| AppError::Config(format!("failed to reload log level: {error}")))?;
    }

    Ok(())
}

fn env_filter(log_level: &str) -> AppResult<EnvFilter> {
    EnvFilter::try_new(log_level)
        .map_err(|error| AppError::Validation(format!("invalid log level `{log_level}`: {error}")))
}
