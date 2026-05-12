use chrono::{TimeZone, Utc};
use hoarder::core::types::{
    ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot, ItemType, SourceId, SyncStatus,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn core_types_item_type_serializes_to_stable_snake_case_values() {
    assert_eq!(serde_json::to_value(ItemType::File).unwrap(), json!("file"));
    assert_eq!(
        serde_json::to_value(ItemType::Directory).unwrap(),
        json!("directory")
    );
    assert_eq!(
        serde_json::to_value(ItemType::VirtualDocument).unwrap(),
        json!("virtual_document")
    );
}

#[test]
fn core_types_core_enums_serialize_for_api_payloads() {
    assert_eq!(
        serde_json::to_value(SyncStatus::DeletedOnSource).unwrap(),
        json!("deleted_on_source")
    );
    assert_eq!(
        serde_json::to_value(ConnectorKind::OpenDal).unwrap(),
        json!("opendal")
    );
}

#[test]
fn core_types_item_snapshot_serializes_with_camel_case_fields() {
    let source_id =
        SourceId::from_uuid(Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap());
    let snapshot = ItemSnapshot {
        source_id,
        source_path: "notes/today.md".to_owned(),
        item_type: ItemType::VirtualDocument,
        size: Some(42),
        etag: Some("etag-1".to_owned()),
        modified_at: Some(Utc.with_ymd_and_hms(2026, 5, 12, 8, 0, 0).unwrap()),
        content_hash: Some("sha256:abc123".to_owned()),
        metadata_json: Some(json!({ "title": "Today" })),
    };

    let encoded = serde_json::to_value(snapshot).unwrap();

    assert_eq!(encoded["sourceId"], json!(source_id));
    assert_eq!(encoded["sourcePath"], json!("notes/today.md"));
    assert_eq!(encoded["itemType"], json!("virtual_document"));
    assert_eq!(encoded["metadataJson"]["title"], json!("Today"));
}

#[test]
fn core_types_item_ref_and_capabilities_are_plain_contract_data() {
    let source_id =
        SourceId::from_uuid(Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap());
    let item_ref = ItemRef {
        source_id,
        source_path: "archive/report.pdf".to_owned(),
        item_type: ItemType::File,
    };
    let capabilities = ConnectorCapabilities {
        supports_files: true,
        supports_directories: true,
        supports_virtual_documents: false,
        supports_incremental_scan: false,
    };

    assert_eq!(item_ref.source_id, source_id);
    assert!(capabilities.supports_files);
    assert!(capabilities.supports_directories);
    assert!(!capabilities.supports_virtual_documents);
}
