use chrono::{TimeZone, Utc};
use hoarder::{
    core::types::{ItemSnapshot, ItemType, SourceId},
    sync::planner::{PlanDecision, StoredItemState, SyncPlanner},
};

#[test]
fn sync_planner_syncs_new_items() {
    let snapshot = file_snapshot("docs/a.txt")
        .with_size(12)
        .with_etag("etag-1")
        .build();

    assert_eq!(SyncPlanner::plan(Some(&snapshot), None), PlanDecision::Sync);
}

#[test]
fn sync_planner_skips_when_etag_size_and_modified_at_match() {
    let modified_at = Utc.with_ymd_and_hms(2026, 5, 12, 8, 30, 0).unwrap();
    let snapshot = file_snapshot("docs/a.txt")
        .with_size(12)
        .with_etag("etag-1")
        .with_modified_at(modified_at)
        .build();
    let stored = StoredItemState {
        source_path: "docs/a.txt".to_owned(),
        item_type: ItemType::File,
        size: Some(12),
        etag: Some("etag-1".to_owned()),
        modified_at: Some(modified_at),
        content_hash: Some("sha256:old".to_owned()),
    };

    assert_eq!(
        SyncPlanner::plan(Some(&snapshot), Some(&stored)),
        PlanDecision::Skip
    );
}

#[test]
fn sync_planner_syncs_when_etag_changes_before_considering_hash() {
    let snapshot = file_snapshot("docs/a.txt")
        .with_size(12)
        .with_etag("etag-2")
        .with_content_hash("sha256:same")
        .build();
    let stored = StoredItemState {
        source_path: "docs/a.txt".to_owned(),
        item_type: ItemType::File,
        size: Some(12),
        etag: Some("etag-1".to_owned()),
        modified_at: None,
        content_hash: Some("sha256:same".to_owned()),
    };

    assert_eq!(
        SyncPlanner::plan(Some(&snapshot), Some(&stored)),
        PlanDecision::Sync
    );
}

#[test]
fn sync_planner_syncs_when_size_changes() {
    let snapshot = file_snapshot("docs/a.txt")
        .with_size(13)
        .with_etag("etag-1")
        .build();
    let stored = StoredItemState {
        source_path: "docs/a.txt".to_owned(),
        item_type: ItemType::File,
        size: Some(12),
        etag: Some("etag-1".to_owned()),
        modified_at: None,
        content_hash: None,
    };

    assert_eq!(
        SyncPlanner::plan(Some(&snapshot), Some(&stored)),
        PlanDecision::Sync
    );
}

#[test]
fn sync_planner_uses_content_hash_when_only_hashes_are_available() {
    let snapshot = file_snapshot("docs/a.txt")
        .with_content_hash("sha256:new")
        .build();
    let stored = StoredItemState {
        source_path: "docs/a.txt".to_owned(),
        item_type: ItemType::File,
        size: None,
        etag: None,
        modified_at: None,
        content_hash: Some("sha256:old".to_owned()),
    };

    assert_eq!(
        SyncPlanner::plan(Some(&snapshot), Some(&stored)),
        PlanDecision::Sync
    );
}

#[test]
fn sync_planner_marks_missing_source_items_deleted_without_local_delete() {
    let stored = StoredItemState {
        source_path: "docs/a.txt".to_owned(),
        item_type: ItemType::File,
        size: Some(12),
        etag: Some("etag-1".to_owned()),
        modified_at: None,
        content_hash: None,
    };

    assert_eq!(
        SyncPlanner::plan(None, Some(&stored)),
        PlanDecision::MarkDeleted
    );
}

struct SnapshotBuilder {
    snapshot: ItemSnapshot,
}

impl SnapshotBuilder {
    const fn with_size(mut self, size: u64) -> Self {
        self.snapshot.size = Some(size);
        self
    }

    fn with_etag(mut self, etag: &str) -> Self {
        self.snapshot.etag = Some(etag.to_owned());
        self
    }

    const fn with_modified_at(mut self, modified_at: chrono::DateTime<Utc>) -> Self {
        self.snapshot.modified_at = Some(modified_at);
        self
    }

    fn with_content_hash(mut self, content_hash: &str) -> Self {
        self.snapshot.content_hash = Some(content_hash.to_owned());
        self
    }

    fn build(self) -> ItemSnapshot {
        self.snapshot
    }
}

fn file_snapshot(source_path: &str) -> SnapshotBuilder {
    SnapshotBuilder {
        snapshot: ItemSnapshot {
            source_id: SourceId::new(),
            source_path: source_path.to_owned(),
            item_type: ItemType::File,
            size: None,
            etag: None,
            modified_at: None,
            content_hash: None,
            metadata_json: None,
        },
    }
}
