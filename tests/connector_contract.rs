use std::{collections::BTreeMap, sync::Arc};

use bytes::Bytes;
use futures::{FutureExt, StreamExt, stream};
use hoarder::{
    connectors::{
        registry::ConnectorRegistry,
        traits::{ByteStream, ConnectorConfig, ConnectorFuture, ScanStream, SourceConnector},
    },
    core::types::{
        ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot, ItemType, SourceId,
    },
};
use uuid::Uuid;

#[derive(Debug)]
struct FakeConnector {
    source_id: SourceId,
}

impl FakeConnector {
    fn new() -> Self {
        Self {
            source_id: SourceId::from_uuid(
                Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap(),
            ),
        }
    }

    fn snapshot(&self) -> ItemSnapshot {
        ItemSnapshot {
            source_id: self.source_id,
            source_path: "docs/readme.md".to_owned(),
            item_type: ItemType::File,
            size: Some(5),
            etag: Some("fake-etag".to_owned()),
            modified_at: None,
            content_hash: None,
            metadata_json: None,
        }
    }
}

impl SourceConnector for FakeConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::OpenDal
    }

    fn validate<'a>(
        &'a self,
        config: &'a ConnectorConfig,
    ) -> ConnectorFuture<'a, ConnectorCapabilities> {
        async move {
            assert_eq!(config.kind(), ConnectorKind::OpenDal);
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
        _config: &'a ConnectorConfig,
        _cursor: Option<&'a str>,
    ) -> ConnectorFuture<'a, ScanStream> {
        async move {
            let snapshot = self.snapshot();
            Ok(Box::pin(stream::iter([Ok(snapshot)])) as ScanStream)
        }
        .boxed()
    }

    fn read<'a>(
        &'a self,
        _config: &'a ConnectorConfig,
        item_ref: &'a ItemRef,
    ) -> ConnectorFuture<'a, ByteStream> {
        async move {
            assert_eq!(item_ref.source_id, self.source_id);
            Ok(Box::pin(stream::iter([Ok(Bytes::from_static(b"hello"))])) as ByteStream)
        }
        .boxed()
    }
}

#[tokio::test]
async fn connector_contract_trait_returns_capabilities_snapshots_and_bytes() {
    let connector = FakeConnector::new();
    let config = opendal_config();

    let capabilities = connector.validate(&config).await.unwrap();
    assert!(capabilities.supports_files);
    assert!(capabilities.supports_directories);

    let mut scan = connector.scan(&config, None).await.unwrap();
    let snapshot = scan.next().await.unwrap().unwrap();
    assert_eq!(snapshot.source_path, "docs/readme.md");

    let item_ref = snapshot.item_ref();
    let mut bytes = connector.read(&config, &item_ref).await.unwrap();
    assert_eq!(
        bytes.next().await.unwrap().unwrap(),
        Bytes::from_static(b"hello")
    );
    assert!(bytes.next().await.is_none());
}

#[test]
fn connector_contract_registry_finds_registered_connector() {
    let mut registry = ConnectorRegistry::default();
    registry.register_factory(
        ConnectorKind::OpenDal,
        Arc::new(|| Arc::new(FakeConnector::new())),
    );

    let connector = registry.create(&ConnectorKind::OpenDal).unwrap();

    assert_eq!(connector.kind(), ConnectorKind::OpenDal);
}

#[test]
fn connector_contract_registry_reports_missing_connector() {
    let registry = ConnectorRegistry::default();

    let error = match registry.create(&ConnectorKind::Notion) {
        Ok(_) => panic!("missing connector should not resolve"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("connector factory"));
}

fn opendal_config() -> ConnectorConfig {
    ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::from([("root".to_owned(), ".".to_owned())]),
    }
}
