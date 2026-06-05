use std::{path::PathBuf, sync::Arc};

use crate::{AppConfig, app::run_control::JobRunRegistry, db::repository::SeaOrmRepository};

#[derive(Clone)]
pub struct ApiState {
    repository: Arc<SeaOrmRepository>,
    config: AppConfig,
    run_registry: Arc<JobRunRegistry>,
}

impl ApiState {
    #[must_use]
    pub fn new(repository: Arc<SeaOrmRepository>, config: AppConfig) -> Self {
        Self {
            repository,
            config,
            run_registry: Arc::new(JobRunRegistry::new()),
        }
    }

    #[must_use]
    pub const fn repository(&self) -> &Arc<SeaOrmRepository> {
        &self.repository
    }

    #[must_use]
    pub const fn config(&self) -> &AppConfig {
        &self.config
    }

    #[must_use]
    pub fn vault_path(&self) -> PathBuf {
        self.config.vault_path.clone()
    }

    #[must_use]
    pub fn run_registry(&self) -> Arc<JobRunRegistry> {
        Arc::clone(&self.run_registry)
    }
}
