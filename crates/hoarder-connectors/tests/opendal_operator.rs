use std::{collections::BTreeMap, fs};

use hoarder_connectors::{
    opendal::source::OpenDalSourceConnector,
    traits::{ConnectorConfig, SourceConnector},
};
use hoarder_core::types::SourceId;
use uuid::Uuid;

#[tokio::test]
async fn opendal_connector_validates_remote_service_operators() {
    let connector = OpenDalSourceConnector::new(SourceId::from_i64(42));
    let cases = [
        ConnectorConfig::OpenDal {
            service: "webdav".to_owned(),
            options: BTreeMap::from([
                (
                    "endpoint".to_owned(),
                    "https://dav.example.test/remote.php/dav/files/ada".to_owned(),
                ),
                ("root".to_owned(), "/docs".to_owned()),
                ("username".to_owned(), "ada".to_owned()),
                ("password".to_owned(), "correct-horse".to_owned()),
            ]),
        },
        ConnectorConfig::OpenDal {
            service: "sftp".to_owned(),
            options: BTreeMap::from([
                ("endpoint".to_owned(), "ssh://example.test:22".to_owned()),
                ("username".to_owned(), "ada".to_owned()),
                ("root".to_owned(), "/srv/docs".to_owned()),
            ]),
        },
        ConnectorConfig::OpenDal {
            service: "s3".to_owned(),
            options: BTreeMap::from([
                ("bucket".to_owned(), "archive".to_owned()),
                ("region".to_owned(), "us-east-1".to_owned()),
                ("access_key_id".to_owned(), "access".to_owned()),
                ("secret_access_key".to_owned(), "secret".to_owned()),
                ("endpoint".to_owned(), "https://s3.example.test".to_owned()),
                ("root".to_owned(), "docs".to_owned()),
                ("session_token".to_owned(), "session".to_owned()),
            ]),
        },
    ];

    for config in cases {
        let capabilities = connector.validate(&config).await.unwrap();

        assert!(capabilities.supports_files);
        assert!(capabilities.supports_directories);
        assert!(!capabilities.supports_virtual_documents);
        assert!(!capabilities.supports_incremental_scan);
    }
}

#[tokio::test]
async fn opendal_connector_validate_does_not_create_missing_fs_root() {
    let root = std::env::temp_dir().join(format!("hoarder-missing-root-{}", Uuid::new_v4()));
    assert!(!root.exists());

    let connector = OpenDalSourceConnector::new(SourceId::from_i64(42));
    let config = ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::from([("root".to_owned(), root.to_string_lossy().into_owned())]),
    };

    let capabilities = connector.validate(&config).await.unwrap();

    assert!(capabilities.supports_files);
    assert!(!root.exists());

    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn opendal_connector_rejects_sftp_password_login() {
    let connector = OpenDalSourceConnector::new(SourceId::from_i64(42));
    let config = ConnectorConfig::OpenDal {
        service: "sftp".to_owned(),
        options: BTreeMap::from([
            ("endpoint".to_owned(), "ssh://example.test:22".to_owned()),
            ("username".to_owned(), "ada".to_owned()),
            ("password".to_owned(), "correct-horse".to_owned()),
        ]),
    };

    let error = connector.validate(&config).await.unwrap_err();

    assert!(
        error
            .to_string()
            .contains("does not support password login"),
        "SFTP password login error should explain OpenDAL's limitation: {error}"
    );
}
