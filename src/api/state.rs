use std::{path::PathBuf, sync::Arc};

use crate::{AppConfig, db::repository::SeaOrmRepository};

#[derive(Clone)]
pub struct ApiState {
    repository: Arc<SeaOrmRepository>,
    config: AppConfig,
}

impl ApiState {
    #[must_use]
    pub const fn new(repository: Arc<SeaOrmRepository>, config: AppConfig) -> Self {
        Self { repository, config }
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
}
