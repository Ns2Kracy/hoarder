use chrono::{TimeZone, Utc};
use hoarder::core::types::{
    ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot, ItemType, JobScheduleKind,
    JobStatus, RunStatus, SourceId, SyncStatus,
};
use serde_json::json;

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
    assert_eq!(
        serde_json::to_value(JobScheduleKind::Interval).unwrap(),
        json!("interval")
    );
    assert_eq!(
        serde_json::to_value(JobStatus::Idle).unwrap(),
        json!("idle")
    );
    assert_eq!(
        serde_json::to_value(RunStatus::CompletedWithFailures).unwrap(),
        json!("completed_with_failures")
    );
}

#[test]
fn core_types_local_ids_serialize_parse_and_display_as_numbers() {
    let source_id = SourceId::from_i64(42);

    assert_eq!(serde_json::to_value(source_id).unwrap(), json!(42));
    assert_eq!(
        serde_json::from_value::<SourceId>(json!(42)).unwrap(),
        source_id
    );
    assert_eq!("42".parse::<SourceId>().unwrap(), source_id);
    assert_eq!(source_id.to_string(), "42");
    assert!("0".parse::<SourceId>().is_err());
    assert!("-1".parse::<SourceId>().is_err());
    assert!(serde_json::from_value::<SourceId>(json!(0)).is_err());
    assert!(serde_json::from_value::<SourceId>(json!(-1)).is_err());
    assert!(serde_json::from_value::<SourceId>(json!("42")).is_err());
    assert!(serde_json::from_value::<SourceId>(json!(42.5)).is_err());
}

#[test]
fn core_types_item_snapshot_serializes_with_camel_case_fields() {
    let source_id = SourceId::from_i64(42);
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

    assert_eq!(encoded["sourceId"], json!(42));
    assert_eq!(encoded["sourcePath"], json!("notes/today.md"));
    assert_eq!(encoded["itemType"], json!("virtual_document"));
    assert_eq!(encoded["metadataJson"]["title"], json!("Today"));
}

#[test]
fn core_types_item_ref_and_capabilities_are_plain_contract_data() {
    let source_id = SourceId::from_i64(42);
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
