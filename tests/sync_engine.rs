use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use camino::Utf8PathBuf;
use futures::{FutureExt, stream};
use hoarder::{
    connectors::traits::{
        ByteStream, ConnectorConfig, ConnectorFuture, ScanStream, SourceConnector,
    },
    core::types::{
        ConnectorCapabilities, ConnectorKind, ItemRef, ItemSnapshot, ItemType, JobId, RunId,
        SourceId, SyncStatus,
    },
    sync::{
        engine::{SyncEngine, SyncJob, SyncRunStatus, SyncRunSummary},
        planner::StoredItemState,
        repository::{ItemSyncOutcome, SyncRepository},
        vault_writer::VaultWriter,
    },
};

#[tokio::test]
async fn sync_engine_records_item_failure_and_continues_run() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("failure-continues");
    let connector = Arc::new(FakeConnector::new(
        source_id,
        [
            file_snapshot(source_id, "ok.txt", 2),
            file_snapshot(source_id, "fail.txt", 4),
            file_snapshot(source_id, "later.txt", 5),
        ],
        BTreeMap::from([
            ("ok.txt".to_owned(), Ok(Bytes::from_static(b"ok"))),
            (
                "fail.txt".to_owned(),
                Err("connector read failed".to_owned()),
            ),
            ("later.txt".to_owned(), Ok(Bytes::from_static(b"later"))),
        ]),
    ));
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    let engine = SyncEngine::new(
        repository.clone(),
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root.clone()),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(
        summary,
        SyncRunSummary {
            run_id,
            processed: 3,
            synced: 2,
            skipped: 0,
            failed: 1,
            bytes_written: 7,
        }
    );
    assert_eq!(
        tokio::fs::read(vault_root.join(source_id.to_string()).join("ok.txt"))
            .await
            .unwrap(),
        b"ok"
    );
    assert_eq!(
        tokio::fs::read(vault_root.join(source_id.to_string()).join("later.txt"))
            .await
            .unwrap(),
        b"later"
    );
    assert!(
        tokio::fs::metadata(vault_root.join(source_id.to_string()).join("fail.txt"))
            .await
            .is_err()
    );

    let events = repository.events();
    assert!(events.contains(&RepoEvent::StartRun(job_id, run_id)));
    assert!(events.contains(&RepoEvent::RecordSynced("ok.txt".to_owned())));
    assert!(events.contains(&RepoEvent::RecordFailure("fail.txt".to_owned())));
    assert!(events.contains(&RepoEvent::RecordSynced("later.txt".to_owned())));
    assert_eq!(
        events.last(),
        Some(&RepoEvent::FinishRun(
            run_id,
            SyncRunStatus::CompletedWithFailures,
            summary
        ))
    );
}

#[tokio::test]
async fn sync_engine_skips_unchanged_items_and_counts_summary() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let snapshot = file_snapshot(source_id, "same.txt", 4);
    let vault_root = temp_vault_root("skip-unchanged");
    let connector = Arc::new(FakeConnector::new(
        source_id,
        [snapshot.clone()],
        BTreeMap::from([("same.txt".to_owned(), Ok(Bytes::from_static(b"same")))]),
    ));
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    repository.set_item_state(StoredItemState {
        source_path: "same.txt".to_owned(),
        item_type: ItemType::File,
        size: snapshot.size,
        etag: snapshot.etag.clone(),
        modified_at: snapshot.modified_at,
        content_hash: None,
    });
    let engine = SyncEngine::new(
        repository.clone(),
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root.clone()),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(
        summary,
        SyncRunSummary {
            run_id,
            processed: 1,
            synced: 0,
            skipped: 1,
            failed: 0,
            bytes_written: 0,
        }
    );
    assert!(
        tokio::fs::metadata(vault_root.join(source_id.to_string()).join("same.txt"))
            .await
            .is_err()
    );
    assert!(
        repository
            .events()
            .contains(&RepoEvent::RecordSkipped("same.txt".to_owned()))
    );
}

