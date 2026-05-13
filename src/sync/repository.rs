use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;

use crate::{
    AppResult,
    core::types::{ItemType, JobId, RunId, SourceId, SyncStatus},
};

use super::{
    engine::{SyncJob, SyncRunStatus, SyncRunSummary},
    planner::StoredItemState,
};

pub type RepositoryFuture<'a, T> = BoxFuture<'a, AppResult<T>>;

pub trait SyncRepository: Send + Sync {
    fn load_job(&self, job_id: JobId) -> RepositoryFuture<'_, SyncJob>;

    fn start_run<'a>(&'a self, job: &'a SyncJob) -> RepositoryFuture<'a, RunId>;

    fn item_state<'a>(
        &'a self,
        source_id: SourceId,
        source_path: &'a str,
    ) -> RepositoryFuture<'a, Option<StoredItemState>>;

    fn known_item_states(&self, source_id: SourceId) -> RepositoryFuture<'_, Vec<StoredItemState>>;

    fn record_item_outcome(
        &self,
        run_id: RunId,
        outcome: ItemSyncOutcome,
    ) -> RepositoryFuture<'_, ()>;

    fn mark_deleted<'a>(
        &'a self,
        run_id: RunId,
        source_id: SourceId,
        source_path: &'a str,
    ) -> RepositoryFuture<'a, ()>;

    fn finish_run(
        &self,
        run_id: RunId,
        status: SyncRunStatus,
        summary: SyncRunSummary,
    ) -> RepositoryFuture<'_, ()>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemSyncOutcome {
    pub source_id: SourceId,
    pub source_path: String,
    pub item_type: ItemType,
    pub status: SyncStatus,
    pub target_path: Option<Utf8PathBuf>,
    pub size: Option<u64>,
    pub etag: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
    pub bytes_written: u64,
    pub error_message: Option<String>,
}
