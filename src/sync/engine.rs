use std::{collections::BTreeSet, num::NonZeroUsize, path::PathBuf, sync::Arc};

use futures::{StreamExt, stream::FuturesUnordered};

use crate::{
    AppError, AppResult,
    connectors::traits::{ConnectorConfig, SourceConnector},
    core::types::{ConnectorKind, ItemSnapshot, ItemType, JobId, RunId, SourceId, SyncStatus},
};

use super::{
    planner::{PlanDecision, SyncPlanner},
    repository::{ItemSyncOutcome, SyncRepository},
    vault_writer::VaultWriter,
};

pub type ConnectorResolver =
    Arc<dyn Fn(ConnectorKind) -> AppResult<Arc<dyn SourceConnector>> + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncJob {
    pub id: JobId,
    pub source_id: SourceId,
    pub connector_kind: ConnectorKind,
    pub connector_config: ConnectorConfig,
    pub scan_cursor: Option<String>,
}

#[derive(Clone)]
pub struct SyncEngine<R>
where
    R: SyncRepository,
{
    repository: Arc<R>,
    connector_resolver: ConnectorResolver,
    vault_writer: VaultWriter,
    options: SyncEngineOptions,
}

impl<R> SyncEngine<R>
where
    R: SyncRepository,
{
    pub fn new(
        repository: Arc<R>,
        connector_resolver: ConnectorResolver,
        vault_writer: VaultWriter,
    ) -> Self {
        Self::with_options(
            repository,
            connector_resolver,
            vault_writer,
            SyncEngineOptions::default(),
        )
    }

    #[must_use]
    pub fn with_options(
        repository: Arc<R>,
        connector_resolver: ConnectorResolver,
        vault_writer: VaultWriter,
        options: SyncEngineOptions,
    ) -> Self {
        Self {
            repository,
            connector_resolver,
            vault_writer,
            options,
        }
    }

    /// Runs one sync job and records its final summary.
    ///
    /// # Errors
    ///
    /// Returns an error when loading the job, resolving the connector, scanning
    /// the source, writing the vault, or recording run state fails.
    pub async fn run_job(&self, job_id: JobId) -> AppResult<SyncRunSummary> {
        let job = self.repository.load_job(job_id).await?;
        let run_id = self.repository.start_run(&job).await?;
        let summary = self.run_started_job(run_id, &job).await;

        match summary {
            Ok(summary) => {
                let status = if summary.failed > 0 {
                    SyncRunStatus::CompletedWithFailures
                } else {
                    SyncRunStatus::Completed
                };
                self.repository
                    .finish_run(run_id, status, summary.clone())
                    .await?;
                Ok(summary)
            }
            Err(error) => {
                self.repository
                    .finish_run(run_id, SyncRunStatus::Failed, error.summary)
                    .await?;
                Err(error.source)
            }
        }
    }

    async fn run_started_job(&self, run_id: RunId, job: &SyncJob) -> SyncRunResult {
        let mut summary = SyncRunSummary {
            run_id,
            processed: 0,
            synced: 0,
            skipped: 0,
            failed: 0,
            bytes_written: 0,
        };
        let connector = (self.connector_resolver)(job.connector_kind)
            .map_err(|source| SyncRunError::new(source, summary.clone()))?;
        let cursor = job.scan_cursor.as_deref();
        let mut snapshots = connector
            .scan(&job.connector_config, cursor)
            .await
            .map_err(|source| SyncRunError::new(source, summary.clone()))?;
        let mut pending = FuturesUnordered::new();
        let mut seen_paths = BTreeSet::new();

        let mut scan_error = None;
        while let Some(snapshot_result) = snapshots.next().await {
            let snapshot = match snapshot_result {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    scan_error = Some(error);
                    break;
                }
            };
            summary.processed += 1;
            seen_paths.insert(snapshot.source_path.clone());
            pending.push(self.process_snapshot(
                run_id,
                &job.connector_config,
                connector.as_ref(),
                snapshot,
            ));

            if pending.len() >= self.options.file_concurrency() {
                let Some(result) = pending.next().await else {
                    continue;
                };
                self.apply_item_process_result(run_id, result, &mut summary)
                    .await
                    .map_err(|source| SyncRunError::new(source, summary.clone()))?;
            }
        }

        while let Some(result) = pending.next().await {
            self.apply_item_process_result(run_id, result, &mut summary)
                .await
                .map_err(|source| SyncRunError::new(source, summary.clone()))?;
        }

        if let Some(error) = scan_error {
            return Err(SyncRunError::new(error, summary));
        }

        for stored in self
            .repository
            .known_item_states(job.source_id)
            .await
            .map_err(|source| SyncRunError::new(source, summary.clone()))?
        {
            if !seen_paths.contains(&stored.source_path)
                && SyncPlanner::plan(None, Some(&stored)) == PlanDecision::MarkDeleted
            {
                self.repository
                    .mark_deleted(run_id, job.source_id, &stored.source_path)
                    .await
                    .map_err(|source| SyncRunError::new(source, summary.clone()))?;
            }
        }

        Ok(summary)
    }

    async fn apply_item_process_result(
        &self,
        run_id: RunId,
        result: Result<ItemProcess, ItemFailure>,
        summary: &mut SyncRunSummary,
    ) -> AppResult<()> {
        match result {
            Ok(ItemProcess::Synced { bytes_written }) => {
                summary.synced += 1;
                summary.bytes_written += bytes_written;
            }
            Ok(ItemProcess::Skipped) => {
                summary.skipped += 1;
            }
            Err(error) => {
                summary.failed += 1;
                let message = error.to_string();
                self.repository
                    .record_item_outcome(run_id, failed_item_outcome(error.snapshot, message))
                    .await?;
            }
        }

        Ok(())
    }

    async fn process_snapshot(
        &self,
        run_id: RunId,
        connector_config: &ConnectorConfig,
        connector: &dyn SourceConnector,
        snapshot: ItemSnapshot,
    ) -> Result<ItemProcess, ItemFailure> {
        let stored = self
            .repository
            .item_state(snapshot.source_id, &snapshot.source_path)
            .await
            .map_err(|source| ItemFailure::new(snapshot.clone(), source))?;

        match SyncPlanner::plan(Some(&snapshot), stored.as_ref()) {
            PlanDecision::Skip => {
                self.repository
                    .record_item_outcome(run_id, skipped_item_outcome(snapshot.clone()))
                    .await
                    .map_err(|source| ItemFailure::new(snapshot, source))?;
                Ok(ItemProcess::Skipped)
            }
            PlanDecision::Sync => {
                let outcome = self
                    .sync_snapshot(run_id, connector_config, connector, snapshot)
                    .await?;
                Ok(ItemProcess::Synced {
                    bytes_written: outcome,
                })
            }
            PlanDecision::MarkDeleted => Ok(ItemProcess::Skipped),
        }
    }

    async fn sync_snapshot(
        &self,
        run_id: RunId,
        connector_config: &ConnectorConfig,
        connector: &dyn SourceConnector,
        snapshot: ItemSnapshot,
    ) -> Result<u64, ItemFailure> {
        if snapshot.item_type == ItemType::Directory {
            self.repository
                .record_item_outcome(run_id, synced_directory_outcome(snapshot.clone()))
                .await
                .map_err(|source| ItemFailure::new(snapshot, source))?;
            return Ok(0);
        }

        let item_ref = snapshot.item_ref();
        let bytes = connector
            .read(connector_config, &item_ref)
            .await
            .map_err(|source| ItemFailure::new(snapshot.clone(), source))?;
        let write = self
            .vault_writer
            .write(&item_ref, bytes)
            .await
            .map_err(|source| ItemFailure::new(snapshot.clone(), source))?;
        let bytes_written = write.bytes_written;

        self.repository
            .record_item_outcome(
                run_id,
                synced_file_outcome(
                    snapshot.clone(),
                    write.target_path,
                    write.content_hash,
                    bytes_written,
                ),
            )
            .await
            .map_err(|source| ItemFailure::new(snapshot, source))?;

        Ok(bytes_written)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncEngineOptions {
    file_concurrency: NonZeroUsize,
}

impl SyncEngineOptions {
    #[must_use]
    pub fn new(file_concurrency: usize) -> Self {
        Self {
            file_concurrency: NonZeroUsize::new(file_concurrency).unwrap_or(NonZeroUsize::MIN),
        }
    }

    #[must_use]
    pub const fn file_concurrency(self) -> usize {
        self.file_concurrency.get()
    }
}

impl Default for SyncEngineOptions {
    fn default() -> Self {
        Self {
            file_concurrency: NonZeroUsize::MIN,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ItemProcess {
    Synced { bytes_written: u64 },
    Skipped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncRunSummary {
    pub run_id: RunId,
    pub processed: u64,
    pub synced: u64,
    pub skipped: u64,
    pub failed: u64,
    pub bytes_written: u64,
}

type SyncRunResult = Result<SyncRunSummary, SyncRunError>;

#[derive(Debug)]
struct SyncRunError {
    source: AppError,
    summary: SyncRunSummary,
}

impl SyncRunError {
    const fn new(source: AppError, summary: SyncRunSummary) -> Self {
        Self { source, summary }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncRunStatus {
    Completed,
    CompletedWithFailures,
    Failed,
}

#[derive(Debug)]
struct ItemFailure {
    snapshot: ItemSnapshot,
    source: AppError,
}

impl ItemFailure {
    const fn new(snapshot: ItemSnapshot, source: AppError) -> Self {
        Self { snapshot, source }
    }
}

impl std::fmt::Display for ItemFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for ItemFailure {}

fn synced_file_outcome(
    snapshot: ItemSnapshot,
    target_path: PathBuf,
    content_hash: String,
    bytes_written: u64,
) -> ItemSyncOutcome {
    ItemSyncOutcome {
        source_id: snapshot.source_id,
        source_path: snapshot.source_path,
        item_type: snapshot.item_type,
        status: SyncStatus::Synced,
        target_path: Some(target_path),
        size: snapshot.size,
        etag: snapshot.etag,
        modified_at: snapshot.modified_at,
        content_hash: Some(content_hash),
        bytes_written,
        error_message: None,
    }
}

fn synced_directory_outcome(snapshot: ItemSnapshot) -> ItemSyncOutcome {
    ItemSyncOutcome {
        source_id: snapshot.source_id,
        source_path: snapshot.source_path,
        item_type: snapshot.item_type,
        status: SyncStatus::Synced,
        target_path: None,
        size: snapshot.size,
        etag: snapshot.etag,
        modified_at: snapshot.modified_at,
        content_hash: snapshot.content_hash,
        bytes_written: 0,
        error_message: None,
    }
}

fn skipped_item_outcome(snapshot: ItemSnapshot) -> ItemSyncOutcome {
    ItemSyncOutcome {
        source_id: snapshot.source_id,
        source_path: snapshot.source_path,
        item_type: snapshot.item_type,
        status: SyncStatus::Skipped,
        target_path: None,
        size: snapshot.size,
        etag: snapshot.etag,
        modified_at: snapshot.modified_at,
        content_hash: snapshot.content_hash,
        bytes_written: 0,
        error_message: None,
    }
}

fn failed_item_outcome(snapshot: ItemSnapshot, error_message: String) -> ItemSyncOutcome {
    ItemSyncOutcome {
        source_id: snapshot.source_id,
        source_path: snapshot.source_path,
        item_type: snapshot.item_type,
        status: SyncStatus::Failed,
        target_path: None,
        size: snapshot.size,
        etag: snapshot.etag,
        modified_at: snapshot.modified_at,
        content_hash: snapshot.content_hash,
        bytes_written: 0,
        error_message: Some(error_message),
    }
}
