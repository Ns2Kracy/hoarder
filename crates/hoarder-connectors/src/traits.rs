use std::{collections::BTreeMap, pin::Pin};

use bytes::Bytes;
use futures::{Stream, future::BoxFuture};
use serde::{Deserialize, Serialize};

use hoarder_core::{
    AppResult,
    types::{ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot},
};

pub type ConnectorFuture<'a, T> = BoxFuture<'a, AppResult<T>>;
pub type ScanStream = Pin<Box<dyn Stream<Item = AppResult<ItemSnapshot>> + Send>>;
pub type ByteStream = Pin<Box<dyn Stream<Item = AppResult<Bytes>> + Send>>;

pub struct ScanOutcome {
    pub items: ScanStream,
    pub next_cursor: Option<String>,
}

impl ScanOutcome {
    #[must_use]
    pub fn new(items: ScanStream, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }
}

/// Read-only source connector used by the source-to-vault sync engine.
///
/// Connectors can validate configuration, scan source metadata, and read source
/// content. They intentionally do not expose source mutation methods.
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
    ) -> ConnectorFuture<'a, ScanOutcome>;

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

impl ConnectorConfig {
    #[must_use]
    pub const fn kind(&self) -> ConnectorKind {
        match self {
            Self::OpenDal { .. } => ConnectorKind::OpenDal,
            Self::Notion { .. } => ConnectorKind::Notion,
            Self::Feishu { .. } => ConnectorKind::Feishu,
            Self::Plugin { .. } => ConnectorKind::Plugin,
        }
    }
}
