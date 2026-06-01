use std::{io::ErrorKind, time::SystemTime};

use chrono::{DateTime, Utc};
use futures::{FutureExt, StreamExt};
use opendal::{
    Entry, EntryMode, Operator,
    services::{Fs, S3, Sftp, Webdav},
};

use crate::{
    connectors::{
        opendal::config::{OpenDalServiceConfig, validate_connector_config},
        traits::{
            ByteStream, ConnectorConfig, ConnectorFuture, ScanOutcome, ScanStream, SourceConnector,
        },
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
            if !matches!(config, OpenDalServiceConfig::Fs { .. }) {
                build_operator(&config)?;
            }

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
    ) -> ConnectorFuture<'a, ScanOutcome> {
        async move {
            let config = validate_connector_config(config)?;
            let operator = build_operator(&config)?;
            let lister = operator
                .lister_with("")
                .recursive(true)
                .await
                .map_err(|error| opendal_error("list OpenDAL source", error))?;
            let source_id = self.source_id;

            let items: ScanStream = Box::pin(lister.filter_map(move |entry| async move {
                match entry {
                    Ok(entry) => snapshot_from_entry(source_id, entry),
                    Err(error) => Some(Err(opendal_error("list OpenDAL source", error))),
                }
            }));

            Ok(ScanOutcome::new(items, None))
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
                .map_err(|error| opendal_error("open OpenDAL source item reader", error))?
                .into_bytes_stream(..)
                .await
                .map_err(|error| opendal_error("stream OpenDAL source item", error))?
                .map(|chunk| chunk.map_err(|error| io_error("read OpenDAL source item", error)));

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
        OpenDalServiceConfig::WebDav {
            endpoint,
            root,
            username,
            password,
            token,
        } => {
            let mut builder = Webdav::default().endpoint(endpoint);
            if let Some(root) = root {
                builder = builder.root(root);
            }
            if let Some(username) = username {
                builder = builder.username(username);
            }
            if let Some(password) = password {
                builder = builder.password(password);
            }
            if let Some(token) = token {
                builder = builder.token(token);
            }

            Operator::new(builder)
                .map(opendal::OperatorBuilder::finish)
                .map_err(|error| opendal_error("build WebDAV source operator", error))
        }
        OpenDalServiceConfig::Sftp {
            endpoint,
            username,
            root,
            password,
            private_key,
        } => {
            if password.is_some() {
                return Err(AppError::Connector(
                    "OpenDAL sftp does not support password login; use an SSH key file or ssh-agent"
                        .to_owned(),
                ));
            }

            let mut builder = Sftp::default().endpoint(endpoint).user(username);
            if let Some(root) = root {
                builder = builder.root(root);
            }
            if let Some(private_key) = private_key {
                builder = builder.key(private_key);
            }

            Operator::new(builder)
                .map(opendal::OperatorBuilder::finish)
                .map_err(|error| opendal_error("build SFTP source operator", error))
        }
        OpenDalServiceConfig::S3 {
            bucket,
            region,
            access_key_id,
            secret_access_key,
            endpoint,
            root,
            session_token,
        } => {
            let mut builder = S3::default()
                .bucket(bucket)
                .region(region)
                .access_key_id(access_key_id)
                .secret_access_key(secret_access_key)
                .disable_config_load();
            if let Some(endpoint) = endpoint {
                builder = builder.endpoint(endpoint);
            }
            if let Some(root) = root {
                builder = builder.root(root);
            }
            if let Some(session_token) = session_token {
                builder = builder.session_token(session_token);
            }

            Operator::new(builder)
                .map(opendal::OperatorBuilder::finish)
                .map_err(|error| opendal_error("build S3 source operator", error))
        }
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
    connector_error(
        context,
        &error,
        error.is_temporary() || error.kind() == opendal::ErrorKind::RateLimited,
    )
}

#[allow(clippy::needless_pass_by_value)]
fn io_error(context: &str, error: std::io::Error) -> AppError {
    connector_error(
        context,
        &error,
        matches!(
            error.kind(),
            ErrorKind::Interrupted
                | ErrorKind::TimedOut
                | ErrorKind::WouldBlock
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::BrokenPipe
        ),
    )
}

fn connector_error(context: &str, error: &dyn std::fmt::Display, transient: bool) -> AppError {
    let message = format!("{context}: {error}");
    if transient {
        AppError::ConnectorTransient(message)
    } else {
        AppError::Connector(message)
    }
}
