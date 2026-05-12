use chrono::{DateTime, Utc};

use crate::core::types::{ItemSnapshot, ItemType};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanDecision {
    Sync,
    Skip,
    MarkDeleted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredItemState {
    pub source_path: String,
    pub item_type: ItemType,
    pub size: Option<u64>,
    pub etag: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Default)]
pub struct SyncPlanner;

impl SyncPlanner {
    pub fn plan(source: Option<&ItemSnapshot>, stored: Option<&StoredItemState>) -> PlanDecision {
        match (source, stored) {
            (Some(_), None) => PlanDecision::Sync,
            (None, Some(_)) => PlanDecision::MarkDeleted,
            (None, None) => PlanDecision::Skip,
            (Some(source), Some(stored)) => plan_existing(source, stored),
        }
    }
}

fn plan_existing(source: &ItemSnapshot, stored: &StoredItemState) -> PlanDecision {
    if source.item_type != stored.item_type {
        return PlanDecision::Sync;
    }

    if let (Some(source_etag), Some(stored_etag)) = (&source.etag, &stored.etag) {
        if source_etag != stored_etag {
            return PlanDecision::Sync;
        }

        if changed(source.size, stored.size) || changed(source.modified_at, stored.modified_at) {
            return PlanDecision::Sync;
        }

        return PlanDecision::Skip;
    }

    if changed(source.modified_at, stored.modified_at) || changed(source.size, stored.size) {
        return PlanDecision::Sync;
    }

    if matched(source.modified_at, stored.modified_at) && matched(source.size, stored.size) {
        return PlanDecision::Skip;
    }

    if let (Some(source_hash), Some(stored_hash)) = (&source.content_hash, &stored.content_hash) {
        if source_hash == stored_hash {
            PlanDecision::Skip
        } else {
            PlanDecision::Sync
        }
    } else {
        PlanDecision::Sync
    }
}

fn changed<T: PartialEq>(left: Option<T>, right: Option<T>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left != right)
}

fn matched<T: PartialEq>(left: Option<T>, right: Option<T>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left == right)
}
