use std::{collections::HashMap, sync::Arc};

use crate::{
    core::types::ConnectorKind,
    error::{AppError, AppResult},
};

use super::traits::SourceConnector;

pub type ConnectorFactory = Arc<dyn Fn() -> Arc<dyn SourceConnector> + Send + Sync>;

#[derive(Clone, Default)]
pub struct ConnectorRegistry {
    factories: HashMap<ConnectorKind, ConnectorFactory>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_factory(
        &mut self,
        kind: ConnectorKind,
        factory: ConnectorFactory,
    ) -> Option<ConnectorFactory> {
        self.factories.insert(kind, factory)
    }

    pub fn get_factory(&self, kind: &ConnectorKind) -> Option<ConnectorFactory> {
        self.factories.get(kind).cloned()
    }

    pub fn create(&self, kind: &ConnectorKind) -> AppResult<Arc<dyn SourceConnector>> {
        let factory = self.get_factory(kind).ok_or_else(|| {
            AppError::NotFound(format!("connector factory not registered for {kind:?}"))
        })?;

        Ok(factory())
    }
}