#[tokio::test]
async fn sync_engine_marks_unseen_previous_items_deleted_without_removing_local_files() {
    let source_id = SourceId::new();
    let job_id = JobId::new();
    let run_id = RunId::new();
    let vault_root = temp_vault_root("mark-deleted");
    let local_path = vault_root.join(source_id.to_string()).join("old.txt");
    tokio::fs::create_dir_all(local_path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&local_path, b"keep me").await.unwrap();
    let connector = Arc::new(FakeConnector::new(
        source_id,
        [file_snapshot(source_id, "current.txt", 7)],
        BTreeMap::from([("current.txt".to_owned(), Ok(Bytes::from_static(b"current")))]),
    ));
    let repository = Arc::new(FakeRepository::new(SyncJob {
        id: job_id,
        source_id,
        connector_kind: ConnectorKind::OpenDal,
        connector_config: connector_config(),
        scan_cursor: None,
    }));
    repository.set_next_run_id(run_id);
    repository.set_all_known_item_states([StoredItemState {
        source_path: "old.txt".to_owned(),
        item_type: ItemType::File,
        size: Some(6),
        etag: Some("old-etag".to_owned()),
        modified_at: None,
        content_hash: None,
    }]);
    let engine = SyncEngine::new(
        repository.clone(),
        Arc::new(move |_kind| Ok(connector.clone() as Arc<dyn SourceConnector>)),
        VaultWriter::new(vault_root.clone()),
    );

    let summary = engine.run_job(job_id).await.unwrap();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.synced, 1);
    assert_eq!(summary.failed, 0);
    assert_eq!(tokio::fs::read(&local_path).await.unwrap(), b"keep me");
    assert!(
        repository
            .events()
            .contains(&RepoEvent::MarkDeleted("old.txt".to_owned()))
    );
}

#[derive(Debug)]
struct FakeConnector {
    source_id: SourceId,
    snapshots: Vec<ItemSnapshot>,
    reads: BTreeMap<String, Result<Bytes, String>>,
}

impl FakeConnector {
    fn new<const N: usize>(
        source_id: SourceId,
        snapshots: [ItemSnapshot; N],
        reads: BTreeMap<String, Result<Bytes, String>>,
    ) -> Self {
        Self {
            source_id,
            snapshots: snapshots.into_iter().collect(),
            reads,
        }
    }
}

impl SourceConnector for FakeConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::OpenDal
    }

    fn validate<'a>(
        &'a self,
        _config: &'a ConnectorConfig,
    ) -> ConnectorFuture<'a, ConnectorCapabilities> {
        async move { Ok(ConnectorCapabilities::default()) }.boxed()
    }

    fn scan<'a>(
        &'a self,
        _config: &'a ConnectorConfig,
        _cursor: Option<&'a str>,
    ) -> ConnectorFuture<'a, ScanStream> {
        async move {
            Ok(Box::pin(stream::iter(self.snapshots.clone().into_iter().map(Ok))) as ScanStream)
        }
        .boxed()
    }

    fn read<'a>(
        &'a self,
        _config: &'a ConnectorConfig,
        item_ref: &'a ItemRef,
    ) -> ConnectorFuture<'a, ByteStream> {
        async move {
            assert_eq!(item_ref.source_id, self.source_id);
            let result = self.reads.get(&item_ref.source_path).unwrap().clone();
            match result {
                Ok(bytes) => Ok(Box::pin(stream::iter([Ok(bytes)])) as ByteStream),
                Err(message) => Ok(Box::pin(stream::iter([Err(hoarder::AppError::Connector(
                    message,
                ))])) as ByteStream),
            }
        }
        .boxed()
    }
}

struct FakeRepository {
    job: SyncJob,
    next_run_ids: Mutex<VecDeque<RunId>>,
    item_states: Mutex<BTreeMap<String, StoredItemState>>,
    all_known_item_states: Mutex<Vec<StoredItemState>>,
    events: Mutex<Vec<RepoEvent>>,
}

impl FakeRepository {
    const fn new(job: SyncJob) -> Self {
        Self {
            job,
            next_run_ids: Mutex::new(VecDeque::new()),
            item_states: Mutex::new(BTreeMap::new()),
            all_known_item_states: Mutex::new(Vec::new()),
            events: Mutex::new(Vec::new()),
        }
    }

