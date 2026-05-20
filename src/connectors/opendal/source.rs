use std::time::SystemTime;

use chrono::{DateTime, Utc};
use futures::{FutureExt, StreamExt};
use opendal::{Entry, EntryMode, Operator, services::Fs};

use crate::{
    connectors::{
        opendal::config::{OpenDalServiceConfig, OpenDalServiceKind, validate_connector_config},
        traits::{ByteStream, ConnectorConfig, ConnectorFuture, ScanStream, SourceConnector},
    },
    core::types::{
        ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot, ItemType, SourceId,
    },
    error::{AppError, AppResult},
};

const READ_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Clone, Debug)]
pub struct OpenDalSourceConnector {
    source_id: SourceId,
}

impl OpenDalSourceConnector {
    #[must_use]
    pub const fn new(source_id: SourceId) -> Self {
        Self { source_id }
    }

    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }
}

impl SourceConnector for OpenDalSourceConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::OpenDal
    }

    fn validate<'a>(
        &'a self,
        config: &'a ConnectorConfig,
    ) -> ConnectorFuture<'a, ConnectorCapabilities> {
        async move {
            let config = validate_connector_config(config)?;
            ensure_fs_service(&config)?;

            Ok(ConnectorCapabilities {
                supports_files: true,
                supports_directories: true,
                supports_virtual_documents: false,
                supports_incremental_scan: false,
            })
        }
        .boxed()
    }

    fn scan<'a>(
        &'a self,
        config: &'a ConnectorConfig,
        _cursor: Option<&'a str>,
    ) -> ConnectorFuture<'a, ScanStream> {
        async move {
            let config = validate_connector_config(config)?;
            let operator = build_operator(&config)?;
            let lister = operator
                .lister_with("")
                .recursive(true)
                .await
                .map_err(|error| opendal_error("list filesystem source", error))?;
            let source_id = self.source_id;

            Ok(Box::pin(lister.filter_map(move |entry| async move {
                match entry {
                    Ok(entry) => snapshot_from_entry(source_id, entry),
                    Err(error) => Some(Err(opendal_error("list filesystem source", error))),
                }
            })) as ScanStream)
        }
        .boxed()
    }

    fn read<'a>(
        &'a self,
        config: &'a ConnectorConfig,
        item_ref: &'a ItemRef,
    ) -> ConnectorFuture<'a, ByteStream> {
        async move {
            if item_ref.item_type != ItemType::File {
                return Err(AppError::Connector(format!(
                    "cannot read non-file source item `{}`",
                    item_ref.source_path
                )));
            }

            let config = validate_connector_config(config)?;
            let operator = build_operator(&config)?;
            let stream = operator
                .reader_with(&item_ref.source_path)
                .chunk(READ_CHUNK_SIZE)
                .await
                .map_err(|error| opendal_error("open filesystem source item reader", error))?
                .into_bytes_stream(..)
                .await
                .map_err(|error| opendal_error("stream filesystem source item", error))?
                .map(|chunk| chunk.map_err(|error| io_error("read filesystem source item", error)));

            Ok(Box::pin(stream) as ByteStream)
        }
        .boxed()
    }
}

impl Default for OpenDalSourceConnector {
    fn default() -> Self {
        Self::new(SourceId::new())
    }
}

fn build_operator(config: &OpenDalServiceConfig) -> AppResult<Operator> {
    match config {
        OpenDalServiceConfig::Fs { root } => Operator::new(Fs::default().root(root))
            .map(opendal::OperatorBuilder::finish)
            .map_err(|error| opendal_error("build filesystem source operator", error)),
        config => Err(AppError::Connector(format!(
            "OpenDAL service `{}` is validated but the source connector currently supports `fs` only",
            config.kind()
        ))),
    }
}

fn ensure_fs_service(config: &OpenDalServiceConfig) -> AppResult<()> {
    if config.kind() == OpenDalServiceKind::Fs {
        Ok(())
    } else {
        Err(AppError::Connector(format!(
            "OpenDAL service `{}` is validated but the source connector currently supports `fs` only",
            config.kind()
        )))
    }
}

fn snapshot_from_entry(source_id: SourceId, entry: Entry) -> Option<AppResult<ItemSnapshot>> {
    let (source_path, metadata) = entry.into_parts();
    if source_path.is_empty() {
        return None;
    }

    let item_type = match metadata.mode() {
        EntryMode::FILE => ItemType::File,
        EntryMode::DIR => ItemType::Directory,
        EntryMode::Unknown => return None,
    };

    Some(Ok(ItemSnapshot {
        source_id,
        source_path,
        item_type,
        size: (item_type == ItemType::File).then_some(metadata.content_length()),
        etag: metadata.etag().map(ToOwned::to_owned),
        modified_at: metadata.last_modified().map(|timestamp| {
            let system_time = SystemTime::from(timestamp);
            DateTime::<Utc>::from(system_time)
        }),
        content_hash: metadata.content_md5().map(ToOwned::to_owned),
        metadata_json: None,
    }))
}

#[allow(clippy::needless_pass_by_value)]
fn opendal_error(context: &str, error: opendal::Error) -> AppError {
    AppError::Connector(format!("{context}: {error}"))
}

#[allow(clippy::needless_pass_by_value)]
fn io_error(context: &str, error: std::io::Error) -> AppError {
    AppError::Connector(format!("{context}: {error}"))
}
