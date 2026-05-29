use hoarder::{
    connectors::plugin::{
        CONNECTOR_PLUGIN_ABI_VERSION, CONNECTOR_PLUGIN_ENTRYPOINT, ConnectorPluginManifest,
        PluginSecretField,
    },
    connectors::traits::ConnectorConfig,
    core::types::ConnectorCapabilities,
};
use std::collections::BTreeMap;

#[test]
fn plugin_abi_manifest_accepts_current_version_and_required_fields() {
    let manifest = ConnectorPluginManifest {
        abi_version: CONNECTOR_PLUGIN_ABI_VERSION,
        id: "com.example.knowledge".to_owned(),
        display_name: "Example Knowledge".to_owned(),
        entrypoint_symbol: CONNECTOR_PLUGIN_ENTRYPOINT.to_owned(),
        config_schema: serde_json::json!({"type":"object"}),
        secret_fields: vec![PluginSecretField {
            name: "api_token".to_owned(),
        }],
        capabilities: ConnectorCapabilities {
            supports_files: false,
            supports_directories: false,
            supports_virtual_documents: true,
            supports_incremental_scan: true,
        },
    };

    manifest.validate().unwrap();
}

#[test]
fn plugin_abi_manifest_rejects_incompatible_version() {
    let mut manifest = minimal_manifest();
    manifest.abi_version = CONNECTOR_PLUGIN_ABI_VERSION + 1;

    let error = manifest.validate().unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unsupported connector plugin ABI")
    );
}

#[test]
fn plugin_connector_config_serializes_plugin_id_and_options() {
    let config = ConnectorConfig::Plugin {
        plugin_id: "com.example.knowledge".to_owned(),
        options: BTreeMap::from([("api_token".to_owned(), "secret".to_owned())]),
    };

    let json = serde_json::to_value(config).unwrap();

    assert_eq!(json["kind"], "plugin");
    assert_eq!(json["pluginId"], "com.example.knowledge");
    assert_eq!(json["options"]["api_token"], "secret");
}

fn minimal_manifest() -> ConnectorPluginManifest {
    ConnectorPluginManifest {
        abi_version: CONNECTOR_PLUGIN_ABI_VERSION,
        id: "com.example.plugin".to_owned(),
        display_name: "Example".to_owned(),
        entrypoint_symbol: CONNECTOR_PLUGIN_ENTRYPOINT.to_owned(),
        config_schema: serde_json::json!({"type":"object"}),
        secret_fields: Vec::new(),
        capabilities: ConnectorCapabilities::default(),
    }
}