    fn set_next_run_id(&self, run_id: RunId) {
        self.next_run_ids.lock().unwrap().push_back(run_id);
    }

    fn set_item_state(&self, state: StoredItemState) {
        self.item_states
            .lock()
            .unwrap()
            .insert(state.source_path.clone(), state);
    }

    fn set_all_known_item_states<const N: usize>(&self, states: [StoredItemState; N]) {
        *self.all_known_item_states.lock().unwrap() = states.into_iter().collect();
    }

    fn events(&self) -> Vec<RepoEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl SyncRepository for FakeRepository {
    fn load_job(&self, job_id: JobId) -> ConnectorFuture<'_, SyncJob> {
        async move {
            assert_eq!(job_id, self.job.id);
            Ok(self.job.clone())
        }
        .boxed()
    }

    fn start_run<'a>(&'a self, job: &'a SyncJob) -> ConnectorFuture<'a, RunId> {
        async move {
            let run_id = self
                .next_run_ids
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default();
            self.events
                .lock()
                .unwrap()
                .push(RepoEvent::StartRun(job.id, run_id));
            Ok(run_id)
        }
        .boxed()
    }

    fn item_state<'a>(
        &'a self,
        _source_id: SourceId,
        source_path: &'a str,
    ) -> ConnectorFuture<'a, Option<StoredItemState>> {
        async move { Ok(self.item_states.lock().unwrap().get(source_path).cloned()) }.boxed()
    }

    fn known_item_states(&self, _source_id: SourceId) -> ConnectorFuture<'_, Vec<StoredItemState>> {
        async move { Ok(self.all_known_item_states.lock().unwrap().clone()) }.boxed()
    }

    fn record_item_outcome(
        &self,
        _run_id: RunId,
        outcome: ItemSyncOutcome,
    ) -> ConnectorFuture<'_, ()> {
        async move {
            let event = match outcome.status {
                SyncStatus::Synced => RepoEvent::RecordSynced(outcome.source_path),
                SyncStatus::Skipped => RepoEvent::RecordSkipped(outcome.source_path),
                SyncStatus::Failed => RepoEvent::RecordFailure(outcome.source_path),
                other => panic!("unexpected item outcome status: {other:?}"),
            };
            self.events.lock().unwrap().push(event);
            Ok(())
        }
        .boxed()
    }

    fn mark_deleted<'a>(
        &'a self,
        _run_id: RunId,
        source_id: SourceId,
        source_path: &'a str,
    ) -> ConnectorFuture<'a, ()> {
        async move {
            assert_eq!(source_id, self.job.source_id);
            self.events
                .lock()
                .unwrap()
                .push(RepoEvent::MarkDeleted(source_path.to_owned()));
            Ok(())
        }
        .boxed()
    }

    fn finish_run(
        &self,
        run_id: RunId,
        status: SyncRunStatus,
        summary: SyncRunSummary,
    ) -> ConnectorFuture<'_, ()> {
        async move {
            self.events
                .lock()
                .unwrap()
                .push(RepoEvent::FinishRun(run_id, status, summary));
            Ok(())
        }
        .boxed()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RepoEvent {
    StartRun(JobId, RunId),
    RecordSynced(String),
    RecordSkipped(String),
    RecordFailure(String),
    MarkDeleted(String),
    FinishRun(RunId, SyncRunStatus, SyncRunSummary),
}

fn file_snapshot(source_id: SourceId, source_path: &str, size: u64) -> ItemSnapshot {
    ItemSnapshot {
        source_id,
        source_path: source_path.to_owned(),
        item_type: ItemType::File,
        size: Some(size),
        etag: Some(format!("{source_path}-etag")),
        modified_at: None,
        content_hash: None,
        metadata_json: None,
    }
}

fn connector_config() -> ConnectorConfig {
    ConnectorConfig::OpenDal {
        service: "fs".to_owned(),
        options: BTreeMap::new(),
    }
}

fn temp_vault_root(name: &str) -> Utf8PathBuf {
    let root = std::env::temp_dir().join(format!("hoarder-sync-engine-{name}-{}", SourceId::new()));
    let root = Utf8PathBuf::from_path_buf(root).unwrap();
    std::fs::remove_dir_all(&root).ok();
    root
}
