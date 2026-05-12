use std::collections::BTreeMap;

use hoarder::connectors::{
    opendal::config::{
        OpenDalServiceConfig, OpenDalServiceKind, REDACTED_SECRET, redacted_connector_config,
        validate_connector_config,
    },
    traits::ConnectorConfig,
};

#[test]
fn opendal_config_validates_required_fs_root() {
    let config = ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::from([("root".to_owned(), "/tmp/hoarder-source".to_owned())]),
    };

    let validated = validate_connector_config(&config).unwrap();

    assert_eq!(
        validated,
        OpenDalServiceConfig::Fs {
            root: "/tmp/hoarder-source".to_owned()
        }
    );
}

#[test]
fn opendal_config_rejects_missing_required_fields() {
    let cases = [
        ("fs", BTreeMap::new(), "root"),
        ("webdav", BTreeMap::new(), "endpoint"),
        (
            "sftp",
            BTreeMap::from([("endpoint".to_owned(), "ssh://example.test".to_owned())]),
            "username",
        ),
        (
            "s3",
            BTreeMap::from([
                ("bucket".to_owned(), "archive".to_owned()),
                ("region".to_owned(), "us-east-1".to_owned()),
                ("access_key_id".to_owned(), "access".to_owned()),
            ]),
            "secret_access_key",
        ),
    ];

    for (service, options, missing_field) in cases {
        let config = ConnectorConfig::OpenDal {
            service: service.to_owned(),
            options,
        };

        let error = validate_connector_config(&config).unwrap_err();

        assert!(
            error.to_string().contains(missing_field),
            "{service} error should mention missing {missing_field}: {error}"
        );
    }
}

#[test]
fn opendal_config_redacts_sensitive_options_without_validating() {
    let config = ConnectorConfig::OpenDal {
        service: "s3".to_owned(),
        options: BTreeMap::from([
            ("bucket".to_owned(), "archive".to_owned()),
            ("access_key_id".to_owned(), "visible-access-key".to_owned()),
            (
                "secret_access_key".to_owned(),
                "visible-secret-key".to_owned(),
            ),
            (
                "session_token".to_owned(),
                "visible-session-token".to_owned(),
            ),
            ("root".to_owned(), "/docs".to_owned()),
        ]),
    };

    let redacted = redacted_connector_config(&config);

    let ConnectorConfig::OpenDal { options, .. } = redacted;
    assert_eq!(options["bucket"], "archive");
    assert_eq!(options["root"], "/docs");
    assert_eq!(options["access_key_id"], REDACTED_SECRET);
    assert_eq!(options["secret_access_key"], REDACTED_SECRET);
    assert_eq!(options["session_token"], REDACTED_SECRET);
}

#[test]
fn opendal_config_serializes_service_variants_for_json_storage() {
    let config = OpenDalServiceConfig::WebDav {
        endpoint: "https://dav.example.test".to_owned(),
        root: Some("/remote/docs".to_owned()),
        username: Some("ada".to_owned()),
        password: Some("correct-horse".to_owned()),
        token: None,
    };

    let value = serde_json::to_value(&config).unwrap();
    let restored: OpenDalServiceConfig = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(value["service"], "webdav");
    assert_eq!(restored, config);
    assert_eq!(OpenDalServiceKind::WebDav.as_str(), "webdav");
}
