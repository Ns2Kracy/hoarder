use camino::Utf8PathBuf;
use hoarder::core::{
    types::SourceId,
    vault_path::{normalize_source_path, target_path},
};
use uuid::Uuid;

#[test]
fn vault_path_normalizes_valid_nested_paths() {
    assert_eq!(
        normalize_source_path("folder/./nested//file.txt").unwrap(),
        "folder/nested/file.txt"
    );
    assert_eq!(
        normalize_source_path("folder\\nested\\file.txt").unwrap(),
        "folder/nested/file.txt"
    );
}

#[test]
fn vault_path_rejects_absolute_and_traversal_paths() {
    for invalid in [
        "",
        ".",
        "/etc/passwd",
        "\\windows\\system32",
        "../escape",
        "folder/../../escape",
        "folder/..",
        "C:\\Users\\alice\\escape.txt",
        "C:/Users/alice/escape.txt",
        "\\\\server\\share\\escape.txt",
    ] {
        assert!(
            normalize_source_path(invalid).is_err(),
            "{invalid:?} should be rejected"
        );
    }
}

#[test]
fn vault_path_rejects_hoarder_root_paths() {
    for invalid in [".hoarder", ".hoarder/config.json", ".hoarder/tmp/file"] {
        assert!(
            normalize_source_path(invalid).is_err(),
            "{invalid:?} should be rejected"
        );
    }
}

#[test]
fn vault_path_target_path_places_items_under_source_directory() {
    let source_id =
        SourceId::from_uuid(Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap());
    let vault_root = Utf8PathBuf::from("/tmp/hoarder-vault");

    let target = target_path(&vault_root, &source_id, "folder/report.pdf").unwrap();

    assert_eq!(
        target,
        Utf8PathBuf::from(format!("/tmp/hoarder-vault/{source_id}/folder/report.pdf"))
    );
}

#[test]
fn vault_path_target_path_revalidates_normalized_input() {
    let source_id =
        SourceId::from_uuid(Uuid::parse_str("018f3f55-6b4d-7b2f-8b1e-f7563f31b8d5").unwrap());

    let vault_root = Utf8PathBuf::from("./vault");

    assert!(target_path(&vault_root, &source_id, "../escape").is_err());
    assert!(target_path(&vault_root, &source_id, ".hoarder/state.db").is_err());
}
