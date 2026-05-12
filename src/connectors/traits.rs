use std::{collections::BTreeMap, pin::Pin};

use bytes::Bytes;
use futures::{Stream, future::BoxFuture};
use serde::{Deserialize, Serialize};

use crate::{
    core::types::{ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot},
    error::AppResult,
};

pub type ConnectorFuture<'a, T> = BoxFuture<'a, AppResult<T>>;
pub type ScanStream = Pin<Box<dyn Stream<Item = AppResult<ItemSnapshot>> + Send>>;
pub type ByteStream = Pin<Box<dyn Stream<Item = AppResult<Bytes>> + Send>>;

pub trait SourceConnector: Send + Sync {
    fn kind(&self) -> ConnectorKind;

    fn validate<'a>(
        &'a self,
        config: &'a ConnectorConfig,
    ) -> ConnectorFuture<'a, ConnectorCapabilities>;

    fn scan<'a>(
        &'a self,
        config: &'a ConnectorConfig,
        cursor: Option<&'a str>,
    ) -> ConnectorFuture<'a, ScanStream>;

    fn read<'a>(
        &'a self,
        config: &'a ConnectorConfig,
        item_ref: &'a ItemRef,
    ) -> ConnectorFuture<'a, ByteStream>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConnectorConfig {
    #[serde(rename = "opendal")]
    OpenDal {
        service: String,
        #[serde(default)]
        options: BTreeMap<String, String>,
    },
}

impl ConnectorConfig {
    pub fn kind(&self) -> ConnectorKind {
        match self {
            Self::OpenDal { .. } => ConnectorKind::OpenDal,
        }
    }
}
